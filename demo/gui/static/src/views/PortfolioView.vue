<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useToast } from '@/composables/useToast';

const toast = useToast();

// Types
interface Trade {
  id: string;
  type: string;
  notional: number;
  currency: string;
  pv: number;
  counterparty: string;
  maturity: string;
}

interface Counterparty {
  name: string;
  rating: string;
  exposure: number;
  limit: number;
  cva: number;
  utilisation: number;
}

// State
const trades = ref<Trade[]>([]);
const counterparties = ref<Counterparty[]>([]);
const isLoading = ref(false);
const selectedTrades = ref<Set<string>>(new Set());

// Computed
const totalPv = computed(() => trades.value.reduce((sum, t) => sum + t.pv, 0));
const tradeCount = computed(() => trades.value.length);

const summaryStats = computed(() => [
  { label: 'Total PV', value: formatCurrency(totalPv.value), positive: totalPv.value >= 0 },
  { label: 'Trade Count', value: tradeCount.value.toString(), positive: true },
  { label: 'Avg Delta', value: '0.42', positive: true },
  { label: 'Total Vega', value: formatCurrency(totalPv.value * 0.1), positive: true },
]);

// Mock data generation
function generateMockData() {
  const tradeTypes = ['IRS', 'FRA', 'Swaption', 'Cap', 'Floor'];
  const currencies = ['USD', 'EUR', 'GBP', 'JPY'];
  const counterpartyNames = ['Bank A', 'Bank B', 'Corp X', 'Corp Y', 'Fund Z'];

  trades.value = Array.from({ length: 20 }, (_, i) => ({
    id: `TRD-${1000 + i}`,
    type: tradeTypes[Math.floor(Math.random() * tradeTypes.length)],
    notional: Math.floor(Math.random() * 100) * 1000000,
    currency: currencies[Math.floor(Math.random() * currencies.length)],
    pv: (Math.random() - 0.3) * 5000000,
    counterparty: counterpartyNames[Math.floor(Math.random() * counterpartyNames.length)],
    maturity: `${2025 + Math.floor(Math.random() * 10)}-${String(Math.floor(Math.random() * 12) + 1).padStart(2, '0')}-01`,
  }));

  counterparties.value = counterpartyNames.map(name => ({
    name,
    rating: ['AAA', 'AA+', 'AA', 'A+', 'A', 'BBB+'][Math.floor(Math.random() * 6)],
    exposure: Math.random() * 50000000,
    limit: 100000000,
    cva: Math.random() * 2000000,
    utilisation: Math.random() * 100,
  }));
}

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

function getRiskLevel(pv: number, notional: number): { level: string; class: string } {
  const ratio = Math.abs(pv) / notional;
  if (ratio > 0.05) return { level: 'High', class: 'bg-[var(--danger)]/20 text-[var(--danger)]' };
  if (ratio > 0.02) return { level: 'Med', class: 'bg-[var(--warning)]/20 text-[var(--warning)]' };
  return { level: 'Low', class: 'bg-[var(--success)]/20 text-[var(--success)]' };
}

function getRatingClass(rating: string): string {
  if (rating.startsWith('AAA') || rating.startsWith('AA')) return 'bg-[var(--success)]/20 text-[var(--success)]';
  if (rating.startsWith('A')) return 'bg-[var(--primary)]/20 text-[var(--primary)]';
  return 'bg-[var(--warning)]/20 text-[var(--warning)]';
}

function getUtilisationClass(utilisation: number): string {
  if (utilisation >= 80) return 'bg-[var(--danger)]';
  if (utilisation >= 50) return 'bg-[var(--warning)]';
  return 'bg-[var(--success)]';
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
  // Simulate API call
  await new Promise(resolve => setTimeout(resolve, 500));
  generateMockData();
  isLoading.value = false;
  toast.success('Portfolio data loaded');
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
        <h3 class="text-lg font-semibold text-[var(--text-primary)]">Trades</h3>
        <div class="flex items-center gap-2">
          <button class="btn-secondary text-sm" @click="loadData">
            <i class="fas fa-sync-alt mr-2"></i>Refresh
          </button>
        </div>
      </div>

      <div class="overflow-x-auto">
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
              <th class="p-3">ID</th>
              <th class="p-3">Instrument</th>
              <th class="p-3">Counterparty</th>
              <th class="p-3">Maturity</th>
              <th class="p-3 text-right">Notional</th>
              <th class="p-3 text-right">PV</th>
              <th class="p-3">Risk</th>
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
              <td class="p-3 text-sm text-[var(--text-primary)]">{{ trade.id }}</td>
              <td class="p-3 text-sm text-[var(--text-primary)]">{{ trade.type }} {{ trade.currency }}</td>
              <td class="p-3 text-sm text-[var(--text-secondary)]">{{ trade.counterparty }}</td>
              <td class="p-3 text-sm text-[var(--text-muted)]">{{ trade.maturity }}</td>
              <td class="p-3 text-sm text-right text-[var(--text-primary)]">{{ formatCurrency(trade.notional) }}</td>
              <td
                :class="[
                  'p-3 text-sm text-right font-medium',
                  trade.pv >= 0 ? 'text-[var(--success)]' : 'text-[var(--danger)]'
                ]"
              >
                {{ formatCurrency(trade.pv) }}
              </td>
              <td class="p-3">
                <span
                  :class="[
                    'px-2 py-1 rounded text-xs font-medium',
                    getRiskLevel(trade.pv, trade.notional).class
                  ]"
                >
                  {{ getRiskLevel(trade.pv, trade.notional).level }}
                </span>
              </td>
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

    <!-- Counterparty Table -->
    <div class="glass-card p-6">
      <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Counterparty Exposure</h3>

      <div class="overflow-x-auto">
        <table class="w-full">
          <thead>
            <tr class="text-left text-sm text-[var(--text-muted)] border-b border-[var(--glass-border)]">
              <th class="p-3">Counterparty</th>
              <th class="p-3">Rating</th>
              <th class="p-3 text-right">Exposure</th>
              <th class="p-3 text-right">Limit</th>
              <th class="p-3">Utilisation</th>
              <th class="p-3 text-right">CVA</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="cp in counterparties"
              :key="cp.name"
              class="border-b border-[var(--glass-border)] hover:bg-[var(--surface-hover)] transition-colors"
            >
              <td class="p-3 text-sm font-medium text-[var(--text-primary)]">{{ cp.name }}</td>
              <td class="p-3">
                <span :class="['px-2 py-1 rounded text-xs font-medium', getRatingClass(cp.rating)]">
                  {{ cp.rating }}
                </span>
              </td>
              <td class="p-3 text-sm text-right text-[var(--text-primary)]">{{ formatCurrency(cp.exposure) }}</td>
              <td class="p-3 text-sm text-right text-[var(--text-muted)]">{{ formatCurrency(cp.limit) }}</td>
              <td class="p-3">
                <div class="flex items-center gap-2">
                  <div class="flex-1 h-2 bg-[var(--surface)] rounded-full overflow-hidden">
                    <div
                      :class="['h-full rounded-full', getUtilisationClass(cp.utilisation)]"
                      :style="{ width: `${cp.utilisation}%` }"
                    ></div>
                  </div>
                  <span class="text-xs text-[var(--text-muted)] w-10 text-right">{{ cp.utilisation.toFixed(0) }}%</span>
                </div>
              </td>
              <td class="p-3 text-sm text-right text-[var(--danger)]">-{{ formatCurrency(cp.cva) }}</td>
            </tr>
          </tbody>
        </table>
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
