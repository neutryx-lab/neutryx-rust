<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useToast } from '@/composables/useToast';
import { fetchPortfolioTrades } from '@/services/api';
import type { TradeSummary, TradeStatistics } from '@/types';

const toast = useToast();

// State
const trades = ref<TradeSummary[]>([]);
const statistics = ref<TradeStatistics | null>(null);
const isLoading = ref(false);
const selectedTrades = ref<Set<string>>(new Set());

// Computed
const totalNotional = computed(() => statistics.value?.total_notional ?? 0);
const tradeCount = computed(() => statistics.value?.total_count ?? trades.value.length);

const summaryStats = computed(() => [
  { label: 'Total Notional', value: formatCurrency(totalNotional.value), positive: true },
  { label: 'Trade Count', value: tradeCount.value.toString(), positive: true },
  {
    label: 'Instruments',
    value: Object.keys(statistics.value?.by_instrument_type ?? {}).length.toString(),
    positive: true,
  },
  {
    label: 'Currencies',
    value: Object.keys(statistics.value?.by_currency ?? {}).length.toString(),
    positive: true,
  },
]);

// Utility functions
function formatCurrency(value: number): string {
  const absValue = Math.abs(value);
  if (absValue >= 1_000_000) {
    return `${value >= 0 ? '' : '-'}$${(absValue / 1_000_000).toFixed(1)}M`;
  } else if (absValue >= 1_000) {
    return `${value >= 0 ? '' : '-'}$${(absValue / 1_000).toFixed(0)}K`;
  }
  return `$${value.toFixed(0)}`;
}

function formatExpiry(expiry: number): string {
  if (expiry >= 1) {
    return `${expiry.toFixed(1)}Y`;
  }
  return `${(expiry * 12).toFixed(0)}M`;
}

function toggleTradeSelection(id: string) {
  if (selectedTrades.value.has(id)) {
    selectedTrades.value.delete(id);
  } else {
    selectedTrades.value.add(id);
  }
}

function selectAllTrades() {
  if (selectedTrades.value.size === trades.value.length) {
    selectedTrades.value.clear();
  } else {
    selectedTrades.value = new Set(trades.value.map(t => t.id));
  }
}

async function loadData() {
  isLoading.value = true;
  try {
    const response = await fetchPortfolioTrades();
    trades.value = response.trades;
    statistics.value = response.statistics;
    toast.success(`Loaded ${response.trades.length} FpML trades`);
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Unknown error';
    toast.error(`Failed to load trades: ${message}`);
    trades.value = [];
    statistics.value = null;
  } finally {
    isLoading.value = false;
  }
}

onMounted(() => loadData());
</script>

