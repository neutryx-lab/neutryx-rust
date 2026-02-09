<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue';
import { useToast } from '@/composables/useToast';

const toast = useToast();

// Types
interface Instrument {
  id?: string;
  type?: string;
  instrumentType?: string;
  displayName?: string;
  name?: string;
  assetClass?: string;
  assetClassName?: string;
  requiredParams?: ParamField[];
  optionalParams?: ParamField[];
}

interface ParamField {
  name: string;
  label?: string;
  fieldType?: 'string' | 'number' | 'date' | 'select';
  defaultValue?: string | number | null;
  options?: Array<{ value: string; label: string }>;
  validation?: { min?: number; max?: number };
}

interface Cashflow {
  paymentDate: string;
  accrualStart: string;
  accrualEnd: string;
  yearFraction: number;
  notional: number;
  rate: number | null;
  payoffType: string;
  rateIndex?: string;
}

interface Leg {
  direction: string;
  currency: string;
  legType: string;
  rateIndex?: string;
  cashflows: Cashflow[];
}

interface ExpandedTrade {
  tradeId: string;
  tradeType: string;
  legs: Leg[];
  metadata: {
    totalLegs: number;
    totalCashflows: number;
    processingTimeMs: number;
  };
}

interface CashflowPvResult {
  pv: number;
  discountFactor: number;
  paymentDate: string;
}

interface LegPvDetail {
  direction: string;
  pv: number;
  currency: string;
  pvOriginal?: number;
  fxRate?: number;
  cashflows?: CashflowPvResult[];
}

interface PricingResult {
  totalPv?: number;
  pv?: number;
  currency?: string;
  legs?: LegPvDetail[];
}

interface GreeksResult {
  delta: number;
  gamma: number | null;
  theta: number | null;
  vega: number | null;
  currency?: string;
}

interface HistoryEntry {
  id: number;
  timestamp: number;
  instrumentId: string;
  instrumentName: string;
  params: Record<string, string | number>;
  pricingResult: PricingResult;
  greeksResult: GreeksResult | null;
  valuationDate: string;
  reportingCcy: string;
  modelType: string;
  curveIndex: string;
}

interface ModelParamDef {
  name: string;
  label: string;
  defaultValue: number;
  min?: number;
  max?: number;
  step?: number;
}

interface StochasticModelConfig {
  type: string;
  label: string;
  params: ModelParamDef[];
}

interface ValidationError {
  field: string;
  message: string;
}

// Constants
const STOCHASTIC_MODELS: StochasticModelConfig[] = [
  {
    type: 'GBM', label: 'Geometric Brownian Motion',
    params: [
      { name: 'drift', label: 'Drift', defaultValue: 0.05, min: -0.5, max: 0.5, step: 0.01 },
      { name: 'vol', label: 'Volatility', defaultValue: 0.2, min: 0.01, max: 2.0, step: 0.01 },
    ],
  },
  {
    type: 'Heston', label: 'Heston Stochastic Vol',
    params: [
      { name: 'v0', label: 'Initial Vol', defaultValue: 0.04, min: 0.001, max: 1.0, step: 0.001 },
      { name: 'kappa', label: 'Mean Reversion', defaultValue: 2.0, min: 0.01, max: 10.0, step: 0.1 },
      { name: 'theta', label: 'Long-run Vol', defaultValue: 0.04, min: 0.001, max: 1.0, step: 0.001 },
      { name: 'sigma', label: 'Vol of Vol', defaultValue: 0.3, min: 0.01, max: 2.0, step: 0.01 },
      { name: 'rho', label: 'Correlation', defaultValue: -0.7, min: -1.0, max: 1.0, step: 0.05 },
    ],
  },
  {
    type: 'HullWhite', label: 'Hull-White',
    params: [
      { name: 'a', label: 'Mean Reversion (a)', defaultValue: 0.1, min: 0.001, max: 1.0, step: 0.01 },
      { name: 'sigma', label: 'Volatility', defaultValue: 0.01, min: 0.001, max: 0.1, step: 0.001 },
    ],
  },
  {
    type: 'CIR', label: 'Cox-Ingersoll-Ross',
    params: [
      { name: 'kappa', label: 'Mean Reversion', defaultValue: 0.5, min: 0.01, max: 5.0, step: 0.1 },
      { name: 'theta', label: 'Long-run Mean', defaultValue: 0.05, min: 0.001, max: 0.5, step: 0.005 },
      { name: 'sigma', label: 'Volatility', defaultValue: 0.1, min: 0.001, max: 1.0, step: 0.01 },
    ],
  },
];

const CURVE_OPTIONS = [
  { index: 'USD-SOFR', label: 'USD SOFR', currency: 'USD' },
  { index: 'EUR-ESTR', label: 'EUR ESTR', currency: 'EUR' },
  { index: 'JPY-TONA', label: 'JPY TONA', currency: 'JPY' },
  { index: 'GBP-SONIA', label: 'GBP SONIA', currency: 'GBP' },
];

// State
const instruments = ref<Instrument[]>([]);
const selectedInstrumentId = ref('');
const instrumentParams = ref<Record<string, string | number>>({});
const expandedTrade = ref<ExpandedTrade | null>(null);
const editedCashflows = ref<Record<string, { notional?: number; rate?: number }>>({});
const pricingResult = ref<PricingResult | null>(null);
const greeksResult = ref<GreeksResult | null>(null);

const valuationDate = ref(new Date().toISOString().split('T')[0]);
const reportingCcy = ref('USD');
const useDefaults = ref(true);
const numPaths = ref(10000);
const numSteps = ref(100);
const seed = ref<number | null>(null);
const rateBump = ref(1);
const fxBump = ref(1);
const volBump = ref(1);

const isExpanding = ref(false);
const isCalculating = ref(false);
const apiAvailable = ref(true);

// Market Data / Curve (R4)
const selectedCurveIndex = ref('USD-SOFR');

