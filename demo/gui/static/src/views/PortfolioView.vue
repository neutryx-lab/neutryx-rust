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
const showFilters = ref(false);

// Filter state
const filters = ref({
  id: '',
  instrument_type: '',
  book: '',
  counterparty: '',
  currency: '',
  maturityStart: '',
  maturityEnd: '',
});

// Sorting state
type SortKey = 'id' | 'instrument_type' | 'counterparty' | 'book' | 'currency' | 'notional' | 'maturity';
const sortKey = ref<SortKey>('id');
const sortOrder = ref<'asc' | 'desc'>('asc');

// Computed - filter options (unique values from data)
const filterOptions = computed(() => ({
  instrumentTypes: [...new Set(trades.value.map(t => t.instrument_type))].sort(),
  books: [...new Set(trades.value.map(t => t.book))].sort(),
  counterparties: [...new Set(trades.value.map(t => t.counterparty))].sort(),
  currencies: [...new Set(trades.value.map(t => t.currency))].sort(),
}));

// Computed - filtered trades
const filteredTrades = computed(() => {
  return trades.value.filter(trade => {
    // Text filters (case-insensitive contains)
    if (filters.value.id && !trade.id.toLowerCase().includes(filters.value.id.toLowerCase())) {
      return false;
    }
    // Dropdown filters (exact match)
    if (filters.value.instrument_type && trade.instrument_type !== filters.value.instrument_type) {
      return false;
    }
    if (filters.value.book && trade.book !== filters.value.book) {
      return false;
    }
    if (filters.value.counterparty && trade.counterparty !== filters.value.counterparty) {
      return false;
    }
    if (filters.value.currency && trade.currency !== filters.value.currency) {
      return false;
    }
    // Maturity range filters
    if (filters.value.maturityStart && trade.maturity && trade.maturity < filters.value.maturityStart) {
      return false;
    }
    if (filters.value.maturityEnd && trade.maturity && trade.maturity > filters.value.maturityEnd) {
      return false;
    }
    return true;
  });
});

const activeFilterCount = computed(() => {
  return Object.values(filters.value).filter(v => v !== '').length;
});

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

const sortedTrades = computed(() => {
  const sorted = [...filteredTrades.value];
  sorted.sort((a, b) => {
    let aVal: string | number = a[sortKey.value];
    let bVal: string | number = b[sortKey.value];

    // Handle numeric sorting for notional
    if (sortKey.value === 'notional') {
      aVal = Number(aVal);
      bVal = Number(bVal);
    } else {
      aVal = String(aVal).toLowerCase();
      bVal = String(bVal).toLowerCase();
    }

    if (aVal < bVal) return sortOrder.value === 'asc' ? -1 : 1;
    if (aVal > bVal) return sortOrder.value === 'asc' ? 1 : -1;
    return 0;
  });
  return sorted;
});

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

