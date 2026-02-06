<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue';

// Types
type AssetClass = 'Rates' | 'FX' | 'IRVol' | 'FXVol' | 'Events' | 'Holidays';

interface Holiday {
  id: string;
  date: string;
  name: string;
  country: string;
  currency?: string;
  type: string; // 'bank', 'market', 'settlement'
}

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

// Individual IR Vol instrument
interface IrVolInstrument {
  id: string;
  currency: string;
  expiry: string;
  tenor: string;
  strike: string; // 'ATM', '-50bp', '+50bp', etc.
  vol: number;
  volType: string;
  source: string;
}

// IR Vol quote with all smile instruments
interface IrVolQuote {
  id: string;
  currency: string;
  expiry: string;
  tenor: string;
  atmVol: number;
  volType: string;
  source: string;
  // Individual instruments for this grid point
  instruments: IrVolInstrument[];
}

// Individual FX Vol instrument (market-quoted instruments)
interface FxVolInstrument {
  id: string;
  pair: string;
  expiry: number;
  expiryLabel: string;
  instrumentType: string; // 'ATM', '25D RR', '25D BF', '10D RR', '10D BF'
  value: number; // vol for ATM, spread for RR/BF
  unit: string; // '%' for vol, 'bps' for RR/BF
}

// FX Vol quote with all smile instruments
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
  // Individual instruments for this grid point
  instruments: FxVolInstrument[];
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
  previous?: string;
  forecast?: string;
  actual?: string;
  /** Expected rate spike in basis points (for turn events) */
  expectedSpikeBp?: number;
}

// State
const assetClass = ref<AssetClass>('Rates');
const rates = ref<MarketRate[]>([]);
const irVolQuotes = ref<IrVolQuote[]>([]);
const fxVolQuotes = ref<FxVolQuote[]>([]);
const events = ref<MarketEvent[]>([]);
const holidays = ref<Holiday[]>([]);
const selectedRateId = ref<string | null>(null);
const selectedIrVolId = ref<string | null>(null);
const selectedFxVolId = ref<string | null>(null);
const selectedEventId = ref<string | null>(null);
const selectedHolidayId = ref<string | null>(null);
const currencyFilter = ref('');
const sortColumn = ref('tenor');
const sortDirection = ref<'asc' | 'desc'>('asc');
const isLoading = ref(false);
const lastUpdated = ref<Date | null>(null);

// Computed
const assetClasses: AssetClass[] = ['Rates', 'FX', 'IRVol', 'FXVol', 'Events', 'Holidays'];

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
  // Sort by currency then tenor (shortest first)
  result = [...result].sort((a, b) => {
    const currDiff = a.currency.localeCompare(b.currency);
    if (currDiff !== 0) return currDiff;
    return tenorToOrder(a.tenor) - tenorToOrder(b.tenor);
  });
  return result;
});

const currencies = computed(() => {
  const set = new Set<string>();
  rates.value.forEach(r => set.add(r.currency));
  return Array.from(set).sort();
});

// Currency options for IRVol tab
const irVolCurrencies = computed(() => {
  const set = new Set<string>();
  irVolQuotes.value.forEach(q => set.add(q.currency));
  return Array.from(set).sort();
});

// Pair options for FXVol tab
const fxVolPairs = computed(() => {
  const set = new Set<string>();
  fxVolQuotes.value.forEach(q => set.add(q.pair));
  return Array.from(set).sort();
});

// Currency options for Events tab
const eventCurrencies = computed(() => {
  const set = new Set<string>();
  events.value.forEach(e => {
    if (e.currency) set.add(e.currency);
  });
  return Array.from(set).sort();
});

// Currency options for Holidays tab
const holidayCurrencies = computed(() => {
  const set = new Set<string>();
  holidays.value.forEach(h => {
    if (h.currency) set.add(h.currency);
  });
  return Array.from(set).sort();
});

// Filtered data for each tab
const filteredIrVolQuotes = computed(() => {
  let result = irVolQuotes.value;
  if (currencyFilter.value) {
    result = result.filter(q => q.currency.toLowerCase() === currencyFilter.value.toLowerCase());
  }
  // Sort by currency then expiry then tenor (shortest first)
  return [...result].sort((a, b) => {
    const currDiff = a.currency.localeCompare(b.currency);
    if (currDiff !== 0) return currDiff;
    const expiryDiff = tenorToOrder(a.expiry) - tenorToOrder(b.expiry);
    if (expiryDiff !== 0) return expiryDiff;
    return tenorToOrder(a.tenor) - tenorToOrder(b.tenor);
  });
});

const filteredFxVolQuotes = computed(() => {
  let result = fxVolQuotes.value;
  if (currencyFilter.value) {
    result = result.filter(q => q.pair.toLowerCase() === currencyFilter.value.toLowerCase());
  }
  // Sort by pair then expiry (shortest first)
  return [...result].sort((a, b) => {
    const pairDiff = a.pair.localeCompare(b.pair);
    if (pairDiff !== 0) return pairDiff;
    return a.expiry - b.expiry;
  });
});

