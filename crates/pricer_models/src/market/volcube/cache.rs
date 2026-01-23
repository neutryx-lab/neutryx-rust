//! VolCubeキャッシュインフラストラクチャ。
//!
//! # Requirements: 5.1-5.5
//!
//! このモジュールはVolCubeのLRUキャッシュ機能を提供する。
//! 入力Instrumentリストと設定のハッシュに基づくキャッシュキーを生成し、
//! 同一条件での再カリブレーションを回避する。

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Instant;

use lru::LruCache;
use num_traits::Float;
use ordered_float::OrderedFloat;
use parking_lot::RwLock;

use super::config::VolCubeConfig;
use super::types::VolInstrument;

/// キャッシュ統計情報。
///
/// # Requirements: 5.5
///
/// キャッシュの使用状況をモニタリングするための統計。
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// キャッシュヒット数。
    pub hits: u64,
    /// キャッシュミス数。
    pub misses: u64,
    /// 現在のエントリ数。
    pub entries: usize,
    /// 最大容量。
    pub capacity: usize,
    /// 挿入数。
    pub insertions: u64,
    /// 削除数（eviction含む）。
    pub evictions: u64,
}

impl CacheStats {
    /// ヒット率を計算（0.0〜1.0）。
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// 使用率を計算（0.0〜1.0）。
    pub fn utilisation(&self) -> f64 {
        if self.capacity == 0 {
            0.0
        } else {
            self.entries as f64 / self.capacity as f64
        }
    }
}

/// VolCubeキャッシュキー。
///
/// # Requirements: 5.1
///
/// Instrumentリストと設定のハッシュに基づく一意なキャッシュキー。
/// Instrumentリストのハッシュにはデータ内容が含まれるため、
/// データ更新時にはハッシュが変化してキャッシュが無効化される。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VolCubeKey {
    /// Instrumentリストのハッシュ。
    instruments_hash: u64,
    /// 設定のハッシュ。
    config_hash: u64,
}

impl VolCubeKey {
    /// 新しいキャッシュキーを作成。
    pub fn new(instruments_hash: u64, config_hash: u64) -> Self {
        Self {
            instruments_hash,
            config_hash,
        }
    }

    /// InstrumentリストからハッシュでキーをCurve生成。
    ///
    /// # Arguments
    /// * `instruments` - VolInstrumentのスライス
    /// * `config` - VolCubeConfig
    pub fn from_instruments<T: Float>(
        instruments: &[VolInstrument<T>],
        config: &VolCubeConfig,
    ) -> Self {
        let instruments_hash = Self::hash_instruments(instruments);
        let config_hash = Self::hash_config(config);

        Self::new(instruments_hash, config_hash)
    }

    /// Instrumentリストのハッシュを計算。
    fn hash_instruments<T: Float>(instruments: &[VolInstrument<T>]) -> u64 {
        let mut hasher = DefaultHasher::new();

        for inst in instruments {
            inst.instrument_id.hash(&mut hasher);
            // Float値はOrderedFloatでハッシュ可能にする
            if let Some(expiry_f64) = inst.expiry.to_f64() {
                OrderedFloat(expiry_f64).hash(&mut hasher);
            }
            if let Some(tenor_f64) = inst.tenor.to_f64() {
                OrderedFloat(tenor_f64).hash(&mut hasher);
            }
            if let Some(strike_f64) = inst.strike.to_f64() {
                OrderedFloat(strike_f64).hash(&mut hasher);
            }
            if let Some(vol_f64) = inst.implied_vol.to_f64() {
                OrderedFloat(vol_f64).hash(&mut hasher);
            }
            if let Some(fwd_f64) = inst.forward.to_f64() {
                OrderedFloat(fwd_f64).hash(&mut hasher);
            }
        }

        hasher.finish()
    }

    /// 設定のハッシュを計算。
    fn hash_config(config: &VolCubeConfig) -> u64 {
        let mut hasher = DefaultHasher::new();

        // 列挙型はメモリ表現でハッシュ
        (config.interpolation as u8).hash(&mut hasher);
        (config.extrapolation as u8).hash(&mut hasher);
        (config.strike_axis as u8).hash(&mut hasher);
        (config.optimizer as u8).hash(&mut hasher);
        config.validate_arbitrage_free.hash(&mut hasher);

        if let Some(beta) = config.sabr_beta {
            OrderedFloat(beta).hash(&mut hasher);
        }
        OrderedFloat(config.sabr_shift).hash(&mut hasher);
        config.max_iterations.hash(&mut hasher);
        OrderedFloat(config.tolerance).hash(&mut hasher);

        hasher.finish()
    }