// Stochastic Model (R5)
const modelType = ref('GBM');
const modelParams = ref<Record<string, number>>({ drift: 0.05, vol: 0.2 });

// Result History (R9)
const resultHistory = ref<HistoryEntry[]>([]);
const compareMode = ref(false);
const compareIndices = ref<[number, number]>([0, 1]);
let historyCounter = 0;

// Validation (C2)
const validationErrors = ref<ValidationError[]>([]);

// Computation Metrics (C4)
const computationMetrics = ref<{ pricingTimeMs: number; method: string; timestamp: number } | null>(null);

// Computed
const selectedInstrument = computed(() =>
  instruments.value.find(inst =>
    (inst.instrumentType || inst.id || inst.type) === selectedInstrumentId.value
  )
);

const groupedInstruments = computed(() => {
  const groups: Record<string, Instrument[]> = {};
  instruments.value.forEach(inst => {
    const assetClass = inst.assetClassName || inst.assetClass || 'Other';
    if (!groups[assetClass]) groups[assetClass] = [];
    groups[assetClass].push(inst);
  });
  return groups;
});

const hasEdits = computed(() => Object.keys(editedCashflows.value).length > 0);

const summaryStats = computed(() => [
  { label: 'Valuation Date', value: valuationDate.value, icon: 'fa-calendar', color: '#10b981' },
  { label: 'Instrument', value: selectedInstrument.value?.displayName || selectedInstrument.value?.name || '-', icon: 'fa-file-contract', color: '#3b82f6' },
  { label: 'PV', value: pricingResult.value ? formatCurrency(pricingResult.value.totalPv ?? pricingResult.value.pv ?? 0) : '-', icon: 'fa-dollar-sign', color: '#8b5cf6' },
  { label: 'DV01', value: greeksResult.value ? formatCurrency(greeksResult.value.delta) : '-', icon: 'fa-chart-line', color: '#f59e0b' },
]);

const selectedModelConfig = computed(() =>
  STOCHASTIC_MODELS.find(m => m.type === modelType.value) || STOCHASTIC_MODELS[0]
);

const recentHistory = computed(() => resultHistory.value.slice(0, 5));

const pvDiff = computed(() => {
  if (resultHistory.value.length < 2) return null;
  const current = resultHistory.value[0].pricingResult;
  const previous = resultHistory.value[1].pricingResult;
  const curPv = current.totalPv ?? current.pv ?? 0;
  const prevPv = previous.totalPv ?? previous.pv ?? 0;
  const diff = curPv - prevPv;
  const pct = prevPv !== 0 ? (diff / Math.abs(prevPv)) * 100 : 0;
  return { absolute: diff, percent: pct };
});

const comparedResults = computed(() => {
  if (!compareMode.value || resultHistory.value.length < 2) return null;
  const [idxA, idxB] = compareIndices.value;
  const a = resultHistory.value[idxA];
  const b = resultHistory.value[idxB];
  if (!a || !b) return null;
  return { a, b };
});

const changedParams = computed(() => {
  if (!comparedResults.value) return [];
  const { a, b } = comparedResults.value;
  const changes: Array<{ name: string; valueA: string | number; valueB: string | number }> = [];
  const allKeys = new Set([...Object.keys(a.params), ...Object.keys(b.params)]);
  allKeys.forEach(key => {
    if (a.params[key] !== b.params[key]) {
      changes.push({ name: key, valueA: a.params[key], valueB: b.params[key] });
    }
  });
  if (a.modelType !== b.modelType) changes.push({ name: 'Model', valueA: a.modelType, valueB: b.modelType });
  if (a.curveIndex !== b.curveIndex) changes.push({ name: 'Curve', valueA: a.curveIndex, valueB: b.curveIndex });
  if (a.valuationDate !== b.valuationDate) changes.push({ name: 'Val Date', valueA: a.valuationDate, valueB: b.valuationDate });
  return changes;
});

const currencyAggregation = computed(() => {
  if (!pricingResult.value?.legs) return [];
  const byCcy: Record<string, number> = {};
  pricingResult.value.legs.forEach(leg => {
    const ccy = leg.currency || reportingCcy.value;
    byCcy[ccy] = (byCcy[ccy] || 0) + leg.pv;
  });
  return Object.entries(byCcy).map(([ccy, pv]) => ({ ccy, pv }));
});

// Utility functions
function formatCurrency(value: number): string {
  const absValue = Math.abs(value);
  if (absValue >= 1_000_000) return `$${(value / 1_000_000).toFixed(2)}M`;
  if (absValue >= 1_000) return `$${(value / 1_000).toFixed(1)}K`;
  return `$${value.toFixed(0)}`;
}

function formatNumberCompact(value: number): string {
  if (Math.abs(value) >= 1_000_000) return `${(value / 1_000_000).toFixed(2)}M`;
  if (Math.abs(value) >= 1_000) return `${(value / 1_000).toFixed(0)}K`;
  return value.toFixed(0);
}

function parseFormattedNumber(str: string): number {
  const cleaned = str.replace(/[,$\s]/g, '').toUpperCase();
  const multipliers: Record<string, number> = { K: 1_000, M: 1_000_000, B: 1_000_000_000 };
  const match = cleaned.match(/^(-?[\d.]+)([KMB])?$/);
  if (!match) return parseFloat(cleaned) || 0;
  const base = parseFloat(match[1]);
  const mult = multipliers[match[2]] || 1;
  return base * mult;
}

function getFieldError(fieldName: string): string | undefined {
  return validationErrors.value.find(e => e.field === fieldName)?.message;
}

