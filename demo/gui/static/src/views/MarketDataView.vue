<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue';

// Types
type AssetClass = 'Rates' | 'FX' | 'IRVol' | 'FXVol' | 'Events';

interface MarketRate {
  id: string;
  currency: string;
  tenor: string;
  rateType: string;
  value: number;
  rateIndex?: string;
  source: string;
  isStale: boolean;
}

interface IrVolQuote {
  id: string;
  currency: string;
  expiry: string;
  tenor: string;
  atmVol: number;
  volType: string;
  source: string;
}

interface FxVolQuote {
  id: string;
  pair: string;
  expiry: number;
  expiryLabel: string;
  atmVol: number;
  rr25d: number;
  bf25d: number;
  rr10d?: number;
  bf10d?: number;
}

interface MarketEvent {
  id: string;
  date: string;
  eventType: string;
  title: string;
  currency?: string;
  region?: string;
  importance: string;
  time?: string;
  source: string;
}

// State
const assetClass = ref<AssetClass>('Rates');
const rates = ref<MarketRate[]>([]);
const irVolQuotes = ref<IrVolQuote[]>([]);
const fxVolQuotes = ref<FxVolQuote[]>([]);
const events = ref<MarketEvent[]>([]);
const selectedRateId = ref<string | null>(null);
const currencyFilter = ref('');
const sortColumn = ref('tenor');
const sortDirection = ref<'asc' | 'desc'>('asc');
const isLoading = ref(false);
const lastUpdated = ref<Date | null>(null);

// Computed
const assetClasses: AssetClass[] = ['Rates', 'FX', 'IRVol', 'FXVol', 'Events'];

const filteredRates = computed(() => {
  let result = rates.value;
  if (currencyFilter.value) {
    result = result.filter(r => r.currency.toLowerCase() === currencyFilter.value.toLowerCase());
  }
  // Filter by asset class
  const typeMap: Record<string, string[]> = {
    Rates: ['deposit', 'swap', 'ois', 'fra', 'future', 'xccybasis'],
    FX: ['fxspot', 'fxforward'],
  };
  const types = typeMap[assetClass.value] || [];
  if (types.length > 0) {
    result = result.filter(r => types.includes(r.rateType?.toLowerCase() || ''));
  }
  // Sort
  result = [...result].sort((a, b) => {
    const dir = sortDirection.value === 'asc' ? 1 : -1;
    const aVal = a[sortColumn.value as keyof MarketRate];
    const bVal = b[sortColumn.value as keyof MarketRate];
    if (sortColumn.value === 'value') return (Number(aVal) - Number(bVal)) * dir;
    return String(aVal || '').localeCompare(String(bVal || '')) * dir;
  });
  return result;
});

const currencies = computed(() => {
  const set = new Set<string>();
  rates.value.forEach(r => set.add(r.currency));
  return Array.from(set).sort();
});

const summaryStats = computed(() => {
  if (assetClass.value === 'IRVol') {
    return [
      { label: 'Total Quotes', value: irVolQuotes.value.length, icon: 'fa-chart-area', color: '#3b82f6' },
      { label: 'Currencies', value: new Set(irVolQuotes.value.map(q => q.currency)).size, icon: 'fa-money-bill', color: '#10b981' },
      { label: 'Expiries', value: new Set(irVolQuotes.value.map(q => q.expiry)).size, icon: 'fa-clock', color: '#8b5cf6' },
      { label: 'Status', value: 'Live', icon: 'fa-check-circle', color: '#10b981' },
    ];
  }
  if (assetClass.value === 'FXVol') {
    return [
      { label: 'Total Quotes', value: fxVolQuotes.value.length, icon: 'fa-chart-area', color: '#3b82f6' },
      { label: 'Pairs', value: new Set(fxVolQuotes.value.map(q => q.pair)).size, icon: 'fa-exchange-alt', color: '#10b981' },
      { label: 'Expiries', value: new Set(fxVolQuotes.value.map(q => q.expiryLabel)).size, icon: 'fa-clock', color: '#8b5cf6' },
      { label: 'Status', value: 'Live', icon: 'fa-check-circle', color: '#10b981' },
    ];
  }
  if (assetClass.value === 'Events') {
    return [
      { label: 'Total Events', value: events.value.length, icon: 'fa-calendar', color: '#3b82f6' },
      { label: 'Upcoming', value: events.value.filter(e => new Date(e.date) >= new Date()).length, icon: 'fa-hourglass-half', color: '#f59e0b' },
      { label: 'Regions', value: new Set(events.value.map(e => e.region).filter(Boolean)).size, icon: 'fa-globe', color: '#8b5cf6' },
      { label: 'Status', value: 'Live', icon: 'fa-check-circle', color: '#10b981' },
    ];
  }
  return [
    { label: 'Total Rates', value: rates.value.length, icon: 'fa-database', color: '#3b82f6' },
    { label: 'Live', value: rates.value.filter(r => !r.isStale).length, icon: 'fa-check-circle', color: '#10b981' },
    { label: 'Displayed', value: filteredRates.value.length, icon: 'fa-eye', color: '#8b5cf6' },
    { label: 'Stale', value: filteredRates.value.filter(r => r.isStale).length, icon: 'fa-clock', color: '#f59e0b' },
  ];
});