const filteredEvents = computed(() => {
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  let result = events.value.filter(e => new Date(e.date) >= today);
  if (currencyFilter.value) {
    result = result.filter(e => e.currency?.toLowerCase() === currencyFilter.value.toLowerCase());
  }
  // Sort by currency then date (earliest first)
  return [...result].sort((a, b) => {
    const currDiff = (a.currency || '').localeCompare(b.currency || '');
    if (currDiff !== 0) return currDiff;
    return new Date(a.date).getTime() - new Date(b.date).getTime();
  });
});

const filteredHolidays = computed(() => {
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  let result = holidays.value.filter(h => new Date(h.date) >= today);
  if (currencyFilter.value) {
    result = result.filter(h => h.currency?.toLowerCase() === currencyFilter.value.toLowerCase());
  }
  // Sort by currency then date (earliest first)
  return [...result].sort((a, b) => {
    const currDiff = (a.currency || '').localeCompare(b.currency || '');
    if (currDiff !== 0) return currDiff;
    return new Date(a.date).getTime() - new Date(b.date).getTime();
  });
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
      { label: 'Upcoming', value: filteredEvents.value.length, icon: 'fa-calendar', color: '#3b82f6' },
      { label: 'Currencies', value: new Set(filteredEvents.value.map(e => e.currency).filter(Boolean)).size, icon: 'fa-money-bill', color: '#10b981' },
      { label: 'Turn Events', value: filteredEvents.value.filter(e => isTurnEvent(e.eventType)).length, icon: 'fa-chart-line', color: '#f59e0b' },
      { label: 'Status', value: 'Live', icon: 'fa-check-circle', color: '#10b981' },
    ];
  }
  if (assetClass.value === 'Holidays') {
    return [
      { label: 'Upcoming', value: filteredHolidays.value.length, icon: 'fa-calendar-day', color: '#3b82f6' },
      { label: 'Currencies', value: new Set(filteredHolidays.value.map(h => h.currency).filter(Boolean)).size, icon: 'fa-money-bill', color: '#10b981' },
      { label: 'Countries', value: new Set(filteredHolidays.value.map(h => h.country)).size, icon: 'fa-flag', color: '#8b5cf6' },
      { label: 'Status', value: 'Live', icon: 'fa-check-circle', color: '#10b981' },
    ];
  }
  return [
    { label: 'Total Rates', value: rates.value.length, icon: 'fa-database', color: '#3b82f6' },
    { label: 'Live', value: rates.value.filter(r => !r.isStale).length, icon: 'fa-signal', color: '#10b981' },
    { label: 'Displayed', value: filteredRates.value.length, icon: 'fa-eye', color: '#8b5cf6' },
    { label: 'Stale', value: filteredRates.value.filter(r => r.isStale).length, icon: 'fa-clock', color: '#f59e0b' },
  ];
});

const selectedRate = computed(() => rates.value.find(r => r.id === selectedRateId.value) || null);
const selectedIrVol = computed(() => irVolQuotes.value.find(q => q.id === selectedIrVolId.value) || null);
const selectedFxVol = computed(() => fxVolQuotes.value.find(q => q.id === selectedFxVolId.value) || null);
const selectedEvent = computed(() => events.value.find(e => e.id === selectedEventId.value) || null);
const selectedHoliday = computed(() => holidays.value.find(h => h.id === selectedHolidayId.value) || null);

// Tenor ordering
const TENOR_ORDER = [
  'ON', 'TN', 'SN',
  '1D', '2D', '3D',
  '1W', '2W', '3W',
  '1M', '1x4', '2M', '2x5', '3M', '3x6', '4M', '4x7', '5M', '5x8', '6M', '6x9',
  '7M', '7x10', '8M', '8x11', '9M', '9x12',
  '1Y', '12x15', '12x18', '15M', '18M',
  '2Y', '3Y', '4Y', '5Y', '6Y', '7Y', '8Y', '9Y', '10Y',
  '12Y', '15Y', '20Y', '25Y', '30Y', '40Y', '50Y',
] as const;

// Utility functions
function tenorToOrder(tenor: string): number {
  if (!tenor) return TENOR_ORDER.length;
  const index = TENOR_ORDER.indexOf(tenor.toUpperCase() as typeof TENOR_ORDER[number]);
  return index >= 0 ? index : TENOR_ORDER.length;
}

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

// Format event type to short label
function formatEventType(eventType: string): string {
  const typeMap: Record<string, string> = {
    'turn_of_year': 'TOY',
    'turn_of_quarter': 'TOQ',
    'turn_of_month': 'TOM',
    'turn': 'Turn',
    'central_bank_meeting': 'CB',
    'economic_release': 'Econ',
    'holiday': 'Hol',
    'news': 'News',
    'expiry': 'Exp',
    'other': 'Other',
  };
  return typeMap[eventType] || eventType;
}

// Check if event type is a turn event
function isTurnEvent(eventType: string): boolean {
  return ['turn_of_year', 'turn_of_quarter', 'turn_of_month', 'turn'].includes(eventType);
}

// Format expected spike in bp
function formatSpikeBp(spike?: number): string {
  if (spike === undefined || spike === null) return '-';
  return `${spike.toFixed(1)} bp`;
}