<template>
  <div class="portfolio-view">
    <!-- Summary Stats -->
    <div class="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
      <div
        v-for="stat in summaryStats"
        :key="stat.label"
        class="glass-card p-4"
      >
        <p class="text-sm text-[var(--text-muted)] mb-1">{{ stat.label }}</p>
        <p
          :class="[
            'text-2xl font-semibold',
            stat.positive ? 'text-[var(--success)]' : 'text-[var(--danger)]'
          ]"
        >
          {{ stat.value }}
        </p>
      </div>
    </div>

    <!-- Trade Table -->
    <div class="glass-card p-6 mb-6">
      <div class="flex items-center justify-between mb-4">
        <h3 class="text-lg font-semibold text-[var(--text-primary)]">FpML Trades</h3>
        <div class="flex items-center gap-2">
          <button class="btn-secondary text-sm" @click="loadData" :disabled="isLoading">
            <i class="fas fa-sync-alt mr-2" :class="{ 'animate-spin': isLoading }"></i>Refresh
          </button>
        </div>
      </div>

      <div v-if="isLoading" class="flex items-center justify-center py-12">
        <i class="fas fa-spinner fa-spin text-2xl text-[var(--primary)]"></i>
      </div>

      <div v-else-if="trades.length === 0" class="text-center py-12 text-[var(--text-muted)]">
        <i class="fas fa-inbox text-4xl mb-4 opacity-50"></i>
        <p>No FpML trades loaded</p>
      </div>

      <div v-else class="overflow-x-auto">
        <table class="w-full">
          <thead>
            <tr class="text-left text-sm text-[var(--text-muted)] border-b border-[var(--glass-border)]">
              <th class="p-3">
                <input
                  type="checkbox"
                  :checked="selectedTrades.size === trades.length && trades.length > 0"
                  @change="selectAllTrades"
                />
              </th>
              <th class="p-3">Trade ID</th>
              <th class="p-3">Instrument Type</th>
              <th class="p-3">Currency</th>
              <th class="p-3 text-right">Notional</th>
              <th class="p-3 text-right">Expiry</th>
              <th class="p-3"></th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="trade in trades"
              :key="trade.id"
              class="border-b border-[var(--glass-border)] hover:bg-[var(--surface-hover)] transition-colors"
            >
              <td class="p-3">
                <input
                  type="checkbox"
                  :checked="selectedTrades.has(trade.id)"
                  @change="toggleTradeSelection(trade.id)"
                />
              </td>
              <td class="p-3 text-sm font-mono text-[var(--text-primary)]">{{ trade.id }}</td>
              <td class="p-3">
                <span class="px-2 py-1 rounded text-xs font-medium bg-[var(--primary)]/20 text-[var(--primary)]">
                  {{ trade.instrument_type }}
                </span>
              </td>
              <td class="p-3 text-sm text-[var(--text-secondary)]">{{ trade.currency }}</td>
              <td class="p-3 text-sm text-right text-[var(--text-primary)]">{{ formatCurrency(trade.notional) }}</td>
              <td class="p-3 text-sm text-right text-[var(--text-muted)]">{{ formatExpiry(trade.expiry) }}</td>
              <td class="p-3">
                <button class="text-[var(--text-muted)] hover:text-[var(--text-primary)]">
                  <i class="fas fa-ellipsis-v"></i>
                </button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <div class="flex items-center justify-between mt-4 text-sm text-[var(--text-muted)]">
        <span>Showing {{ trades.length }} trades</span>
        <span>{{ selectedTrades.size }} selected</span>
      </div>
    </div>

    <!-- Statistics Breakdown -->
    <div v-if="statistics" class="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <!-- By Instrument Type -->
      <div class="glass-card p-6">
        <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">By Instrument Type</h3>
        <div class="space-y-3">
          <div
            v-for="(count, type) in statistics.by_instrument_type"
            :key="type"
            class="flex items-center justify-between"
          >
            <span class="text-sm text-[var(--text-secondary)]">{{ type }}</span>
            <span class="px-2 py-1 rounded text-xs font-medium bg-[var(--surface)] text-[var(--text-primary)]">
              {{ count }}
            </span>
          </div>
        </div>
      </div>

      <!-- By Currency -->
      <div class="glass-card p-6">
        <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">By Currency</h3>
        <div class="space-y-3">
          <div
            v-for="(count, currency) in statistics.by_currency"
            :key="currency"
            class="flex items-center justify-between"
          >
            <span class="text-sm text-[var(--text-secondary)]">{{ currency }}</span>
            <span class="px-2 py-1 rounded text-xs font-medium bg-[var(--surface)] text-[var(--text-primary)]">
              {{ count }}
            </span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.glass-card {
  background: var(--glass-bg);
  backdrop-filter: blur(20px);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-lg);
  box-shadow: var(--glass-shadow);
}

.btn-secondary {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0.5rem 1rem;
  border-radius: 0.5rem;
  background: var(--surface);
  border: 1px solid var(--glass-border);
  color: var(--text-primary);
  transition: all 0.2s;
}

.btn-secondary:hover {
  background: var(--surface-hover);
}
</style>