function validateParams(): ValidationError[] {
  const errors: ValidationError[] = [];
  if (!selectedInstrumentId.value) {
    errors.push({ field: 'instrumentType', message: 'Please select an instrument' });
  }
  const inst = selectedInstrument.value;
  if (inst?.requiredParams) {
    inst.requiredParams.forEach(param => {
      const val = instrumentParams.value[param.name];
      if (val === undefined || val === '' || val === null) {
        errors.push({ field: param.name, message: `${param.label || param.name} is required` });
      }
      if (param.validation && val !== undefined && val !== '') {
        const numVal = Number(val);
        if (param.validation.min !== undefined && numVal < param.validation.min) {
          errors.push({ field: param.name, message: `Minimum value is ${param.validation.min}` });
        }
        if (param.validation.max !== undefined && numVal > param.validation.max) {
          errors.push({ field: param.name, message: `Maximum value is ${param.validation.max}` });
        }
      }
    });
  }
  return errors;
}

// API calls
async function loadInstruments() {
  try {
    const response = await fetch('/api/pricer/instruments');
    if (!response.ok) throw new Error('Failed to load instruments');
    const data = await response.json();
    instruments.value = data.instruments || [];

    // Auto-select IRS and set USD OIS 5Y defaults
    const irs = instruments.value.find(inst =>
      ['IRS', 'irs'].includes(inst.instrumentType || inst.id || inst.type || '')
    );
    if (irs) {
      selectedInstrumentId.value = irs.instrumentType || irs.id || irs.type || 'IRS';
      const today = new Date();
      const fiveYears = new Date(today);
      fiveYears.setFullYear(fiveYears.getFullYear() + 5);
      instrumentParams.value = {
        notional: 1_000_000,
        currency: 'USD',
        startDate: today.toISOString().split('T')[0],
        endDate: fiveYears.toISOString().split('T')[0],
        fixedRate: 0.04,
      };
      setTimeout(() => expandCashflows(), 100);
    }
  } catch (error) {
    console.error('Failed to load instruments:', error);
    apiAvailable.value = false;
    toast.error('Failed to load instruments. API may be unavailable.');
  }
}

async function expandCashflows() {
  if (!selectedInstrumentId.value) return;

  validationErrors.value = validateParams();
  if (validationErrors.value.length > 0) {
    toast.warning('Please fix validation errors before expanding.');
    return;
  }

  isExpanding.value = true;
  try {
    const response = await fetch('/api/pricer/expand', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        instrumentType: selectedInstrumentId.value,
        params: { ...instrumentParams.value },
      }),
    });

    if (!response.ok) throw new Error('Failed to expand cashflows');
    expandedTrade.value = await response.json();
    editedCashflows.value = {};
    pricingResult.value = null;
    greeksResult.value = null;
  } catch (error) {
    console.error('Failed to expand cashflows:', error);
    toast.error(`Failed to expand cashflows: ${(error as Error).message}`);
  } finally {
    isExpanding.value = false;
  }
}

async function calculateAll() {
  if (!selectedInstrumentId.value || !expandedTrade.value) return;

  isCalculating.value = true;
  const startTime = performance.now();

  try {
    const request = buildPricingRequest();
    const [priceRes, greeksRes] = await Promise.all([
      fetch('/api/pricer/price', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(request),
      }),
      fetch('/api/pricer/greeks', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          ...request,
          bumpSizes: { rateBumpBp: rateBump.value, fxBumpPct: fxBump.value, volBumpPct: volBump.value },
        }),
      }),
    ]);

    const endTime = performance.now();

    if (priceRes.ok) {
      pricingResult.value = await priceRes.json();
    } else {
      const errBody = await priceRes.json().catch(() => ({}));
      toast.error(`Pricing failed: ${errBody.message || errBody.error || `HTTP ${priceRes.status}`}`);
    }

    if (greeksRes.ok) {
      greeksResult.value = await greeksRes.json();
    } else {
      toast.warning('Greeks calculation failed. PV result may still be valid.');
    }

    computationMetrics.value = {
      pricingTimeMs: endTime - startTime,
      method: modelType.value,
      timestamp: Date.now(),
    };

    if (pricingResult.value) {
      addToHistory();
      toast.success('Pricing complete');
    }
  } catch (error) {
    console.error('Calculation failed:', error);
    toast.error(`Calculation failed: ${(error as Error).message}`);
  } finally {
    isCalculating.value = false;
  }
}

function buildPricingRequest() {
  const legs: Array<{
    currency: string;
    direction: 'payer' | 'receiver';
    cashflows: Array<{ paymentDate: string; amount: number }>;
  }> = [];

  if (expandedTrade.value?.legs) {
    expandedTrade.value.legs.forEach((leg, legIdx) => {
      const cashflows = leg.cashflows.map((cf, cfIdx) => {
        const key = `${legIdx}-${cfIdx}`;
        const edited = editedCashflows.value[key] || {};
        const notional = edited.notional !== undefined ? edited.notional : cf.notional;
        const rate = edited.rate !== undefined ? edited.rate : (cf.rate || 0);
        return { paymentDate: cf.paymentDate, amount: notional * rate * cf.yearFraction };
      });
      legs.push({
        currency: leg.currency,
        direction: leg.direction.toLowerCase() as 'payer' | 'receiver',
        cashflows,
      });
    });
  }

  return {
    valuationDate: valuationDate.value,
    reportingCurrency: reportingCcy.value,
    curveIndex: selectedCurveIndex.value,
    modelType: modelType.value,
    modelParams: { ...modelParams.value },
    legs,
    modelConfig: useDefaults.value ? null : { numPaths: numPaths.value, numSteps: numSteps.value, seed: seed.value },
  };
}

function addToHistory() {
  if (!pricingResult.value) return;
  const entry: HistoryEntry = {
    id: ++historyCounter,
    timestamp: Date.now(),
    instrumentId: selectedInstrumentId.value,
    instrumentName: selectedInstrument.value?.displayName || selectedInstrument.value?.name || selectedInstrumentId.value,
    params: { ...instrumentParams.value },
    pricingResult: structuredClone(pricingResult.value),
    greeksResult: greeksResult.value ? structuredClone(greeksResult.value) : null,
    valuationDate: valuationDate.value,
    reportingCcy: reportingCcy.value,
    modelType: modelType.value,
    curveIndex: selectedCurveIndex.value,
  };
  resultHistory.value.unshift(entry);
  if (resultHistory.value.length > 5) {
    resultHistory.value = resultHistory.value.slice(0, 5);
  }
}