// Convert strike string to ID-safe format (e.g., '-50bp' -> 'm50bp', '+50bp' -> 'p50bp')
function strikeToIdSuffix(strike: string): string {
  if (strike === 'ATM') return 'ATM';
  return strike.replace(/^\+/, 'p').replace(/^-/, 'm');
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
    const currenciesRes = await fetch('/api/irvol/currencies');
    const currenciesData = await currenciesRes.json();
    const allQuotes: IrVolQuote[] = [];

    for (const curr of currenciesData.currencies || []) {
      const quotesRes = await fetch(`/api/irvol/quotes/${curr.currency}`);
      if (quotesRes.ok) {
        const quotesData = await quotesRes.json();
        for (const q of quotesData.quotes || []) {
          const baseId = `${curr.currency}-${q.expiry}-${q.tenor}`;
          const volType = quotesData.volType || 'Normal';
          const source = quotesData.source || 'Demo';

          // Create individual instruments for this grid point
          const instruments: IrVolInstrument[] = [];

          // ATM instrument (always present)
          instruments.push({
            id: `${baseId}-ATM`,
            currency: curr.currency,
            expiry: q.expiry,
            tenor: q.tenor,
            strike: 'ATM',
            vol: q.atmVol,
            volType,
            source,
          });

          // Add smile instruments if available (strike offsets)
          if (q.smile && Array.isArray(q.smile)) {
            for (const smilePoint of q.smile) {
              instruments.push({
                id: `${baseId}-${strikeToIdSuffix(smilePoint.strike)}`,
                currency: curr.currency,
                expiry: q.expiry,
                tenor: q.tenor,
                strike: smilePoint.strike,
                vol: smilePoint.vol,
                volType,
                source,
              });
            }
          } else {
            // Generate typical smile points if not provided (demo data)
            const smileOffsets = ['-100bp', '-50bp', '+50bp', '+100bp'];
            for (const offset of smileOffsets) {
              // Generate smile vol with typical skew pattern
              const offsetBp = parseInt(offset.replace('bp', '').replace('+', ''));
              const skewAdjust = offsetBp * 0.00002; // Small skew adjustment
              const smileVol = q.atmVol + Math.abs(offsetBp) * 0.00001 + (offsetBp < 0 ? skewAdjust : -skewAdjust);
              instruments.push({
                id: `${baseId}-${strikeToIdSuffix(offset)}`,
                currency: curr.currency,
                expiry: q.expiry,
                tenor: q.tenor,
                strike: offset,
                vol: smileVol,
                volType,
                source,
              });
            }
          }

          allQuotes.push({
            id: baseId,
            currency: curr.currency,
            expiry: q.expiry,
            tenor: q.tenor,
            atmVol: q.atmVol,
            volType,
            source,
            instruments,
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
    const pairsRes = await fetch('/api/fxvol/pairs');
    const pairsData = await pairsRes.json();
    const allQuotes: FxVolQuote[] = [];

    for (const pairInfo of pairsData.pairs || []) {
      const quotesRes = await fetch(`/api/fxvol/quotes/${pairInfo.pair}`);
      if (quotesRes.ok) {
        const quotesData = await quotesRes.json();
        for (const q of quotesData.quotes || []) {
          // Use API-provided expiry_label (strict conversion from infra_domain)
          const expLabel = q.expiryLabel;
          const baseId = `${pairInfo.pair}-${expLabel}`;

          // Create individual market-quoted instruments
          const instruments: FxVolInstrument[] = [];

          // ATM instrument
          instruments.push({
            id: `${baseId}-ATM`,
            pair: pairInfo.pair,
            expiry: q.expiry,
            expiryLabel: expLabel,
            instrumentType: 'ATM',
            value: q.atmVol,
            unit: '%',
          });

          // 25D Risk Reversal
          instruments.push({
            id: `${baseId}-RR25`,
            pair: pairInfo.pair,
            expiry: q.expiry,
            expiryLabel: expLabel,
            instrumentType: 'RR25',
            value: q.rr25d,
            unit: 'bps',
          });

          // 25D Butterfly
          instruments.push({
            id: `${baseId}-BF25`,
            pair: pairInfo.pair,
            expiry: q.expiry,
            expiryLabel: expLabel,
            instrumentType: 'BF25',
            value: q.bf25d,
            unit: 'bps',
          });

          // 10D instruments (if available)
          if (q.rr10d !== undefined && q.bf10d !== undefined) {
            // 10D Risk Reversal
            instruments.push({
              id: `${baseId}-RR10`,
              pair: pairInfo.pair,
              expiry: q.expiry,
              expiryLabel: expLabel,
              instrumentType: 'RR10',
              value: q.rr10d,
              unit: 'bps',
            });

            // 10D Butterfly
            instruments.push({
              id: `${baseId}-BF10`,
              pair: pairInfo.pair,
              expiry: q.expiry,
              expiryLabel: expLabel,
              instrumentType: 'BF10',
              value: q.bf10d,
              unit: 'bps',
            });
          }

          allQuotes.push({
            id: baseId,
            pair: pairInfo.pair,
            expiry: q.expiry,
            expiryLabel: expLabel,
            atmVol: q.atmVol,
            rr25d: q.rr25d,
            bf25d: q.bf25d,
            rr10d: q.rr10d,
            bf10d: q.bf10d,
            instruments,
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
    // Filter out holidays from events
    events.value = (data.events || []).filter((e: MarketEvent) => e.eventType !== 'Holiday');
    lastUpdated.value = new Date();
  } catch (error) {
    console.error('Failed to load events:', error);
  } finally {
    isLoading.value = false;
  }
}

async function loadHolidaysData() {
  isLoading.value = true;
  try {
    const response = await fetch('/api/market/holidays');
    if (!response.ok) {
      // Fallback: try to get holidays from events endpoint
      const eventsRes = await fetch('/api/market/events');
      if (eventsRes.ok) {
        const data = await eventsRes.json();
        // Filter holidays from events
        const holidayEvents = (data.events || []).filter((e: MarketEvent) => e.eventType === 'Holiday');
        holidays.value = holidayEvents.map((e: MarketEvent) => ({
          id: e.id,
          date: e.date,
          name: e.title,
          country: e.region || 'Unknown',
          currency: e.currency,
          type: 'bank',
        }));
      }
    } else {
      const data = await response.json();
      holidays.value = data.holidays || [];
    }
    lastUpdated.value = new Date();
  } catch (error) {
    console.error('Failed to load holidays:', error);
  } finally {
    isLoading.value = false;
  }
}

async function refresh() {
  if (assetClass.value === 'IRVol') await loadIrVolData();
  else if (assetClass.value === 'FXVol') await loadFxVolData();
  else if (assetClass.value === 'Events') await loadEventsData();
  else if (assetClass.value === 'Holidays') await loadHolidaysData();
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

function selectIrVol(quoteId: string) {
  selectedIrVolId.value = selectedIrVolId.value === quoteId ? null : quoteId;
}

function selectFxVol(quoteId: string) {
  selectedFxVolId.value = selectedFxVolId.value === quoteId ? null : quoteId;
}

function selectEvent(eventId: string) {
  selectedEventId.value = selectedEventId.value === eventId ? null : eventId;
}

function selectHoliday(holidayId: string) {
  selectedHolidayId.value = selectedHolidayId.value === holidayId ? null : holidayId;
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
  selectedIrVolId.value = null;
  selectedFxVolId.value = null;
  selectedEventId.value = null;
  selectedHolidayId.value = null;
  currencyFilter.value = '';
  if (newClass === 'IRVol' && irVolQuotes.value.length === 0) loadIrVolData();
  else if (newClass === 'FXVol' && fxVolQuotes.value.length === 0) loadFxVolData();
  else if (newClass === 'Events' && events.value.length === 0) loadEventsData();
  else if (newClass === 'Holidays' && holidays.value.length === 0) loadHolidaysData();
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
        <!-- Currency filter for Rates/FX -->
        <select
          v-if="assetClass === 'Rates' || assetClass === 'FX'"
          v-model="currencyFilter"
          class="px-4 py-2 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-[var(--primary)]"
        >
          <option value="">All Currencies</option>
          <option v-for="ccy in currencies" :key="ccy" :value="ccy">{{ ccy }}</option>
        </select>
        <!-- Currency filter for IRVol -->
        <select
          v-if="assetClass === 'IRVol'"
          v-model="currencyFilter"
          class="px-4 py-2 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-[var(--primary)]"
        >
          <option value="">All Currencies</option>
          <option v-for="ccy in irVolCurrencies" :key="ccy" :value="ccy">{{ ccy }}</option>
        </select>
        <!-- Pair filter for FXVol -->
        <select
          v-if="assetClass === 'FXVol'"
          v-model="currencyFilter"
          class="px-4 py-2 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-[var(--primary)]"
        >
          <option value="">All Pairs</option>
          <option v-for="pair in fxVolPairs" :key="pair" :value="pair">{{ pair }}</option>
        </select>
        <!-- Currency filter for Events -->
        <select
          v-if="assetClass === 'Events'"
          v-model="currencyFilter"
          class="px-4 py-2 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-[var(--primary)]"
        >
          <option value="">All Currencies</option>
          <option v-for="ccy in eventCurrencies" :key="ccy" :value="ccy">{{ ccy }}</option>
        </select>
        <!-- Currency filter for Holidays -->
        <select
          v-if="assetClass === 'Holidays'"
          v-model="currencyFilter"
          class="px-4 py-2 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-[var(--primary)]"
        >
          <option value="">All Currencies</option>
          <option v-for="ccy in holidayCurrencies" :key="ccy" :value="ccy">{{ ccy }}</option>
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
              {{ assetClass === 'IRVol' ? 'IR Volatility' : assetClass === 'FXVol' ? 'FX Volatility' : assetClass === 'Events' ? 'Market Events' : assetClass === 'Holidays' ? 'Market Holidays' : 'Market Rates' }}
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
            <div v-if="filteredIrVolQuotes.length === 0" class="text-center py-12">
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
                    <th class="text-left py-3 px-3 text-xs font-medium text-[var(--text-muted)]">Strike</th>
                    <th class="text-right py-3 px-3 text-xs font-medium text-[var(--text-muted)]">Vol</th>
                    <th class="text-left py-3 px-3 text-xs font-medium text-[var(--text-muted)]">Type</th>
                    <th class="text-left py-3 px-3 text-xs font-medium text-[var(--text-muted)]">Source</th>
                  </tr>
                </thead>
                <tbody>
                  <tr
                    v-for="quote in filteredIrVolQuotes"
                    :key="quote.id"
                    :class="['border-b border-[var(--glass-border)] cursor-pointer transition-colors', selectedIrVolId === quote.id ? 'bg-[var(--primary)]/10' : 'hover:bg-[var(--surface-hover)]']"
                    @click="selectIrVol(quote.id)"
                  >
                    <td class="py-3 px-3 text-[var(--text-primary)]">{{ quote.currency }}</td>
                    <td class="py-3 px-3 text-[var(--text-secondary)]">{{ quote.expiry }}</td>
                    <td class="py-3 px-3 text-[var(--text-secondary)]">{{ quote.tenor }}</td>
                    <td class="py-3 px-3"><span class="px-2 py-0.5 rounded bg-blue-500/10 text-blue-400 text-xs">ATM</span></td>
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
            <div v-if="filteredFxVolQuotes.length === 0" class="text-center py-12">
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
                  <tr
                    v-for="quote in filteredFxVolQuotes"
                    :key="quote.id"
                    :class="['border-b border-[var(--glass-border)] cursor-pointer transition-colors', selectedFxVolId === quote.id ? 'bg-[var(--primary)]/10' : 'hover:bg-[var(--surface-hover)]']"
                    @click="selectFxVol(quote.id)"
                  >
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
            <div v-if="filteredEvents.length === 0" class="text-center py-12">
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
                    <th class="text-right py-3 px-3 text-xs font-medium text-[var(--text-muted)]">Jump</th>
                    <th class="text-center py-3 px-3 text-xs font-medium text-[var(--text-muted)]">Importance</th>
                  </tr>
                </thead>
                <tbody>
                  <tr
                    v-for="event in filteredEvents"
                    :key="event.id"
                    :class="['border-b border-[var(--glass-border)] cursor-pointer transition-colors', selectedEventId === event.id ? 'bg-[var(--primary)]/10' : 'hover:bg-[var(--surface-hover)]']"
                    @click="selectEvent(event.id)"
                  >
                    <td class="py-3 px-3 text-[var(--text-primary)] font-mono">{{ event.date }}</td>
                    <td class="py-3 px-3">
                      <span :class="[
                        'px-2 py-0.5 rounded text-xs',
                        isTurnEvent(event.eventType) ? 'bg-orange-500/10 text-orange-400' : 'bg-[var(--primary)]/10 text-[var(--primary)]'
                      ]">{{ formatEventType(event.eventType) }}</span>
                    </td>
                    <td class="py-3 px-3 text-[var(--text-primary)]">{{ event.title }}</td>
                    <td class="py-3 px-3 text-[var(--text-secondary)]">{{ event.currency || '-' }}</td>
                    <td class="py-3 px-3 text-right font-mono" :class="event.expectedSpikeBp !== undefined ? (isTurnEvent(event.eventType) ? 'text-orange-400' : 'text-blue-400') : 'text-[var(--text-muted)]'">
                      {{ formatSpikeBp(event.expectedSpikeBp) }}
                    </td>
                    <td class="py-3 px-3 text-center">
                      <span :class="['px-2 py-0.5 rounded text-xs', event.importance === 'High' || event.importance === 'critical' ? 'bg-red-500/10 text-red-400' : event.importance === 'Medium' || event.importance === 'medium' ? 'bg-yellow-500/10 text-yellow-400' : 'bg-green-500/10 text-green-400']">
                        {{ event.importance }}
                      </span>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </template>

          <!-- Holidays Table -->
          <template v-else-if="assetClass === 'Holidays'">
            <div v-if="filteredHolidays.length === 0" class="text-center py-12">
              <i class="fas fa-calendar-day text-4xl text-[var(--text-muted)] mb-4"></i>
              <p class="text-[var(--text-muted)]">No holidays available</p>
            </div>
            <div v-else class="overflow-x-auto">
              <table class="w-full text-sm">
                <thead>
                  <tr class="border-b border-[var(--glass-border)]">
                    <th class="text-left py-3 px-3 text-xs font-medium text-[var(--text-muted)]">Date</th>
                    <th class="text-left py-3 px-3 text-xs font-medium text-[var(--text-muted)]">Name</th>
                    <th class="text-left py-3 px-3 text-xs font-medium text-[var(--text-muted)]">Country</th>
                    <th class="text-left py-3 px-3 text-xs font-medium text-[var(--text-muted)]">Currency</th>
                    <th class="text-left py-3 px-3 text-xs font-medium text-[var(--text-muted)]">Type</th>
                  </tr>
                </thead>
                <tbody>
                  <tr
                    v-for="holiday in filteredHolidays"
                    :key="holiday.id"
                    :class="['border-b border-[var(--glass-border)] cursor-pointer transition-colors', selectedHolidayId === holiday.id ? 'bg-[var(--primary)]/10' : 'hover:bg-[var(--surface-hover)]']"
                    @click="selectHoliday(holiday.id)"
                  >
                    <td class="py-3 px-3 text-[var(--text-primary)] font-mono">{{ holiday.date }}</td>
                    <td class="py-3 px-3 text-[var(--text-primary)]">{{ holiday.name }}</td>
                    <td class="py-3 px-3 text-[var(--text-secondary)]">{{ holiday.country }}</td>
                    <td class="py-3 px-3 text-[var(--text-secondary)]">{{ holiday.currency || '-' }}</td>
                    <td class="py-3 px-3">
                      <span :class="[
                        'px-2 py-0.5 rounded text-xs',
                        holiday.type === 'bank' ? 'bg-blue-500/10 text-blue-400' :
                        holiday.type === 'market' ? 'bg-purple-500/10 text-purple-400' :
                        'bg-orange-500/10 text-orange-400'
                      ]">{{ holiday.type }}</span>
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
          <!-- Rate Details (Rates/FX) -->
          <template v-if="assetClass === 'Rates' || assetClass === 'FX'">
            <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Instrument Details</h3>

            <div v-if="!selectedRate" class="text-center py-8">
              <i class="fas fa-hand-pointer text-3xl text-[var(--text-muted)] mb-4"></i>
              <p class="text-[var(--text-muted)]">Select an instrument to view details</p>
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
          </template>

          <!-- IR Vol Details -->
          <template v-else-if="assetClass === 'IRVol'">
            <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Instrument Details</h3>

            <div v-if="!selectedIrVol" class="text-center py-8">
              <i class="fas fa-hand-pointer text-3xl text-[var(--text-muted)] mb-4"></i>
              <p class="text-[var(--text-muted)]">Select an instrument to view details</p>
            </div>

            <template v-else>
              <!-- Grid Point Info -->
              <div class="space-y-2 mb-4">
                <div class="flex justify-between text-sm">
                  <span class="text-[var(--text-muted)]">Currency</span>
                  <span class="text-[var(--text-primary)]">{{ selectedIrVol.currency }}</span>
                </div>
                <div class="flex justify-between text-sm">
                  <span class="text-[var(--text-muted)]">Expiry</span>
                  <span class="text-[var(--text-primary)]">{{ selectedIrVol.expiry }}</span>
                </div>
                <div class="flex justify-between text-sm">
                  <span class="text-[var(--text-muted)]">Tenor</span>
                  <span class="text-[var(--text-primary)]">{{ selectedIrVol.tenor }}</span>
                </div>
                <div class="flex justify-between text-sm">
                  <span class="text-[var(--text-muted)]">Vol Type</span>
                  <span class="text-[var(--text-primary)]">{{ selectedIrVol.volType }}</span>
                </div>
              </div>

              <!-- All Instruments List -->
              <div class="border-t border-[var(--glass-border)] pt-4">
                <h4 class="text-sm font-medium text-[var(--text-secondary)] mb-3">
                  Instruments ({{ selectedIrVol.instruments.length }})
                </h4>
                <div class="space-y-2 max-h-80 overflow-y-auto">
                  <div
                    v-for="inst in selectedIrVol.instruments"
                    :key="inst.id"
                    class="p-3 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)]"
                  >
                    <div class="flex items-center justify-between mb-2">
                      <span class="text-xs font-mono text-[var(--text-muted)]">{{ inst.id }}</span>
                      <span :class="[
                        'px-2 py-0.5 rounded text-xs',
                        inst.strike === 'ATM' ? 'bg-blue-500/10 text-blue-400' : 'bg-purple-500/10 text-purple-400'
                      ]">{{ inst.strike }}</span>
                    </div>
                    <div class="flex justify-between text-sm">
                      <span class="text-[var(--text-muted)]">Volatility</span>
                      <span class="text-[var(--text-primary)] font-mono">{{ formatVol(inst.vol) }}</span>
                    </div>
                  </div>
                </div>
              </div>

              <!-- Underlying Info -->
              <div class="mt-4 pt-4 border-t border-[var(--glass-border)]">
                <h4 class="text-sm font-medium text-[var(--text-secondary)] mb-2">Underlying</h4>
                <div class="text-xs space-y-1">
                  <div class="flex justify-between">
                    <span class="text-[var(--text-muted)]">Type</span>
                    <span class="text-[var(--text-primary)]">Interest Rate Swap</span>
                  </div>
                  <div class="flex justify-between">
                    <span class="text-[var(--text-muted)]">Index</span>
                    <span class="text-[var(--text-primary)]">{{ selectedIrVol.currency === 'USD' ? 'SOFR' : selectedIrVol.currency === 'EUR' ? 'EURIBOR' : selectedIrVol.currency + ' OIS' }}</span>
                  </div>
                </div>
              </div>
            </template>
          </template>

          <!-- FX Vol Details -->
          <template v-else-if="assetClass === 'FXVol'">
            <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Instrument Details</h3>

            <div v-if="!selectedFxVol" class="text-center py-8">
              <i class="fas fa-hand-pointer text-3xl text-[var(--text-muted)] mb-4"></i>
              <p class="text-[var(--text-muted)]">Select an instrument to view details</p>
            </div>

            <template v-else>
              <!-- Grid Point Info -->
              <div class="space-y-2 mb-4">
                <div class="flex justify-between text-sm">
                  <span class="text-[var(--text-muted)]">Currency Pair</span>
                  <span class="text-[var(--text-primary)]">{{ selectedFxVol.pair }}</span>
                </div>
                <div class="flex justify-between text-sm">
                  <span class="text-[var(--text-muted)]">Expiry</span>
                  <span class="text-[var(--text-primary)]">{{ selectedFxVol.expiryLabel }}</span>
                </div>
              </div>

              <!-- Market Instruments List -->
              <div class="border-t border-[var(--glass-border)] pt-4">
                <h4 class="text-sm font-medium text-[var(--text-secondary)] mb-3">
                  Market Instruments ({{ selectedFxVol.instruments.length }})
                </h4>
                <div class="space-y-2 max-h-60 overflow-y-auto">
                  <div
                    v-for="inst in selectedFxVol.instruments"
                    :key="inst.id"
                    class="p-3 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)]"
                  >
                    <div class="flex items-center justify-between mb-2">
                      <span class="text-xs font-mono text-[var(--text-muted)]">{{ inst.id }}</span>
                      <span :class="[
                        'px-2 py-0.5 rounded text-xs',
                        inst.instrumentType === 'ATM' ? 'bg-blue-500/10 text-blue-400' :
                        inst.instrumentType.includes('RR') ? 'bg-purple-500/10 text-purple-400' : 'bg-orange-500/10 text-orange-400'
                      ]">{{ inst.instrumentType }}</span>
                    </div>
                    <div class="flex justify-between text-sm">
                      <span class="text-[var(--text-muted)]">{{ inst.instrumentType === 'ATM' ? 'Volatility' : 'Spread' }}</span>
                      <span class="text-[var(--text-primary)] font-mono">{{ inst.unit === '%' ? formatVol(inst.value) : formatVolBps(inst.value) }}</span>
                    </div>
                  </div>
                </div>
              </div>

              <!-- Underlying Info -->
              <div class="mt-4 pt-4 border-t border-[var(--glass-border)]">
                <h4 class="text-sm font-medium text-[var(--text-secondary)] mb-2">Underlying</h4>
                <div class="text-xs space-y-1">
                  <div class="flex justify-between">
                    <span class="text-[var(--text-muted)]">Type</span>
                    <span class="text-[var(--text-primary)]">FX Vanilla Option</span>
                  </div>
                  <div class="flex justify-between">
                    <span class="text-[var(--text-muted)]">Base</span>
                    <span class="text-[var(--text-primary)]">{{ selectedFxVol.pair.slice(0, 3) }}</span>
                  </div>
                  <div class="flex justify-between">
                    <span class="text-[var(--text-muted)]">Quote</span>
                    <span class="text-[var(--text-primary)]">{{ selectedFxVol.pair.slice(3, 6) }}</span>
                  </div>
                </div>
              </div>
            </template>
          </template>

          <!-- Events Details -->
          <template v-else-if="assetClass === 'Events'">
            <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Event Details</h3>

            <div v-if="!selectedEvent" class="text-center py-8">
              <i class="fas fa-hand-pointer text-3xl text-[var(--text-muted)] mb-4"></i>
              <p class="text-[var(--text-muted)]">Select an event to view details</p>
            </div>

            <template v-else>
              <div class="space-y-3">
                <div class="flex justify-between text-sm">
                  <span class="text-[var(--text-muted)]">ID</span>
                  <span class="text-[var(--text-primary)] font-mono text-xs">{{ selectedEvent.id }}</span>
                </div>
                <div class="flex justify-between text-sm">
                  <span class="text-[var(--text-muted)]">Date</span>
                  <span class="text-[var(--text-primary)]">{{ selectedEvent.date }}</span>
                </div>
                <div v-if="selectedEvent.time" class="flex justify-between text-sm">
                  <span class="text-[var(--text-muted)]">Time</span>
                  <span class="text-[var(--text-primary)]">{{ selectedEvent.time }}</span>
                </div>
                <div class="flex justify-between text-sm">
                  <span class="text-[var(--text-muted)]">Type</span>
                  <span :class="[
                    'px-2 py-0.5 rounded text-xs',
                    isTurnEvent(selectedEvent.eventType) ? 'bg-orange-500/10 text-orange-400' : 'bg-[var(--primary)]/10 text-[var(--primary)]'
                  ]">{{ formatEventType(selectedEvent.eventType) }}</span>
                </div>
                <div class="flex justify-between text-sm">
                  <span class="text-[var(--text-muted)]">Title</span>
                  <span class="text-[var(--text-primary)] text-right max-w-[60%]">{{ selectedEvent.title }}</span>
                </div>
                <div v-if="selectedEvent.currency" class="flex justify-between text-sm">
                  <span class="text-[var(--text-muted)]">Currency</span>
                  <span class="text-[var(--text-primary)]">{{ selectedEvent.currency }}</span>
                </div>
                <div v-if="isTurnEvent(selectedEvent.eventType) && selectedEvent.expectedSpikeBp !== undefined" class="flex justify-between text-sm">
                  <span class="text-[var(--text-muted)]">Expected Jump</span>
                  <span class="text-orange-400 font-mono">{{ formatSpikeBp(selectedEvent.expectedSpikeBp) }}</span>
                </div>
                <div v-if="selectedEvent.region" class="flex justify-between text-sm">
                  <span class="text-[var(--text-muted)]">Region</span>
                  <span class="text-[var(--text-primary)]">{{ selectedEvent.region }}</span>
                </div>
                <div class="flex justify-between text-sm">
                  <span class="text-[var(--text-muted)]">Importance</span>
                  <span :class="['px-2 py-0.5 rounded text-xs', selectedEvent.importance === 'High' || selectedEvent.importance === 'critical' ? 'bg-red-500/10 text-red-400' : selectedEvent.importance === 'Medium' || selectedEvent.importance === 'medium' ? 'bg-yellow-500/10 text-yellow-400' : 'bg-green-500/10 text-green-400']">
                    {{ selectedEvent.importance }}
                  </span>
                </div>
                <div class="flex justify-between text-sm">
                  <span class="text-[var(--text-muted)]">Source</span>
                  <span class="text-[var(--text-primary)]">{{ selectedEvent.source }}</span>
                </div>
              </div>

              <!-- Economic Data (if available) -->
              <div v-if="selectedEvent.previous || selectedEvent.forecast || selectedEvent.actual" class="mt-4 pt-4 border-t border-[var(--glass-border)]">
                <h4 class="text-sm font-medium text-[var(--text-secondary)] mb-3">Economic Data</h4>
                <div class="space-y-2">
                  <div v-if="selectedEvent.previous" class="flex justify-between text-sm">
                    <span class="text-[var(--text-muted)]">Previous</span>
                    <span class="text-[var(--text-primary)] font-mono">{{ selectedEvent.previous }}</span>
                  </div>
                  <div v-if="selectedEvent.forecast" class="flex justify-between text-sm">
                    <span class="text-[var(--text-muted)]">Forecast</span>
                    <span class="text-[var(--text-primary)] font-mono">{{ selectedEvent.forecast }}</span>
                  </div>
                  <div v-if="selectedEvent.actual" class="flex justify-between text-sm">
                    <span class="text-[var(--text-muted)]">Actual</span>
                    <span class="text-[var(--text-primary)] font-mono">{{ selectedEvent.actual }}</span>
                  </div>
                </div>
              </div>
            </template>
          </template>

          <!-- Holidays Details -->
          <template v-else-if="assetClass === 'Holidays'">
            <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Holiday Details</h3>

            <div v-if="!selectedHoliday" class="text-center py-8">
              <i class="fas fa-hand-pointer text-3xl text-[var(--text-muted)] mb-4"></i>
              <p class="text-[var(--text-muted)]">Select a holiday to view details</p>
            </div>

            <template v-else>
              <div class="space-y-3">
                <div class="flex justify-between text-sm">
                  <span class="text-[var(--text-muted)]">ID</span>
                  <span class="text-[var(--text-primary)] font-mono text-xs">{{ selectedHoliday.id }}</span>
                </div>
                <div class="flex justify-between text-sm">
                  <span class="text-[var(--text-muted)]">Date</span>
                  <span class="text-[var(--text-primary)]">{{ selectedHoliday.date }}</span>
                </div>
                <div class="flex justify-between text-sm">
                  <span class="text-[var(--text-muted)]">Name</span>
                  <span class="text-[var(--text-primary)] text-right max-w-[60%]">{{ selectedHoliday.name }}</span>
                </div>
                <div class="flex justify-between text-sm">
                  <span class="text-[var(--text-muted)]">Country</span>
                  <span class="text-[var(--text-primary)]">{{ selectedHoliday.country }}</span>
                </div>
                <div v-if="selectedHoliday.currency" class="flex justify-between text-sm">
                  <span class="text-[var(--text-muted)]">Currency</span>
                  <span class="text-[var(--text-primary)]">{{ selectedHoliday.currency }}</span>
                </div>
                <div class="flex justify-between text-sm">
                  <span class="text-[var(--text-muted)]">Type</span>
                  <span :class="[
                    'px-2 py-0.5 rounded text-xs',
                    selectedHoliday.type === 'bank' ? 'bg-blue-500/10 text-blue-400' :
                    selectedHoliday.type === 'market' ? 'bg-purple-500/10 text-purple-400' :
                    'bg-orange-500/10 text-orange-400'
                  ]">{{ selectedHoliday.type }}</span>
                </div>
              </div>

              <!-- Calendar Impact -->
              <div class="mt-4 pt-4 border-t border-[var(--glass-border)]">
                <h4 class="text-sm font-medium text-[var(--text-secondary)] mb-3">Calendar Impact</h4>
                <div class="space-y-2 text-xs">
                  <div class="flex justify-between">
                    <span class="text-[var(--text-muted)]">Settlement</span>
                    <span class="text-[var(--text-primary)]">{{ selectedHoliday.type === 'settlement' || selectedHoliday.type === 'bank' ? 'Affected' : 'Not Affected' }}</span>
                  </div>
                  <div class="flex justify-between">
                    <span class="text-[var(--text-muted)]">Market Trading</span>
                    <span class="text-[var(--text-primary)]">{{ selectedHoliday.type === 'market' || selectedHoliday.type === 'bank' ? 'Closed' : 'Open' }}</span>
                  </div>
                </div>
              </div>
            </template>
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