const selectedRate = computed(() => rates.value.find(r => r.id === selectedRateId.value) || null);

// Utility functions
function formatRate(value: number, rateType: string): string {
  if (rateType === 'fxspot' || rateType === 'fxforward') return value.toFixed(4);
  return `${(value * 100).toFixed(4)}%`;
}

function formatVol(vol: number): string {
  return `${(vol * 100).toFixed(2)}%`;
}

function formatVolBps(vol?: number): string {
  if (vol === undefined) return '-';
  return `${(vol * 10000).toFixed(1)} bps`;
}

function expiryToLabel(expiry: number): string {
  if (expiry < 0.05) return '1W';
  if (expiry < 0.125) return '1M';
  if (expiry < 0.33) return '3M';
  if (expiry < 0.54) return '6M';
  if (expiry < 1.5) return '1Y';
  if (expiry < 2.5) return '2Y';
  return `${Math.round(expiry)}Y`;
}

// API calls
async function loadRates() {
  isLoading.value = true;
  try {
    const response = await fetch('/api/market/rates');
    if (!response.ok) throw new Error('Failed to load rates');
    const data = await response.json();
    rates.value = data.rates || [];
    lastUpdated.value = new Date();
  } catch (error) {
    console.error('Failed to load market rates:', error);
  } finally {
    isLoading.value = false;
  }
}

async function loadIrVolData() {
  isLoading.value = true;
  try {
    const currenciesRes = await fetch('/api/volatility/ir/currencies');
    const currenciesData = await currenciesRes.json();
    const allQuotes: IrVolQuote[] = [];

    for (const curr of currenciesData.currencies || []) {
      const quotesRes = await fetch(`/api/volatility/ir/quotes/${curr.currency}`);
      if (quotesRes.ok) {
        const quotesData = await quotesRes.json();
        for (const q of quotesData.quotes || []) {
          allQuotes.push({
            id: `${curr.currency}-${q.expiry}-${q.tenor}`,
            currency: curr.currency,
            expiry: q.expiry,
            tenor: q.tenor,
            atmVol: q.atmVol,
            volType: quotesData.volType || 'Normal',
            source: quotesData.source || 'Demo',
          });
        }
      }
    }
    irVolQuotes.value = allQuotes;
    lastUpdated.value = new Date();
  } catch (error) {
    console.error('Failed to load IR vol data:', error);
  } finally {
    isLoading.value = false;
  }
}

async function loadFxVolData() {
  isLoading.value = true;
  try {
    const pairsRes = await fetch('/api/volatility/fx/pairs');
    const pairsData = await pairsRes.json();
    const allQuotes: FxVolQuote[] = [];

    for (const pairInfo of pairsData.pairs || []) {
      const quotesRes = await fetch(`/api/volatility/fx/quotes/${pairInfo.pair}`);
      if (quotesRes.ok) {
        const quotesData = await quotesRes.json();
        for (const q of quotesData.quotes || []) {
          allQuotes.push({
            id: `${pairInfo.pair}-${q.expiry}`,
            pair: pairInfo.pair,
            expiry: q.expiry,
            expiryLabel: expiryToLabel(q.expiry),
            atmVol: q.atmVol,
            rr25d: q.rr25d,
            bf25d: q.bf25d,
            rr10d: q.rr10d,
            bf10d: q.bf10d,
          });
        }
      }
    }
    fxVolQuotes.value = allQuotes;
    lastUpdated.value = new Date();
  } catch (error) {
    console.error('Failed to load FX vol data:', error);
  } finally {
    isLoading.value = false;
  }
}