function formatMaturity(maturity: string): string {
  if (!maturity || maturity === 'N/A') return 'N/A';
  return maturity;
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

function handleSort(key: SortKey) {
  if (sortKey.value === key) {
    sortOrder.value = sortOrder.value === 'asc' ? 'desc' : 'asc';
  } else {
    sortKey.value = key;
    sortOrder.value = 'asc';
  }
}

function getSortIcon(key: SortKey): string {
  if (sortKey.value !== key) return 'fa-sort';
  return sortOrder.value === 'asc' ? 'fa-sort-up' : 'fa-sort-down';
}

function clearFilters() {
  filters.value = {
    id: '',
    instrument_type: '',
    book: '',
    counterparty: '',
    currency: '',
    maturityStart: '',
    maturityEnd: '',
  };
}

function toggleFilters() {
  showFilters.value = !showFilters.value;
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
            'text-xl font-semibold',
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
        <h3 class="text-lg font-semibold text-[var(--text-primary)]">Transaction View</h3>
        <div class="flex items-center gap-2">
          <button
            class="btn-secondary text-sm"
            :class="{ 'bg-[var(--primary)]/20 border-[var(--primary)]': showFilters || activeFilterCount > 0 }"
            @click="toggleFilters"
          >
            <i class="fas fa-filter mr-2"></i>
            Filter
            <span v-if="activeFilterCount > 0" class="ml-1 px-1.5 py-0.5 rounded-full bg-[var(--primary)] text-white text-xs">
              {{ activeFilterCount }}
            </span>
          </button>
          <button class="btn-secondary text-sm" @click="loadData" :disabled="isLoading">
            <i class="fas fa-sync-alt mr-2" :class="{ 'animate-spin': isLoading }"></i>Refresh
          </button>
        </div>
      </div>

      <!-- Filter Panel -->
      <Transition name="slide">
        <div v-if="showFilters" class="filter-panel mb-4 p-4 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] overflow-hidden">
          <div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-5 gap-3">
            <!-- Trade ID -->
            <div class="min-w-0">
              <label class="block text-xs text-[var(--text-muted)] mb-1">Trade ID</label>
              <input
                v-model="filters.id"
                type="text"
                placeholder="Search..."
                class="filter-input"
              />
            </div>

            <!-- Type -->
            <div class="min-w-0">
              <label class="block text-xs text-[var(--text-muted)] mb-1">Type</label>
              <select v-model="filters.instrument_type" class="filter-input">
                <option value="">All</option>
                <option v-for="opt in filterOptions.instrumentTypes" :key="opt" :value="opt">{{ opt }}</option>
              </select>
            </div>

            <!-- Book -->
            <div class="min-w-0">
              <label class="block text-xs text-[var(--text-muted)] mb-1">Book</label>
              <select v-model="filters.book" class="filter-input">
                <option value="">All</option>
                <option v-for="opt in filterOptions.books" :key="opt" :value="opt">{{ opt }}</option>
              </select>
            </div>

            <!-- Counterparty -->
            <div class="min-w-0">
              <label class="block text-xs text-[var(--text-muted)] mb-1">Counterparty</label>
              <select v-model="filters.counterparty" class="filter-input">
                <option value="">All</option>
                <option v-for="opt in filterOptions.counterparties" :key="opt" :value="opt">{{ opt }}</option>
              </select>
            </div>

            <!-- Currency -->
            <div class="min-w-0">
              <label class="block text-xs text-[var(--text-muted)] mb-1">Currency</label>
              <select v-model="filters.currency" class="filter-input">
                <option value="">All</option>
                <option v-for="opt in filterOptions.currencies" :key="opt" :value="opt">{{ opt }}</option>
              </select>
            </div>

            <!-- Maturity Range - spans 2 columns on larger screens -->
            <div class="min-w-0 col-span-2 md:col-span-2 lg:col-span-2">
              <label class="block text-xs text-[var(--text-muted)] mb-1">Maturity Range</label>
              <div class="flex gap-2">
                <input
                  v-model="filters.maturityStart"
                  type="date"
                  class="filter-input flex-1 min-w-0"
                />
                <input
                  v-model="filters.maturityEnd"
                  type="date"
                  class="filter-input flex-1 min-w-0"
                />
              </div>
            </div>
          </div>

          <div class="flex justify-end mt-3">
            <button
              v-if="activeFilterCount > 0"
              class="text-sm text-[var(--primary)] hover:underline"
              @click="clearFilters"
            >
              <i class="fas fa-times mr-1"></i>Clear all filters
            </button>
          </div>
        </div>
      </Transition>

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
              <th class="p-3 sortable-header" @click="handleSort('id')">
                Trade ID <i class="fas" :class="getSortIcon('id')"></i>
              </th>
              <th class="p-3 sortable-header" @click="handleSort('instrument_type')">
                Type <i class="fas" :class="getSortIcon('instrument_type')"></i>
              </th>
              <th class="p-3 sortable-header" @click="handleSort('book')">
                Book <i class="fas" :class="getSortIcon('book')"></i>
              </th>
              <th class="p-3 sortable-header" @click="handleSort('counterparty')">
                Counterparty <i class="fas" :class="getSortIcon('counterparty')"></i>
              </th>
              <th class="p-3 sortable-header" @click="handleSort('currency')">
                Currency <i class="fas" :class="getSortIcon('currency')"></i>
              </th>
              <th class="p-3 text-right sortable-header" @click="handleSort('notional')">
                Notional <i class="fas" :class="getSortIcon('notional')"></i>
              </th>
              <th class="p-3 text-right sortable-header" @click="handleSort('maturity')">
                Maturity <i class="fas" :class="getSortIcon('maturity')"></i>
              </th>
              <th class="p-3"></th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="trade in sortedTrades"
              :key="trade.id"
              :class="[
                'border-b border-[var(--glass-border)] hover:bg-[var(--surface-hover)] transition-colors',
                selectedTrades.has(trade.id) ? 'bg-[var(--primary)]/10' : ''
              ]"
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
              <td class="p-3 text-sm text-[var(--text-secondary)]">{{ trade.book }}</td>
              <td class="p-3 text-sm text-[var(--text-secondary)]">{{ trade.counterparty }}</td>
              <td class="p-3 text-sm text-[var(--text-secondary)]">{{ trade.currency }}</td>
              <td class="p-3 text-sm text-right text-[var(--text-primary)]">{{ formatCurrency(trade.notional) }}</td>
              <td class="p-3 text-sm text-right text-[var(--text-muted)]">{{ formatMaturity(trade.maturity) }}</td>
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
        <span>
          Showing {{ sortedTrades.length }} of {{ trades.length }} trades
          <span v-if="activeFilterCount > 0" class="text-[var(--primary)]">(filtered)</span>
        </span>
        <span>{{ selectedTrades.size }} selected</span>
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

.sortable-header {
  cursor: pointer;
  user-select: none;
  transition: color 0.2s;
}

.sortable-header:hover {
  color: var(--text-primary);
}

.sortable-header i {
  margin-left: 0.25rem;
  font-size: 0.75rem;
  opacity: 0.5;
}

.sortable-header:hover i,
.sortable-header i.fa-sort-up,
.sortable-header i.fa-sort-down {
  opacity: 1;
}

/* Filter inputs */
.filter-input {
  width: 100%;
  padding: 0.5rem 0.75rem;
  font-size: 0.875rem;
  border-radius: 0.375rem;
  background: var(--glass-bg);
  border: 1px solid var(--glass-border);
  color: var(--text-primary);
  transition: border-color 0.2s, box-shadow 0.2s;
}

.filter-input:focus {
  outline: none;
  border-color: var(--primary);
  box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.2);
}

.filter-input::placeholder {
  color: var(--text-muted);
}

/* Date input adjustments */
.filter-input[type="date"] {
  padding: 0.375rem 0.5rem;
  min-width: 0;
}

.filter-input[type="date"]::-webkit-calendar-picker-indicator {
  opacity: 0.6;
  cursor: pointer;
}

.filter-input[type="date"]::-webkit-calendar-picker-indicator:hover {
  opacity: 1;
}

/* Slide transition */
.slide-enter-active,
.slide-leave-active {
  transition: all 0.2s ease;
  overflow: hidden;
}

.slide-enter-from,
.slide-leave-to {
  opacity: 0;
  max-height: 0;
  margin-bottom: 0;
  padding-top: 0;
  padding-bottom: 0;
}

.slide-enter-to,
.slide-leave-from {
  opacity: 1;
  max-height: 200px;
}
</style>