function restoreFromHistory(entry: HistoryEntry) {
  selectedInstrumentId.value = entry.instrumentId;
  instrumentParams.value = { ...entry.params };
  valuationDate.value = entry.valuationDate;
  reportingCcy.value = entry.reportingCcy;
  modelType.value = entry.modelType;
  selectedCurveIndex.value = entry.curveIndex;
  pricingResult.value = structuredClone(entry.pricingResult);
  greeksResult.value = entry.greeksResult ? structuredClone(entry.greeksResult) : null;
  toast.info('Restored pricing result from history');
}

function updateCashflow(legIdx: number, cfIdx: number, field: 'notional' | 'rate', value: number) {
  const key = `${legIdx}-${cfIdx}`;
  if (!editedCashflows.value[key]) editedCashflows.value[key] = {};
  editedCashflows.value[key][field] = value;
}

function resetCashflows() {
  expandedTrade.value = null;
  editedCashflows.value = {};
  pricingResult.value = null;
  greeksResult.value = null;
}

function resetEdits() {
  editedCashflows.value = {};
}

// Watchers
watch(selectedInstrumentId, () => {
  instrumentParams.value = {};
  expandedTrade.value = null;
  editedCashflows.value = {};
  pricingResult.value = null;
  greeksResult.value = null;
});

watch(modelType, () => {
  const config = STOCHASTIC_MODELS.find(m => m.type === modelType.value);
  if (config) {
    const defaults: Record<string, number> = {};
    config.params.forEach(p => { defaults[p.name] = p.defaultValue; });
    modelParams.value = defaults;
  }
});

watch(instrumentParams, () => {
  validationErrors.value = [];
}, { deep: true });

// Initialize
onMounted(() => {
  loadInstruments();
});
</script>