async function loadEventsData() {
  isLoading.value = true;
  try {
    const response = await fetch('/api/market/events');
    if (!response.ok) throw new Error('Failed to load events');
    const data = await response.json();
    events.value = data.events || [];
    lastUpdated.value = new Date();
  } catch (error) {
    console.error('Failed to load events:', error);
  } finally {
    isLoading.value = false;
  }
}

async function refresh() {
  if (assetClass.value === 'IRVol') await loadIrVolData();
  else if (assetClass.value === 'FXVol') await loadFxVolData();
  else if (assetClass.value === 'Events') await loadEventsData();
  else await loadRates();
}

function toggleSort(column: string) {
  if (sortColumn.value === column) {
    sortDirection.value = sortDirection.value === 'asc' ? 'desc' : 'asc';
  } else {
    sortColumn.value = column;
    sortDirection.value = 'asc';
  }
}

function selectRate(rateId: string) {
  selectedRateId.value = selectedRateId.value === rateId ? null : rateId;
}

async function exportData(format: 'csv' | 'json') {
  try {
    const response = await fetch(`/api/market/export/${format}`);
    if (!response.ok) throw new Error('Export failed');
    const blob = await response.blob();
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `market_data.${format}`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  } catch (error) {
    console.error('Export failed:', error);
  }
}

// Watch asset class changes
watch(assetClass, (newClass) => {
  selectedRateId.value = null;
  if (newClass === 'IRVol' && irVolQuotes.value.length === 0) loadIrVolData();
  else if (newClass === 'FXVol' && fxVolQuotes.value.length === 0) loadFxVolData();
  else if (newClass === 'Events' && events.value.length === 0) loadEventsData();
});

// Initialize
onMounted(() => {
  loadRates();
});
</script>