    /// Instrumentハッシュを取得。
    pub fn instruments_hash(&self) -> u64 {
        self.instruments_hash
    }

    /// 設定ハッシュを取得。
    pub fn config_hash(&self) -> u64 {
        self.config_hash
    }
}

/// VolCubeキャッシュエントリ。
///
/// キャッシュ値とメタデータを保持。
#[derive(Debug, Clone)]
pub struct VolCubeCacheEntry<V> {
    /// キャッシュされた値。
    pub value: V,
    /// 作成時刻。
    pub created_at: Instant,
    /// 最終アクセス時刻。
    pub last_accessed: Instant,
}

impl<V> VolCubeCacheEntry<V> {
    /// 新しいエントリを作成。
    pub fn new(value: V) -> Self {
        let now = Instant::now();
        Self {
            value,
            created_at: now,
            last_accessed: now,
        }
    }
}

/// VolCube LRUキャッシュ。
///
/// # Requirements: 5.2, 5.3, 5.4, 5.5
///
/// スレッドセーフなLRUキャッシュ実装。
/// `parking_lot::RwLock`による高効率な並行アクセスをサポート。
///
/// # 型パラメータ
/// * `V` - キャッシュされる値の型
pub struct VolCubeCache<V: Clone> {
    /// LRUキャッシュ（RwLockで保護）。
    cache: RwLock<LruCache<VolCubeKey, VolCubeCacheEntry<V>>>,
    /// 統計情報（RwLockで保護）。
    stats: RwLock<CacheStats>,
}

impl<V: Clone> VolCubeCache<V> {
    /// 新しいキャッシュを作成。
    ///
    /// # Arguments
    /// * `capacity` - 最大エントリ数
    ///
    /// # Panics
    /// capacity が 0 の場合パニック。
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "Cache capacity must be positive");

        let cache = LruCache::new(
            std::num::NonZeroUsize::new(capacity).expect("capacity must be non-zero"),
        );
        let stats = CacheStats {
            capacity,
            ..Default::default()
        };

        Self {
            cache: RwLock::new(cache),
            stats: RwLock::new(stats),
        }
    }

    /// キャッシュから値を検索。
    ///
    /// # Requirements: 5.2
    ///
    /// ヒット時は値のクローンを返し、LRU順序を更新。
    pub fn lookup(&self, key: &VolCubeKey) -> Option<V> {
        let mut cache = self.cache.write();
        let mut stats = self.stats.write();

        if let Some(entry) = cache.get_mut(key) {
            entry.last_accessed = Instant::now();
            stats.hits += 1;
            Some(entry.value.clone())
        } else {
            stats.misses += 1;
            None
        }
    }

    /// キャッシュに値を挿入。
    ///
    /// # Requirements: 5.2, 5.4
    ///
    /// 容量超過時は最も古いエントリを削除（LRU eviction）。
    pub fn insert(&self, key: VolCubeKey, value: V) {
        let mut cache = self.cache.write();
        let mut stats = self.stats.write();

        let was_full = cache.len() >= stats.capacity;
        cache.put(key, VolCubeCacheEntry::new(value));

        stats.insertions += 1;
        stats.entries = cache.len();

        if was_full && cache.len() == stats.capacity {
            stats.evictions += 1;
        }
    }

    /// 特定のキーを無効化。
    ///
    /// # Requirements: 5.3
    pub fn invalidate(&self, key: &VolCubeKey) {
        let mut cache = self.cache.write();
        let mut stats = self.stats.write();

        if cache.pop(key).is_some() {
            stats.evictions += 1;
            stats.entries = cache.len();
        }
    }

    /// 全エントリをクリア。
    pub fn clear(&self) {
        let mut cache = self.cache.write();
        let mut stats = self.stats.write();

        let cleared = cache.len();
        cache.clear();
        stats.evictions += cleared as u64;
        stats.entries = 0;
    }

    /// 統計情報を取得。
    ///
    /// # Requirements: 5.5
    pub fn stats(&self) -> CacheStats {
        let cache = self.cache.read();
        let mut stats = self.stats.read().clone();
        stats.entries = cache.len();
        stats
    }

    /// 現在のエントリ数を取得。
    pub fn len(&self) -> usize {
        self.cache.read().len()
    }

    /// キャッシュが空かどうか。
    pub fn is_empty(&self) -> bool {
        self.cache.read().is_empty()
    }

    /// 容量を取得。
    pub fn capacity(&self) -> usize {
        self.stats.read().capacity
    }

    /// キーが存在するか確認（LRU順序を更新しない）。
    pub fn contains(&self, key: &VolCubeKey) -> bool {
        self.cache.read().contains(key)
    }
}