<template>
  <div class="pricer-view">
    <!-- Summary Stats -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
      <div v-for="stat in summaryStats" :key="stat.label" class="glass-card p-4">
        <div class="flex items-start justify-between">
          <div>
            <p class="text-sm text-[var(--text-muted)] mb-1">{{ stat.label }}</p>
            <p class="text-2xl font-semibold text-[var(--text-primary)] truncate">{{ stat.value }}</p>
          </div>
          <div class="w-10 h-10 rounded-lg flex items-center justify-center flex-shrink-0" :style="{ backgroundColor: `${stat.color}1a` }">
            <i :class="['fas', stat.icon]" :style="{ color: stat.color }"></i>
          </div>
        </div>
      </div>
    </div>

    <!-- API Not Available -->
    <div v-if="!apiAvailable" class="glass-card p-8 text-center">
      <i class="fas fa-info-circle text-4xl text-[var(--text-muted)] mb-4"></i>
      <p class="text-[var(--text-muted)]">Pricer API is not available in this build configuration.</p>
    </div>

    <template v-else>
      <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <!-- Left Panel: Configuration -->
        <div class="space-y-6">
          <!-- Instrument Selection -->
          <div class="glass-card p-6">
            <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Trade Setup</h3>
            <div class="space-y-4">
              <div>
                <label class="block text-sm text-[var(--text-muted)] mb-2">Instrument Type</label>
                <select v-model="selectedInstrumentId" :class="['w-full px-4 py-2.5 rounded-lg bg-[var(--surface)] text-[var(--text-primary)] focus:outline-none focus:ring-2', getFieldError('instrumentType') ? 'border-2 border-[var(--danger)] focus:ring-[var(--danger)]' : 'border border-[var(--glass-border)] focus:ring-[var(--primary)]']">
                  <option value="">Select instrument...</option>
                  <optgroup v-for="(items, group) in groupedInstruments" :key="group" :label="group">
                    <option v-for="inst in items" :key="inst.instrumentType || inst.id || inst.type" :value="inst.instrumentType || inst.id || inst.type">
                      {{ inst.displayName || inst.name || inst.instrumentType || inst.id }}
                    </option>
                  </optgroup>
                </select>
                <p v-if="getFieldError('instrumentType')" class="text-xs text-[var(--danger)] mt-1">{{ getFieldError('instrumentType') }}</p>
              </div>

              <!-- Dynamic Parameter Form -->
              <template v-if="selectedInstrument?.requiredParams?.length || selectedInstrument?.optionalParams?.length">
                <div v-for="param in selectedInstrument.requiredParams" :key="param.name" class="form-field">
                  <label class="block text-sm text-[var(--text-muted)] mb-2">{{ param.label || param.name }} <span class="text-red-500">*</span></label>
                  <input
                    v-if="param.fieldType === 'number'"
                    type="number"
                    v-model.number="instrumentParams[param.name]"
                    :class="['w-full px-4 py-2.5 rounded-lg bg-[var(--surface)] text-[var(--text-primary)] focus:outline-none focus:ring-2', getFieldError(param.name) ? 'border-2 border-[var(--danger)] focus:ring-[var(--danger)]' : 'border border-[var(--glass-border)] focus:ring-[var(--primary)]']"
                  >
                  <input
                    v-else-if="param.fieldType === 'date'"
                    type="date"
                    v-model="instrumentParams[param.name]"
                    :class="['w-full px-4 py-2.5 rounded-lg bg-[var(--surface)] text-[var(--text-primary)] focus:outline-none focus:ring-2', getFieldError(param.name) ? 'border-2 border-[var(--danger)] focus:ring-[var(--danger)]' : 'border border-[var(--glass-border)] focus:ring-[var(--primary)]']"
                  >
                  <select
                    v-else-if="param.fieldType === 'select'"
                    v-model="instrumentParams[param.name]"
                    :class="['w-full px-4 py-2.5 rounded-lg bg-[var(--surface)] text-[var(--text-primary)] focus:outline-none focus:ring-2', getFieldError(param.name) ? 'border-2 border-[var(--danger)] focus:ring-[var(--danger)]' : 'border border-[var(--glass-border)] focus:ring-[var(--primary)]']"
                  >
                    <option v-for="opt in param.options" :key="opt.value" :value="opt.value">{{ opt.label }}</option>
                  </select>
                  <input
                    v-else
                    type="text"
                    v-model="instrumentParams[param.name]"
                    :class="['w-full px-4 py-2.5 rounded-lg bg-[var(--surface)] text-[var(--text-primary)] focus:outline-none focus:ring-2', getFieldError(param.name) ? 'border-2 border-[var(--danger)] focus:ring-[var(--danger)]' : 'border border-[var(--glass-border)] focus:ring-[var(--primary)]']"
                  >
                  <p v-if="getFieldError(param.name)" class="text-xs text-[var(--danger)] mt-1">{{ getFieldError(param.name) }}</p>
                </div>
              </template>
            </div>
          </div>

          <!-- Valuation Settings -->
          <div class="glass-card p-6">
            <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Valuation Settings</h3>
            <div class="space-y-4">
              <div>
                <label class="block text-sm text-[var(--text-muted)] mb-2">Valuation Date</label>
                <input type="date" v-model="valuationDate" class="w-full px-4 py-2.5 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--primary)]">
              </div>
              <div>
                <label class="block text-sm text-[var(--text-muted)] mb-2">Reporting Currency</label>
                <select v-model="reportingCcy" class="w-full px-4 py-2.5 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--primary)]">
                  <option value="USD">USD</option>
                  <option value="EUR">EUR</option>
                  <option value="GBP">GBP</option>
                  <option value="JPY">JPY</option>
                </select>
              </div>
              <label class="flex items-center gap-3 cursor-pointer">
                <input type="checkbox" v-model="useDefaults" class="w-5 h-5 rounded border-[var(--glass-border)] bg-[var(--surface)] text-[var(--primary)] focus:ring-[var(--primary)]">
                <span class="text-sm text-[var(--text-secondary)]">Use Default Model Config</span>
              </label>
              <template v-if="!useDefaults">
                <div class="grid grid-cols-2 gap-3">
                  <div>
                    <label class="block text-xs text-[var(--text-muted)] mb-1">Paths</label>
                    <input type="number" v-model.number="numPaths" class="w-full px-3 py-2 text-sm rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)]">
                  </div>
                  <div>
                    <label class="block text-xs text-[var(--text-muted)] mb-1">Steps</label>
                    <input type="number" v-model.number="numSteps" class="w-full px-3 py-2 text-sm rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)]">
                  </div>
                </div>
              </template>
              <!-- Bump Settings -->
              <div class="grid grid-cols-3 gap-3">
                <div>
                  <label class="block text-xs text-[var(--text-muted)] mb-1">Rate Bump (bp)</label>
                  <input type="number" v-model.number="rateBump" step="0.1" class="w-full px-3 py-2 text-sm rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)]">
                </div>
                <div>
                  <label class="block text-xs text-[var(--text-muted)] mb-1">FX Bump (%)</label>
                  <input type="number" v-model.number="fxBump" step="0.1" class="w-full px-3 py-2 text-sm rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)]">
                </div>
                <div>
                  <label class="block text-xs text-[var(--text-muted)] mb-1">Vol Bump (%)</label>
                  <input type="number" v-model.number="volBump" step="0.1" class="w-full px-3 py-2 text-sm rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)]">
                </div>
              </div>
            </div>
          </div>

          <!-- Market Data / Curve Selection (R4) -->
          <div class="glass-card p-6">
            <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Market Data</h3>
            <div class="space-y-4">
              <div>
                <label class="block text-sm text-[var(--text-muted)] mb-2">Discount Curve</label>
                <select v-model="selectedCurveIndex" class="w-full px-4 py-2.5 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--primary)]">
                  <option v-for="c in CURVE_OPTIONS" :key="c.index" :value="c.index">{{ c.label }}</option>
                </select>
              </div>
            </div>
          </div>

          <!-- Stochastic Model Selection (R5) -->
          <div class="glass-card p-6">
            <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Stochastic Model</h3>
            <div class="space-y-4">
              <div>
                <label class="block text-sm text-[var(--text-muted)] mb-2">Model Type</label>
                <select v-model="modelType" class="w-full px-4 py-2.5 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--primary)]">
                  <option v-for="m in STOCHASTIC_MODELS" :key="m.type" :value="m.type">{{ m.label }}</option>
                </select>
              </div>
              <div v-for="param in selectedModelConfig.params" :key="param.name">
                <label class="block text-xs text-[var(--text-muted)] mb-1">{{ param.label }}</label>
                <input
                  type="number"
                  v-model.number="modelParams[param.name]"
                  :min="param.min"
                  :max="param.max"
                  :step="param.step"
                  class="w-full px-3 py-2 text-sm rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)]"
                >
              </div>
            </div>
          </div>

          <!-- Actions -->
          <div class="glass-card p-6">
            <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Actions</h3>
            <div class="space-y-3">
              <button
                :disabled="!selectedInstrumentId || isExpanding"
                class="w-full px-4 py-2.5 rounded-lg bg-[var(--surface)] text-[var(--text-primary)] font-medium transition-all duration-200 hover:bg-[var(--surface-hover)] disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
                @click="expandCashflows"
              >
                <i :class="['fas', isExpanding ? 'fa-spinner fa-spin' : 'fa-expand']"></i>
                {{ isExpanding ? 'Expanding...' : 'Expand Cashflows' }}
              </button>
              <button
                :disabled="!expandedTrade || isCalculating"
                class="w-full px-4 py-2.5 rounded-lg bg-[var(--primary)] text-white font-medium transition-all duration-200 hover:opacity-90 disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
                @click="calculateAll"
              >
                <i :class="['fas', isCalculating ? 'fa-spinner fa-spin' : 'fa-play']"></i>
                {{ isCalculating ? 'Calculating...' : 'Price & Risks' }}
              </button>
              <button
                :disabled="!expandedTrade"
                class="w-full px-4 py-2 rounded-lg bg-[var(--surface)] text-[var(--text-secondary)] font-medium transition-all duration-200 hover:bg-[var(--surface-hover)] disabled:opacity-50 disabled:cursor-not-allowed"
                @click="resetCashflows"
              >
                <i class="fas fa-undo mr-2"></i>Reset
              </button>
            </div>
          </div>

          <!-- Results: PV -->
          <div v-if="pricingResult" class="glass-card p-6">
            <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Present Value</h3>
            <div :class="['text-3xl font-bold text-center py-4', (pricingResult.totalPv ?? pricingResult.pv ?? 0) >= 0 ? 'text-[var(--success)]' : 'text-[var(--danger)]']">
              {{ formatCurrency(pricingResult.totalPv ?? pricingResult.pv ?? 0) }}
            </div>
            <!-- PV Diff from Previous (R9) -->
            <div v-if="pvDiff" class="mt-2 text-center text-sm">
              <span :class="pvDiff.absolute >= 0 ? 'text-[var(--success)]' : 'text-[var(--danger)]'">
                {{ pvDiff.absolute >= 0 ? '+' : '' }}{{ formatCurrency(pvDiff.absolute) }}
                ({{ pvDiff.percent >= 0 ? '+' : '' }}{{ pvDiff.percent.toFixed(2) }}%)
              </span>
              <span class="text-[var(--text-muted)]"> vs previous</span>
            </div>
            <!-- Leg-level PV (R7) -->
            <div v-if="pricingResult.legs" class="mt-4 space-y-2">
              <div v-for="(leg, idx) in pricingResult.legs" :key="idx" class="flex justify-between text-sm">
                <span class="text-[var(--text-muted)]">
                  Leg {{ idx + 1 }} ({{ leg.direction }})
                  <span class="ml-1 px-1.5 py-0.5 rounded bg-[var(--surface)] text-xs">{{ leg.currency }}</span>
                </span>
                <span :class="leg.pv >= 0 ? 'text-[var(--success)]' : 'text-[var(--danger)]'">{{ formatCurrency(leg.pv) }}</span>
              </div>
              <!-- Currency Aggregation (R7) -->
              <div v-if="currencyAggregation.length > 1" class="pt-2 border-t border-[var(--glass-border)]">
                <p class="text-xs text-[var(--text-muted)] mb-1">By Currency</p>
                <div v-for="agg in currencyAggregation" :key="agg.ccy" class="flex justify-between text-sm">
                  <span class="text-[var(--text-muted)]">{{ agg.ccy }}</span>
                  <span :class="agg.pv >= 0 ? 'text-[var(--success)]' : 'text-[var(--danger)]'">{{ formatCurrency(agg.pv) }}</span>
                </div>
              </div>
            </div>
          </div>

          <!-- Results: Greeks -->
          <div v-if="greeksResult" class="glass-card p-6">
            <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Greeks</h3>
            <div class="grid grid-cols-2 gap-4">
              <div class="text-center p-3 rounded-lg bg-[var(--surface)]">
                <p class="text-xs text-[var(--text-muted)] mb-1">DV01</p>
                <p :class="['text-lg font-semibold', greeksResult.delta >= 0 ? 'text-[var(--success)]' : 'text-[var(--danger)]']">{{ formatCurrency(greeksResult.delta) }}</p>
              </div>
              <div v-if="greeksResult.gamma !== null" class="text-center p-3 rounded-lg bg-[var(--surface)]">
                <p class="text-xs text-[var(--text-muted)] mb-1">Gamma</p>
                <p class="text-lg font-semibold text-[var(--text-primary)]">{{ formatCurrency(greeksResult.gamma) }}</p>
              </div>
              <div v-if="greeksResult.theta !== null" class="text-center p-3 rounded-lg bg-[var(--surface)]">
                <p class="text-xs text-[var(--text-muted)] mb-1">Theta</p>
                <p :class="['text-lg font-semibold', greeksResult.theta >= 0 ? 'text-[var(--success)]' : 'text-[var(--danger)]']">{{ formatCurrency(greeksResult.theta) }}</p>
              </div>
              <div v-if="greeksResult.vega !== null" class="text-center p-3 rounded-lg bg-[var(--surface)]">
                <p class="text-xs text-[var(--text-muted)] mb-1">Vega</p>
                <p class="text-lg font-semibold text-[var(--text-primary)]">{{ formatCurrency(greeksResult.vega) }}</p>
              </div>
            </div>
          </div>

          <!-- Computation Metrics (C4) -->
          <div v-if="computationMetrics" class="glass-card p-4">
            <div class="flex items-center justify-between text-sm">
              <span class="text-[var(--text-muted)]"><i class="fas fa-clock mr-1"></i>{{ computationMetrics.pricingTimeMs.toFixed(0) }}ms</span>
              <span class="text-[var(--text-muted)]"><i class="fas fa-microchip mr-1"></i>{{ computationMetrics.method }}</span>
              <span class="text-[var(--text-muted)]"><i class="fas fa-calendar mr-1"></i>{{ new Date(computationMetrics.timestamp).toLocaleTimeString() }}</span>
            </div>
          </div>

          <!-- Result History (R9) -->
          <div v-if="recentHistory.length > 0" class="glass-card p-6">
            <div class="flex items-center justify-between mb-4">
              <h3 class="text-lg font-semibold text-[var(--text-primary)]">History</h3>
              <button
                v-if="resultHistory.length >= 2"
                class="px-3 py-1.5 text-xs rounded-lg transition-colors"
                :class="compareMode ? 'bg-[var(--primary)] text-white' : 'bg-[var(--surface)] text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]'"
                @click="compareMode = !compareMode"
              >
                <i class="fas fa-columns mr-1"></i>{{ compareMode ? 'Exit Compare' : 'Compare' }}
              </button>
            </div>

            <!-- History List -->
            <div class="space-y-2">
              <div
                v-for="entry in recentHistory"
                :key="entry.id"
                class="p-3 rounded-lg bg-[var(--surface)] cursor-pointer hover:bg-[var(--surface-hover)] transition-colors"
                @click="restoreFromHistory(entry)"
              >
                <div class="flex justify-between items-center">
                  <div>
                    <span class="text-sm font-medium text-[var(--text-primary)]">{{ entry.instrumentName }}</span>
                    <span class="text-xs text-[var(--text-muted)] ml-2">{{ new Date(entry.timestamp).toLocaleTimeString() }}</span>
                  </div>
                  <span :class="['text-sm font-semibold', (entry.pricingResult.totalPv ?? entry.pricingResult.pv ?? 0) >= 0 ? 'text-[var(--success)]' : 'text-[var(--danger)]']">
                    {{ formatCurrency(entry.pricingResult.totalPv ?? entry.pricingResult.pv ?? 0) }}
                  </span>
                </div>
              </div>
            </div>

            <!-- Compare Mode View -->
            <div v-if="compareMode && comparedResults" class="mt-4 border-t border-[var(--glass-border)] pt-4">
              <div class="grid grid-cols-2 gap-4 mb-3">
                <select v-model.number="compareIndices[0]" class="px-3 py-2 text-sm rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)]">
                  <option v-for="(e, i) in recentHistory" :key="i" :value="i">#{{ e.id }} {{ e.instrumentName }}</option>
                </select>
                <select v-model.number="compareIndices[1]" class="px-3 py-2 text-sm rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)]">
                  <option v-for="(e, i) in recentHistory" :key="i" :value="i">#{{ e.id }} {{ e.instrumentName }}</option>
                </select>
              </div>
              <!-- Side-by-side PV -->
              <div class="grid grid-cols-2 gap-4 text-center">
                <div class="p-3 rounded-lg bg-[var(--surface)]">
                  <p class="text-xs text-[var(--text-muted)] mb-1">Result A</p>
                  <p :class="['text-lg font-bold', (comparedResults.a.pricingResult.totalPv ?? 0) >= 0 ? 'text-[var(--success)]' : 'text-[var(--danger)]']">
                    {{ formatCurrency(comparedResults.a.pricingResult.totalPv ?? comparedResults.a.pricingResult.pv ?? 0) }}
                  </p>
                </div>
                <div class="p-3 rounded-lg bg-[var(--surface)]">
                  <p class="text-xs text-[var(--text-muted)] mb-1">Result B</p>
                  <p :class="['text-lg font-bold', (comparedResults.b.pricingResult.totalPv ?? 0) >= 0 ? 'text-[var(--success)]' : 'text-[var(--danger)]']">
                    {{ formatCurrency(comparedResults.b.pricingResult.totalPv ?? comparedResults.b.pricingResult.pv ?? 0) }}
                  </p>
                </div>
              </div>
              <!-- Changed Parameters -->
              <div v-if="changedParams.length > 0" class="mt-3">
                <p class="text-xs text-[var(--text-muted)] mb-2">Changed Parameters</p>
                <div v-for="cp in changedParams" :key="cp.name" class="flex justify-between text-xs py-1 border-b border-[var(--glass-border)]">
                  <span class="text-[var(--text-muted)]">{{ cp.name }}</span>
                  <span>
                    <span class="text-[var(--danger)]">{{ cp.valueA }}</span>
                    <i class="fas fa-arrow-right mx-1 text-[var(--text-muted)]"></i>
                    <span class="text-[var(--success)]">{{ cp.valueB }}</span>
                  </span>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Right Panel: Cashflows -->
        <div class="lg:col-span-2">
          <div class="glass-card p-6">
            <div class="flex items-center justify-between mb-4">
              <h3 class="text-lg font-semibold text-[var(--text-primary)]">Cashflows</h3>
              <div v-if="hasEdits" class="flex items-center gap-2">
                <span class="text-sm text-[var(--warning)] flex items-center gap-1">
                  <i class="fas fa-edit"></i> Modified
                </span>
                <button class="px-3 py-1.5 text-xs rounded-lg bg-[var(--surface)] text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]" @click="resetEdits">
                  Reset Edits
                </button>
              </div>
            </div>

            <!-- Loading Skeleton (C3) -->
            <div v-if="isExpanding" class="space-y-3">
              <div v-for="i in 6" :key="i" class="h-10 rounded-lg bg-[var(--surface)] animate-pulse"></div>
            </div>

            <!-- Empty State -->
            <div v-else-if="!expandedTrade" class="text-center py-12">
              <i class="fas fa-stream text-4xl text-[var(--text-muted)] mb-4"></i>
              <p class="text-[var(--text-muted)]">Click "Expand Cashflows" to view cashflows</p>
            </div>

            <!-- Expanded Trade -->
            <template v-else>
              <div class="mb-4 flex items-center gap-3">
                <span class="px-3 py-1 rounded-lg bg-[var(--primary)]/10 text-[var(--primary)] text-sm font-medium">
                  <i class="fas fa-hashtag mr-1"></i>{{ expandedTrade.tradeId }}
                </span>
                <span class="px-3 py-1 rounded-lg bg-[var(--surface)] text-[var(--text-secondary)] text-sm">
                  {{ expandedTrade.tradeType }}
                </span>
              </div>

              <div v-for="(leg, legIdx) in expandedTrade.legs" :key="legIdx" class="mb-6">
                <div class="flex items-center gap-2 mb-3">
                  <span class="w-6 h-6 rounded-full bg-[var(--primary)] text-white text-xs flex items-center justify-center font-medium">{{ legIdx + 1 }}</span>
                  <span :class="['px-2 py-0.5 rounded text-xs font-medium', leg.direction === 'Payer' ? 'bg-red-500/10 text-red-400' : 'bg-green-500/10 text-green-400']">{{ leg.direction }}</span>
                  <span class="px-2 py-0.5 rounded bg-[var(--surface)] text-[var(--text-secondary)] text-xs">{{ leg.currency }}</span>
                  <span class="px-2 py-0.5 rounded bg-[var(--surface)] text-[var(--text-secondary)] text-xs">{{ leg.legType }}</span>
                  <span v-if="leg.rateIndex" class="px-2 py-0.5 rounded bg-blue-500/10 text-blue-400 text-xs">{{ leg.rateIndex }}</span>
                </div>

                <div class="overflow-x-auto">
                  <table class="w-full text-sm">
                    <thead>
                      <tr class="border-b border-[var(--glass-border)]">
                        <th class="text-left py-2 px-3 text-xs font-medium text-[var(--text-muted)]">Payment</th>
                        <th class="text-left py-2 px-3 text-xs font-medium text-[var(--text-muted)]">Accrual</th>
                        <th class="text-right py-2 px-3 text-xs font-medium text-[var(--text-muted)]">YF</th>
                        <th class="text-right py-2 px-3 text-xs font-medium text-[var(--text-muted)]">Notional</th>
                        <th class="text-right py-2 px-3 text-xs font-medium text-[var(--text-muted)]">Rate</th>
                        <th class="text-center py-2 px-3 text-xs font-medium text-[var(--text-muted)]">Type</th>
                        <th class="text-right py-2 px-3 text-xs font-medium text-[var(--text-muted)]">DF</th>
                        <th class="text-right py-2 px-3 text-xs font-medium text-[var(--text-muted)]">PV</th>
                      </tr>
                    </thead>
                    <tbody>
                      <tr
                        v-for="(cf, cfIdx) in leg.cashflows"
                        :key="cfIdx"
                        :class="['border-b border-[var(--glass-border)]', editedCashflows[`${legIdx}-${cfIdx}`] ? 'bg-[var(--warning)]/5' : 'hover:bg-[var(--surface-hover)]']"
                      >
                        <td class="py-2 px-3 text-[var(--text-primary)]">{{ cf.paymentDate }}</td>
                        <td class="py-2 px-3 text-[var(--text-secondary)]">
                          {{ cf.accrualStart }} <span class="text-[var(--text-muted)]">-</span> {{ cf.accrualEnd }}
                        </td>
                        <td class="py-2 px-3 text-right text-[var(--text-secondary)] font-mono">{{ cf.yearFraction.toFixed(4) }}</td>
                        <td class="py-2 px-3 text-right">
                          <input
                            type="text"
                            :value="formatNumberCompact(editedCashflows[`${legIdx}-${cfIdx}`]?.notional ?? cf.notional)"
                            class="w-20 px-2 py-1 text-right text-sm rounded bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] font-mono"
                            @change="updateCashflow(legIdx, cfIdx, 'notional', parseFormattedNumber(($event.target as HTMLInputElement).value))"
                          >
                        </td>
                        <td class="py-2 px-3 text-right">
                          <template v-if="cf.rate !== null">
                            <input
                              type="text"
                              :value="((editedCashflows[`${legIdx}-${cfIdx}`]?.rate ?? cf.rate) * 100).toFixed(4)"
                              class="w-20 px-2 py-1 text-right text-sm rounded bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] font-mono"
                              @change="updateCashflow(legIdx, cfIdx, 'rate', parseFloat(($event.target as HTMLInputElement).value) / 100)"
                            >
                            <span class="text-[var(--text-muted)] ml-1">%</span>
                          </template>
                          <span v-else class="text-[var(--text-muted)] italic">Floating</span>
                        </td>
                        <td class="py-2 px-3 text-center">
                          <span :class="['px-2 py-0.5 rounded text-xs', cf.payoffType === 'Fixed' ? 'bg-blue-500/10 text-blue-400' : 'bg-purple-500/10 text-purple-400']">{{ cf.payoffType }}</span>
                        </td>
                        <td class="py-2 px-3 text-right font-mono text-[var(--text-secondary)]">
                          <template v-if="pricingResult?.legs?.[legIdx]?.cashflows?.[cfIdx]">
                            {{ pricingResult.legs[legIdx].cashflows[cfIdx].discountFactor.toFixed(6) }}
                          </template>
                          <span v-else class="text-[var(--text-muted)]">-</span>
                        </td>
                        <td class="py-2 px-3 text-right font-mono">
                          <template v-if="pricingResult?.legs?.[legIdx]?.cashflows?.[cfIdx]">
                            <span :class="pricingResult.legs[legIdx].cashflows[cfIdx].pv >= 0 ? 'text-[var(--success)]' : 'text-[var(--danger)]'">
                              {{ formatCurrency(pricingResult.legs[legIdx].cashflows[cfIdx].pv) }}
                            </span>
                          </template>
                          <span v-else class="text-[var(--text-muted)]">-</span>
                        </td>
                      </tr>
                    </tbody>
                  </table>
                </div>
              </div>

              <div class="flex items-center gap-4 text-sm text-[var(--text-muted)] pt-4 border-t border-[var(--glass-border)]">
                <span><i class="fas fa-layer-group mr-1"></i>{{ expandedTrade.metadata.totalLegs }} legs</span>
                <span><i class="fas fa-coins mr-1"></i>{{ expandedTrade.metadata.totalCashflows }} cashflows</span>
                <span><i class="fas fa-clock mr-1"></i>{{ expandedTrade.metadata.processingTimeMs.toFixed(2) }}ms</span>
              </div>
            </template>
          </div>
        </div>
      </div>
    </template>
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