<template>
  <div class="market-data-view">
    <!-- Summary Stats -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
      <div v-for="stat in summaryStats" :key="stat.label" class="glass-card p-4">
        <div class="flex items-start justify-between">
          <div>
            <p class="text-sm text-[var(--text-muted)] mb-1">{{ stat.label }}</p>
            <p class="text-2xl font-semibold text-[var(--text-primary)]">{{ stat.value }}</p>
          </div>
          <div class="w-10 h-10 rounded-lg flex items-center justify-center flex-shrink-0" :style="{ backgroundColor: `${stat.color}1a` }">
            <i :class="['fas', stat.icon]" :style="{ color: stat.color }"></i>
          </div>
        </div>
      </div>
    </div>

    <!-- Asset Class Tabs & Controls -->
    <div class="flex flex-wrap items-center justify-between gap-4 mb-6">
      <div class="flex gap-2">
        <button
          v-for="ac in assetClasses"
          :key="ac"
          :class="[
            'px-4 py-2 rounded-lg font-medium transition-all duration-200',
            assetClass === ac
              ? 'bg-[var(--primary)] text-white'
              : 'bg-[var(--surface)] text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]'
          ]"
          @click="assetClass = ac"
        >
          {{ ac }}
        </button>
      </div>
      <div class="flex items-center gap-3">
        <select
          v-if="assetClass === 'Rates' || assetClass === 'FX'"
          v-model="currencyFilter"
          class="px-4 py-2 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-[var(--primary)]"
        >
          <option value="">All Currencies</option>
          <option v-for="ccy in currencies" :key="ccy" :value="ccy">{{ ccy }}</option>
        </select>
        <button
          class="px-4 py-2 rounded-lg bg-[var(--surface)] text-[var(--text-secondary)] hover:bg-[var(--surface-hover)] transition-colors flex items-center gap-2"
          @click="refresh"
        >
          <i :class="['fas fa-sync-alt', isLoading ? 'fa-spin' : '']"></i>
          Refresh
        </button>
        <div class="relative">
          <button
            class="px-4 py-2 rounded-lg bg-[var(--surface)] text-[var(--text-secondary)] hover:bg-[var(--surface-hover)] transition-colors flex items-center gap-2"
            @click="exportData('csv')"
          >
            <i class="fas fa-download"></i>
            Export
          </button>
        </div>
      </div>
    </div>

    <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <!-- Data Table -->
      <div class="lg:col-span-2">
        <div class="glass-card p-6">
          <div class="flex items-center justify-between mb-4">
            <h3 class="text-lg font-semibold text-[var(--text-primary)]">
              {{ assetClass === 'IRVol' ? 'IR Volatility' : assetClass === 'FXVol' ? 'FX Volatility' : assetClass === 'Events' ? 'Market Events' : 'Market Rates' }}
            </h3>
            <span v-if="lastUpdated" class="text-xs text-[var(--text-muted)]">
              Updated: {{ lastUpdated.toLocaleTimeString() }}
            </span>
          </div>

          <!-- Loading -->
          <div v-if="isLoading" class="text-center py-12">
            <i class="fas fa-spinner fa-spin text-3xl text-[var(--primary)] mb-4"></i>
            <p class="text-[var(--text-muted)]">Loading data...</p>
          </div>

          <!-- Rates Table -->
          <template v-else-if="assetClass === 'Rates' || assetClass === 'FX'">
            <div v-if="filteredRates.length === 0" class="text-center py-12">
              <i class="fas fa-search text-4xl text-[var(--text-muted)] mb-4"></i>
              <p class="text-[var(--text-muted)]">No rates match the current filters</p>
            </div>
            <div v-else class="overflow-x-auto">
              <table class="w-full text-sm">
                <thead>
                  <tr class="border-b border-[var(--glass-border)]">
                    <th class="text-left py-3 px-3 text-xs font-medium text-[var(--text-muted)] cursor-pointer hover:text-[var(--text-primary)]" @click="toggleSort('id')">ID</th>
                    <th class="text-left py-3 px-3 text-xs font-medium text-[var(--text-muted)] cursor-pointer hover:text-[var(--text-primary)]" @click="toggleSort('currency')">Currency</th>
                    <th class="text-left py-3 px-3 text-xs font-medium text-[var(--text-muted)] cursor-pointer hover:text-[var(--text-primary)]" @click="toggleSort('tenor')">Tenor</th>
                    <th class="text-left py-3 px-3 text-xs font-medium text-[var(--text-muted)]">Type</th>
                    <th class="text-right py-3 px-3 text-xs font-medium text-[var(--text-muted)] cursor-pointer hover:text-[var(--text-primary)]" @click="toggleSort('value')">Value</th>
                    <th class="text-left py-3 px-3 text-xs font-medium text-[var(--text-muted)]">Index</th>
                    <th class="text-center py-3 px-3 text-xs font-medium text-[var(--text-muted)]">Status</th>
                  </tr>
                </thead>
                <tbody>
                  <tr
                    v-for="rate in filteredRates"
                    :key="rate.id"
                    :class="['border-b border-[var(--glass-border)] cursor-pointer transition-colors', selectedRateId === rate.id ? 'bg-[var(--primary)]/10' : 'hover:bg-[var(--surface-hover)]']"
                    @click="selectRate(rate.id)"
                  >
                    <td class="py-3 px-3 text-[var(--text-primary)] font-mono text-xs">{{ rate.id }}</td>
                    <td class="py-3 px-3 text-[var(--text-secondary)]">{{ rate.currency }}</td>
                    <td class="py-3 px-3 text-[var(--text-secondary)]">{{ rate.tenor }}</td>
                    <td class="py-3 px-3"><span class="px-2 py-0.5 rounded bg-[var(--surface)] text-[var(--text-secondary)] text-xs">{{ rate.rateType }}</span></td>
                    <td class="py-3 px-3 text-right font-mono" :class="rate.value >= 0 ? 'text-[var(--text-primary)]' : 'text-[var(--danger)]'">{{ formatRate(rate.value, rate.rateType) }}</td>
                    <td class="py-3 px-3">
                      <span v-if="rate.rateIndex" class="px-2 py-0.5 rounded bg-blue-500/10 text-blue-400 text-xs">{{ rate.rateIndex }}</span>
                      <span v-else class="text-[var(--text-muted)]">-</span>
                    </td>
                    <td class="py-3 px-3 text-center">
                      <span :class="['px-2 py-0.5 rounded text-xs', rate.isStale ? 'bg-yellow-500/10 text-yellow-400' : 'bg-green-500/10 text-green-400']">
                        <i :class="['fas mr-1', rate.isStale ? 'fa-clock' : 'fa-check']"></i>
                        {{ rate.isStale ? 'Stale' : 'Live' }}
                      </span>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </template>

          <!-- IR Vol Table -->
          <template v-else-if="assetClass === 'IRVol'">
            <div v-if="irVolQuotes.length === 0" class="text-center py-12">
              <i class="fas fa-chart-area text-4xl text-[var(--text-muted)] mb-4"></i>
              <p class="text-[var(--text-muted)]">No IR volatility data available</p>
            </div>
            <div v-else class="overflow-x-auto">
              <table class="w-full text-sm">
                <thead>
                  <tr class="border-b border-[var(--glass-border)]">
                    <th class="text-left py-3 px-3 text-xs font-medium text-[var(--text-muted)]">Currency</th>
                    <th class="text-left py-3 px-3 text-xs font-medium text-[var(--text-muted)]">Expiry</th>
                    <th class="text-left py-3 px-3 text-xs font-medium text-[var(--text-muted)]">Tenor</th>
                    <th class="text-right py-3 px-3 text-xs font-medium text-[var(--text-muted)]">ATM Vol</th>
                    <th class="text-left py-3 px-3 text-xs font-medium text-[var(--text-muted)]">Type</th>
                    <th class="text-left py-3 px-3 text-xs font-medium text-[var(--text-muted)]">Source</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="quote in irVolQuotes" :key="quote.id" class="border-b border-[var(--glass-border)] hover:bg-[var(--surface-hover)] transition-colors">
                    <td class="py-3 px-3 text-[var(--text-primary)]">{{ quote.currency }}</td>
                    <td class="py-3 px-3 text-[var(--text-secondary)]">{{ quote.expiry }}</td>
                    <td class="py-3 px-3 text-[var(--text-secondary)]">{{ quote.tenor }}</td>
                    <td class="py-3 px-3 text-right font-mono text-[var(--text-primary)]">{{ formatVol(quote.atmVol) }}</td>
                    <td class="py-3 px-3 text-[var(--text-secondary)]">{{ quote.volType }}</td>
                    <td class="py-3 px-3 text-[var(--text-muted)]">{{ quote.source }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </template>

          <!-- FX Vol Table -->
          <template v-else-if="assetClass === 'FXVol'">
            <div v-if="fxVolQuotes.length === 0" class="text-center py-12">
              <i class="fas fa-exchange-alt text-4xl text-[var(--text-muted)] mb-4"></i>
              <p class="text-[var(--text-muted)]">No FX volatility data available</p>
            </div>
            <div v-else class="overflow-x-auto">
              <table class="w-full text-sm">
                <thead>
                  <tr class="border-b border-[var(--glass-border)]">
                    <th class="text-left py-3 px-3 text-xs font-medium text-[var(--text-muted)]">Pair</th>
                    <th class="text-left py-3 px-3 text-xs font-medium text-[var(--text-muted)]">Expiry</th>
                    <th class="text-right py-3 px-3 text-xs font-medium text-[var(--text-muted)]">ATM Vol</th>
                    <th class="text-right py-3 px-3 text-xs font-medium text-[var(--text-muted)]">25D RR</th>
                    <th class="text-right py-3 px-3 text-xs font-medium text-[var(--text-muted)]">25D BF</th>
                    <th class="text-right py-3 px-3 text-xs font-medium text-[var(--text-muted)]">10D RR</th>
                    <th class="text-right py-3 px-3 text-xs font-medium text-[var(--text-muted)]">10D BF</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="quote in fxVolQuotes" :key="quote.id" class="border-b border-[var(--glass-border)] hover:bg-[var(--surface-hover)] transition-colors">
                    <td class="py-3 px-3 text-[var(--text-primary)] font-medium">{{ quote.pair }}</td>
                    <td class="py-3 px-3 text-[var(--text-secondary)]">{{ quote.expiryLabel }}</td>
                    <td class="py-3 px-3 text-right font-mono text-[var(--text-primary)]">{{ formatVol(quote.atmVol) }}</td>
                    <td class="py-3 px-3 text-right font-mono" :class="quote.rr25d >= 0 ? 'text-[var(--text-secondary)]' : 'text-[var(--danger)]'">{{ formatVolBps(quote.rr25d) }}</td>
                    <td class="py-3 px-3 text-right font-mono text-[var(--text-secondary)]">{{ formatVolBps(quote.bf25d) }}</td>
                    <td class="py-3 px-3 text-right font-mono" :class="(quote.rr10d ?? 0) >= 0 ? 'text-[var(--text-secondary)]' : 'text-[var(--danger)]'">{{ formatVolBps(quote.rr10d) }}</td>
                    <td class="py-3 px-3 text-right font-mono text-[var(--text-secondary)]">{{ formatVolBps(quote.bf10d) }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </template>

          <!-- Events Table -->
          <template v-else-if="assetClass === 'Events'">
            <div v-if="events.length === 0" class="text-center py-12">
              <i class="fas fa-calendar-alt text-4xl text-[var(--text-muted)] mb-4"></i>
              <p class="text-[var(--text-muted)]">No events available</p>
            </div>
            <div v-else class="overflow-x-auto">
              <table class="w-full text-sm">
                <thead>
                  <tr class="border-b border-[var(--glass-border)]">
                    <th class="text-left py-3 px-3 text-xs font-medium text-[var(--text-muted)]">Date</th>
                    <th class="text-left py-3 px-3 text-xs font-medium text-[var(--text-muted)]">Type</th>
                    <th class="text-left py-3 px-3 text-xs font-medium text-[var(--text-muted)]">Title</th>
                    <th class="text-left py-3 px-3 text-xs font-medium text-[var(--text-muted)]">Currency</th>
                    <th class="text-left py-3 px-3 text-xs font-medium text-[var(--text-muted)]">Region</th>
                    <th class="text-center py-3 px-3 text-xs font-medium text-[var(--text-muted)]">Importance</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="event in events" :key="event.id" class="border-b border-[var(--glass-border)] hover:bg-[var(--surface-hover)] transition-colors">
                    <td class="py-3 px-3 text-[var(--text-primary)] font-mono">{{ event.date }}</td>
                    <td class="py-3 px-3"><span class="px-2 py-0.5 rounded bg-[var(--primary)]/10 text-[var(--primary)] text-xs">{{ event.eventType }}</span></td>
                    <td class="py-3 px-3 text-[var(--text-primary)]">{{ event.title }}</td>
                    <td class="py-3 px-3 text-[var(--text-secondary)]">{{ event.currency || '-' }}</td>
                    <td class="py-3 px-3 text-[var(--text-secondary)]">{{ event.region || '-' }}</td>
                    <td class="py-3 px-3 text-center">
                      <span :class="['px-2 py-0.5 rounded text-xs', event.importance === 'High' ? 'bg-red-500/10 text-red-400' : event.importance === 'Medium' ? 'bg-yellow-500/10 text-yellow-400' : 'bg-green-500/10 text-green-400']">
                        {{ event.importance }}
                      </span>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </template>
        </div>
      </div>

      <!-- Detail Panel -->
      <div>
        <div class="glass-card p-6">
          <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Rate Details</h3>

          <div v-if="!selectedRate" class="text-center py-8">
            <i class="fas fa-hand-pointer text-3xl text-[var(--text-muted)] mb-4"></i>
            <p class="text-[var(--text-muted)]">Select a rate to view details</p>
          </div>

          <template v-else>
            <div class="space-y-3">
              <div class="flex justify-between text-sm">
                <span class="text-[var(--text-muted)]">ID</span>
                <span class="text-[var(--text-primary)] font-mono">{{ selectedRate.id }}</span>
              </div>
              <div class="flex justify-between text-sm">
                <span class="text-[var(--text-muted)]">Currency</span>
                <span class="text-[var(--text-primary)]">{{ selectedRate.currency }}</span>
              </div>
              <div class="flex justify-between text-sm">
                <span class="text-[var(--text-muted)]">Tenor</span>
                <span class="text-[var(--text-primary)]">{{ selectedRate.tenor }}</span>
              </div>
              <div class="flex justify-between text-sm">
                <span class="text-[var(--text-muted)]">Type</span>
                <span class="text-[var(--text-primary)]">{{ selectedRate.rateType }}</span>
              </div>
              <div class="flex justify-between text-sm">
                <span class="text-[var(--text-muted)]">Value</span>
                <span :class="['font-mono', selectedRate.value >= 0 ? 'text-[var(--success)]' : 'text-[var(--danger)]']">
                  {{ formatRate(selectedRate.value, selectedRate.rateType) }}
                </span>
              </div>
              <div v-if="selectedRate.rateIndex" class="flex justify-between text-sm">
                <span class="text-[var(--text-muted)]">Index</span>
                <span class="text-[var(--text-primary)]">{{ selectedRate.rateIndex }}</span>
              </div>
              <div class="flex justify-between text-sm">
                <span class="text-[var(--text-muted)]">Source</span>
                <span class="text-[var(--text-primary)]">{{ selectedRate.source }}</span>
              </div>
              <div class="flex justify-between text-sm">
                <span class="text-[var(--text-muted)]">Status</span>
                <span :class="selectedRate.isStale ? 'text-yellow-400' : 'text-green-400'">
                  {{ selectedRate.isStale ? 'Stale' : 'Live' }}
                </span>
              </div>
            </div>
          </template>
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
</style>