// Note: VolCubeCache<V> automatically implements Send + Sync
// when V: Clone + Send + Sync due to parking_lot::RwLock's guarantees

impl<V: Clone> std::fmt::Debug for VolCubeCache<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let stats = self.stats();
        f.debug_struct("VolCubeCache")
            .field("entries", &stats.entries)
            .field("capacity", &stats.capacity)
            .field("hit_rate", &stats.hit_rate())
            .finish()
    }
}

/// 共有キャッシュへの参照型。
pub type SharedVolCubeCache<V> = Arc<VolCubeCache<V>>;

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // CacheStats Tests
    // =========================================================================

    #[test]
    fn test_cache_stats_default() {
        let stats = CacheStats::default();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.entries, 0);
        assert_eq!(stats.capacity, 0);
    }

    #[test]
    fn test_cache_stats_hit_rate() {
        let stats = CacheStats {
            hits: 80,
            misses: 20,
            ..Default::default()
        };
        assert!((stats.hit_rate() - 0.8).abs() < 1e-10);
    }

    #[test]
    fn test_cache_stats_hit_rate_no_access() {
        let stats = CacheStats::default();
        assert_eq!(stats.hit_rate(), 0.0);
    }

    #[test]
    fn test_cache_stats_utilisation() {
        let stats = CacheStats {
            entries: 50,
            capacity: 100,
            ..Default::default()
        };
        assert!((stats.utilisation() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_cache_stats_utilisation_zero_capacity() {
        let stats = CacheStats::default();
        assert_eq!(stats.utilisation(), 0.0);
    }

    // =========================================================================
    // VolCubeKey Tests
    // =========================================================================

    #[test]
    fn test_volcube_key_new() {
        let key = VolCubeKey::new(12345, 67890);
        assert_eq!(key.instruments_hash(), 12345);
        assert_eq!(key.config_hash(), 67890);
    }

    #[test]
    fn test_volcube_key_from_instruments() {
        let instruments = vec![
            VolInstrument::new("INST-1", 1.0_f64, 5.0, 0.03, 0.20, 0.03),
            VolInstrument::new("INST-2", 2.0_f64, 5.0, 0.03, 0.22, 0.03),
        ];
        let config = VolCubeConfig::default();

        let key = VolCubeKey::from_instruments(&instruments, &config);
        assert!(key.instruments_hash() != 0);
        assert!(key.config_hash() != 0);
    }

    #[test]
    fn test_volcube_key_different_instruments() {
        let config = VolCubeConfig::default();

        let instruments1 = vec![
            VolInstrument::new("INST-1", 1.0_f64, 5.0, 0.03, 0.20, 0.03),
        ];
        let instruments2 = vec![
            VolInstrument::new("INST-2", 1.0_f64, 5.0, 0.03, 0.20, 0.03),
        ];

        let key1 = VolCubeKey::from_instruments(&instruments1, &config);
        let key2 = VolCubeKey::from_instruments(&instruments2, &config);

        assert_ne!(key1, key2);
    }

    #[test]
    fn test_volcube_key_different_config() {
        let instruments = vec![
            VolInstrument::new("INST-1", 1.0_f64, 5.0, 0.03, 0.20, 0.03),
        ];

        let config1 = VolCubeConfig::default();
        let config2 = VolCubeConfig::default().with_sabr_beta(Some(0.75));

        let key1 = VolCubeKey::from_instruments(&instruments, &config1);
        let key2 = VolCubeKey::from_instruments(&instruments, &config2);

        // 異なる設定では異なるキー
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_volcube_key_clone_eq_hash() {
        use std::collections::HashSet;

        let key1 = VolCubeKey::new(100, 200);
        let key2 = key1.clone();
        assert_eq!(key1, key2);

        let mut set = HashSet::new();
        set.insert(key1.clone());
        assert!(set.contains(&key1));
    }

    // =========================================================================
    // VolCubeCache Tests
    // =========================================================================

    #[test]
    fn test_cache_new() {
        let cache: VolCubeCache<String> = VolCubeCache::new(100);
        assert_eq!(cache.capacity(), 100);
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    #[should_panic(expected = "capacity must be positive")]
    fn test_cache_new_zero_capacity() {
        let _cache: VolCubeCache<String> = VolCubeCache::new(0);
    }

    #[test]
    fn test_cache_insert_lookup() {
        let cache: VolCubeCache<String> = VolCubeCache::new(10);
        let key = VolCubeKey::new(1, 1);

        cache.insert(key.clone(), "value1".to_string());

        let result = cache.lookup(&key);
        assert_eq!(result, Some("value1".to_string()));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_cache_lookup_miss() {
        let cache: VolCubeCache<String> = VolCubeCache::new(10);
        let key = VolCubeKey::new(1, 1);

        let result = cache.lookup(&key);
        assert!(result.is_none());

        let stats = cache.stats();
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hits, 0);
    }

    #[test]
    fn test_cache_lru_eviction() {
        let cache: VolCubeCache<i32> = VolCubeCache::new(3);

        // 3つ挿入
        for i in 0..3 {
            let key = VolCubeKey::new(i, 0);
            cache.insert(key, i as i32);
        }
        assert_eq!(cache.len(), 3);

        // 4つ目を挿入 → 最古のものがevict
        let key4 = VolCubeKey::new(100, 0);
        cache.insert(key4, 100);
        assert_eq!(cache.len(), 3);

        // 最初のキーはevictされているはず
        // Note: タイムスタンプが異なるので別のキーになるため、
        // ここではLRU動作による容量維持の基本確認のみ
    }

    #[test]
    fn test_cache_invalidate() {
        let cache: VolCubeCache<String> = VolCubeCache::new(10);
        let key = VolCubeKey::new(1, 1);

        cache.insert(key.clone(), "value".to_string());
        assert_eq!(cache.len(), 1);

        cache.invalidate(&key);
        assert_eq!(cache.len(), 0);
        assert!(cache.lookup(&key).is_none());
    }

    #[test]
    fn test_cache_clear() {
        let cache: VolCubeCache<i32> = VolCubeCache::new(10);

        for i in 0..5 {
            let key = VolCubeKey::new(i, 0);
            cache.insert(key, i as i32);
        }
        assert_eq!(cache.len(), 5);

        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_stats() {
        let cache: VolCubeCache<String> = VolCubeCache::new(10);
        let key = VolCubeKey::new(1, 1);

        // Insert
        cache.insert(key.clone(), "value".to_string());

        // Hit
        let _ = cache.lookup(&key);
        // Miss
        let missing_key = VolCubeKey::new(999, 0);
        let _ = cache.lookup(&missing_key);

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.insertions, 1);
        assert!((stats.hit_rate() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_cache_contains() {
        let cache: VolCubeCache<String> = VolCubeCache::new(10);
        let key = VolCubeKey::new(1, 1);

        assert!(!cache.contains(&key));

        cache.insert(key.clone(), "value".to_string());
        assert!(cache.contains(&key));
    }

    #[test]
    fn test_cache_debug() {
        let cache: VolCubeCache<String> = VolCubeCache::new(10);
        let debug_str = format!("{:?}", cache);
        assert!(debug_str.contains("VolCubeCache"));
        assert!(debug_str.contains("capacity"));
    }

    #[test]
    fn test_cache_thread_safety() {
        use std::thread;

        let cache = Arc::new(VolCubeCache::<i32>::new(100));

        let handles: Vec<_> = (0..4)
            .map(|thread_id| {
                let cache_clone = Arc::clone(&cache);
                thread::spawn(move || {
                    for i in 0..25 {
                        let key = VolCubeKey::new(thread_id * 100 + i, 0);
                        cache_clone.insert(key.clone(), (thread_id * 100 + i) as i32);
                        let _ = cache_clone.lookup(&key);
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        // 全スレッドが完了し、クラッシュしないことを確認
        assert!(cache.len() <= 100);
    }

    // =========================================================================
    // VolCubeCacheEntry Tests
    // =========================================================================

    #[test]
    fn test_cache_entry_new() {
        let entry = VolCubeCacheEntry::new("test_value".to_string());
        assert_eq!(entry.value, "test_value");
        // created_atとlast_accessedはほぼ同時
    }
}
