<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue';
import { Chart, registerables } from 'chart.js';
import { getChartColors } from '@/composables/useChartTheme';

Chart.register(...registerables);

// Types
interface CurveConfig {
  name: string;
  description?: string;
  rateIndex: string;
  instruments: string[];
  calibrationMethod: string;
  interpolation: string;
  allowExtrapolation: boolean;
}

interface CurvesData {
  metadata: {
    description: string;
    version: string;
    sections: Record<string, string>;
  };
  curves: CurveConfig[];
}

interface RateInstrument {
  type: string;
  tenor?: string;
  tenor_years?: number;
  rate?: number;
  frequency?: string;
  description?: string;
  // For event type instruments
  id?: string;
  event_date?: string;
  expected_rate_spike?: number;
  end_date?: string; // Turn events: spike reverts after this date
}

interface RateData {
  index: string;
  currency: string;
  reference_date: string;
  instruments: RateInstrument[];
}

interface DisplayInstrument {
  id: string;
  type: string;
  tenor: string;
  tenorYears: number;
  rate: number;
  enabled: boolean;
  originalRate: number;
  eventDate?: string; // For EVENT type instruments
  endDate?: string; // Turn events: spike reverts after this date
}

// instruments.json types
interface InstrumentConfig {
  id: string;
  currency: string;
  convention: string;
  tenor: string;
  rateIndex: string;
  eventDate?: string;
  expectedRateSpike?: number; // Expected rate jump for CB events (e.g., -0.0025 = -25bp)
}

interface InstrumentsData {
  metadata: Record<string, unknown>;
  templates: unknown[];
  instruments: InstrumentConfig[];
}

interface CurvePillar {
  date: string;
  time: number;
  discount_factor: number;
  zero_rate: number;
  forward_rate: number;
}

interface ForwardRatePoint {
  date: string;
  time: number;
  forward_rate: number;
}

interface ChartGridPoint {
  date: string;
  time: number;
  discount_factor: number;
  forward_rate: number;
  label: string;
}

interface JacobianData {
  row_labels: string[];
  col_labels: string[];
  matrix: number[][];
  size: number;
}

interface BuildResult {
  curve_id?: string;
  instrument_count?: number;
  interpolation?: string;
  calculation_time_ms?: number;
  pillars?: CurvePillar[];
  forward_curve?: ForwardRatePoint[];
  short_term_grid?: ChartGridPoint[];
  long_term_grid?: ChartGridPoint[];
  converged?: boolean;
  bootstrap_method?: string;
  jacobian?: JacobianData;
}

// State
const curvesConfig = ref<CurvesData | null>(null);
const instrumentsConfig = ref<InstrumentsData | null>(null);
const selectedCurveName = ref<string>('');
const selectedCurve = ref<CurveConfig | null>(null);
const rateData = ref<RateData | null>(null);
const instruments = ref<DisplayInstrument[]>([]);
const buildResult = ref<BuildResult | null>(null);
const isLoading = ref(false);
const isBuilding = ref(false);
const loadError = ref<string | null>(null);
const buildError = ref<string | null>(null);

// Build settings (editable)
const calibrationMethod = ref<string>('bootstrapping');
const interpolation = ref<string>('log_linear_df');
const allowExtrapolation = ref<boolean>(true);

// Last-built settings — used to detect "rebuild required"
const lastBuiltSettings = ref<{
  calibrationMethod: string;
  interpolation: string;
  allowExtrapolation: boolean;
} | null>(null);

// Chart
const shortTermChartCanvas = ref<HTMLCanvasElement | null>(null);
const longTermChartCanvas = ref<HTMLCanvasElement | null>(null);
let shortTermChartInstance: Chart | null = null;
let longTermChartInstance: Chart | null = null;
const chartType = ref<'discount_factor' | 'forward_rate'>('forward_rate');

// Normalise interpolation values from curves.json (legacy) to backend snake_case format
function normaliseInterpolation(value: string): string {
  const map: Record<string, string> = {
    'loglinear': 'log_linear_df',
    'log_linear': 'log_linear_df',
    'linear': 'linear_df',
    'monotone_cubic': 'log_linear_df',
    'cubic': 'log_linear_df',
    'cubic_spline': 'log_linear_df',
  };
  return map[value] || value;
}

// Normalise calibration method values (legacy "sequential" → "bootstrapping")
function normaliseCalibrationMethod(value: string): string {
  if (value === 'sequential') return 'bootstrapping';
  return value;
}

// Options for build settings
const calibrationMethods = [
  { value: 'bootstrapping', label: 'Bootstrapping' },
  { value: 'global', label: 'Global' },
];
const interpolationMethods = [
  { value: 'flat_forward', label: 'Flat Forward' },
  { value: 'log_linear_df', label: 'Log-Linear DF' },
  { value: 'linear_df', label: 'Linear DF' },
];

// Computed
const curveOptions = computed(() => {
  if (!curvesConfig.value) return [];
  return curvesConfig.value.curves.map(c => ({
    name: c.name,
    rateIndex: c.rateIndex,
  }));
});

const enabledInstruments = computed(() =>
  instruments.value.filter(inst => inst.enabled)
);

const hasChanges = computed(() => {
  if (!buildResult.value) return false; // never built yet — nothing to rebuild

  // Check if any rate changed since last build
  const rateChanged = instruments.value.some(inst => inst.rate !== inst.originalRate);

  // Check if build settings changed since last build
  const ref = lastBuiltSettings.value;
  const settingsChanged = ref != null && (
    calibrationMethod.value !== ref.calibrationMethod ||
    interpolation.value !== ref.interpolation ||
    allowExtrapolation.value !== ref.allowExtrapolation
  );

  return rateChanged || settingsChanged;
});

const summaryStats = computed(() => {
  const eventCount = enabledInstruments.value.filter(i => i.type === 'event').length;

  return [
    { label: 'Valuation Date', value: rateData.value?.reference_date || '-', icon: 'fa-calendar', color: '#8b5cf6' },
    { label: 'Instruments', value: `${enabledInstruments.value.length}/${instruments.value.length}${eventCount > 0 ? ` (${eventCount} events)` : ''}`, icon: 'fa-list-alt', color: '#3b82f6' },
    { label: 'Interpolation', value: interpolationMethods.find(m => m.value === interpolation.value)?.label ?? interpolation.value, icon: 'fa-wave-square', color: '#10b981' },
    { label: 'Status', value: buildResult.value ? 'Built' : 'Pending', icon: 'fa-info-circle', color: buildResult.value ? '#10b981' : '#f59e0b' },
  ];
});

// Curve data table — merge short + long term grids, deduplicate by date
const curveTableRows = computed(() => {
  if (!buildResult.value) return [];
  const shortGrid = buildResult.value.short_term_grid || [];
  const longGrid = buildResult.value.long_term_grid || [];

  const seen = new Set<string>();
  const rows: { date: string; time: number; df: number; fwd: number }[] = [];
  for (const pt of [...shortGrid, ...longGrid]) {
    if (!seen.has(pt.date)) {
      seen.add(pt.date);
      rows.push({ date: pt.date, time: pt.time, df: pt.discount_factor, fwd: pt.forward_rate });
    }
  }
  rows.sort((a, b) => a.time - b.time);
  return rows;
});

// Jacobian heatmap helpers
const jacobianAbsMax = computed(() => {
  if (!buildResult.value?.jacobian) return 1;
  const vals = buildResult.value.jacobian.matrix.flat().filter(v => v !== 0);
  if (vals.length === 0) return 1;
  return Math.max(...vals.map(Math.abs));
});

function jacobianHeatmapColour(value: number): string {
  const max = jacobianAbsMax.value;
  if (max === 0 || value === 0) return 'transparent';
  const t = Math.min(Math.abs(value) / max, 1);
  if (value < 0) {
    return `rgba(239, 68, 68, ${0.08 + t * 0.35})`;
  }
  return `rgba(59, 130, 246, ${0.08 + t * 0.35})`;
}

function jacobianTextColour(value: number): string {
  const max = jacobianAbsMax.value;
  if (max === 0 || value === 0) return 'var(--text-muted)';
  const t = Math.min(Math.abs(value) / max, 1);
  if (t > 0.4) return value < 0 ? '#f87171' : '#60a5fa';
  return 'var(--text-secondary)';
}

// Chart options
function createChartOptions(yAxisLabel: string) {
  const cc = getChartColors();
  return {
    responsive: true,
    maintainAspectRatio: false,
    plugins: {
      legend: { display: false },
      tooltip: {
        backgroundColor: cc.tooltipBg,
        titleColor: cc.tooltipTitle,
        bodyColor: cc.tooltipBody,
        callbacks: {
          title: (items: { label: string }[]) => items[0].label,
          label: (item: { raw: unknown }) => {
            const value = item.raw as number;
            if (chartType.value === 'discount_factor') {
              return `DF: ${value.toFixed(6)}`;
            } else {
              return `Forward Rate: ${value.toFixed(4)}%`;
            }
          },
        },
      },
    },
    scales: {
      x: {
        ticks: { color: cc.tick, maxTicksLimit: 12 },
        grid: { color: cc.grid },
      },
      y: {
        title: { display: true, text: yAxisLabel, color: cc.tick },
        ticks: { color: cc.tick },
        grid: { color: cc.grid },
      },
    },
  };
}

// Milestone definitions for term tick labels
const SHORT_MILESTONES = [
  { time: 7 / 365, term: '1W' }, { time: 14 / 365, term: '2W' },
  { time: 1 / 12, term: '1M' }, { time: 2 / 12, term: '2M' }, { time: 3 / 12, term: '3M' },
  { time: 6 / 12, term: '6M' }, { time: 9 / 12, term: '9M' }, { time: 1.0, term: '1Y' },
];
const LONG_MILESTONES = [
  { time: 1, term: '1Y' }, { time: 2, term: '2Y' }, { time: 3, term: '3Y' },
  { time: 5, term: '5Y' }, { time: 7, term: '7Y' }, { time: 10, term: '10Y' },
  { time: 15, term: '15Y' }, { time: 20, term: '20Y' }, { time: 25, term: '25Y' },
  { time: 30, term: '30Y' },
];

// Render chart from pre-computed backend grid with milestone term labels
function renderChart(
  canvas: HTMLCanvasElement | null,
  existing: Chart | null,
  grid: ChartGridPoint[],
  label: string,
  color: string,
  milestones: { time: number; term: string }[],
): Chart | null {
  if (!canvas || grid.length === 0) return existing;
  if (existing) existing.destroy();

  const labels = grid.map(pt => pt.label);
  const data = chartType.value === 'forward_rate'
    ? grid.map(pt => pt.forward_rate * 100)
    : grid.map(pt => pt.discount_factor);

  // Compute milestone index → [dateLabel, term]
  const milestoneAt = new Map<number, string[]>();
  for (const ms of milestones) {
    let bestIdx = 0;
    let bestDist = Infinity;
    for (let i = 0; i < grid.length; i++) {
      const dist = Math.abs(grid[i].time - ms.time);
      if (dist < bestDist) { bestDist = dist; bestIdx = i; }
    }
    milestoneAt.set(bestIdx, [grid[bestIdx].label, ms.term]);
  }

  const opts = createChartOptions(label);
  const cc = getChartColors();
  (opts.scales.x as Record<string, unknown>).ticks = {
    autoSkip: false,
    maxRotation: 0,
    color: cc.tick,
    callback: (_value: unknown, index: number) => milestoneAt.get(index) ?? null,
  };

  const ctx = canvas.getContext('2d');
  if (!ctx) return null;

  // Flat Forward: use stepped line for forward rate charts to show flat segments
  const isFlatFwd = interpolation.value === 'flat_forward' && chartType.value === 'forward_rate';

  return new Chart(ctx, {
    type: 'line',
    data: {
      labels,
      datasets: [{
        label,
        data,
        borderColor: color,
        backgroundColor: `${color}1a`,
        borderWidth: 2,
        fill: true,
        tension: isFlatFwd ? 0 : 0.3,
        stepped: isFlatFwd ? 'before' : false,
        pointRadius: 1,
        pointBackgroundColor: color,
      }],
    },
    options: opts,
  });
}

// Chart update — reads pre-computed grids from backend response
function updateCharts() {
  if (!buildResult.value) return;

  const shortGrid = buildResult.value.short_term_grid || [];
  const longGrid = buildResult.value.long_term_grid || [];

  const chartLabels: Record<string, string> = {
    discount_factor: 'Discount Factor',
    forward_rate: 'Forward Rate (%)',
  };
  const chartColors: Record<string, string> = {
    discount_factor: '#6366f1',
    forward_rate: '#10b981',
  };

  const currentLabel = chartLabels[chartType.value];
  const currentColor = chartColors[chartType.value];

  shortTermChartInstance = renderChart(
    shortTermChartCanvas.value, shortTermChartInstance, shortGrid, currentLabel, currentColor, SHORT_MILESTONES,
  );
  longTermChartInstance = renderChart(
    longTermChartCanvas.value, longTermChartInstance, longGrid, currentLabel, currentColor, LONG_MILESTONES,
  );
}

// API calls
async function loadCurvesConfig() {
  loadError.value = null;
  try {
    const response = await fetch('/data/config/curves.json');
    if (!response.ok) throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    curvesConfig.value = await response.json();
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Unknown error';
    console.error('Failed to load curves config:', message);
    loadError.value = `Failed to load curves: ${message}`;
  }
}

async function loadInstrumentsConfig() {
  try {
    const response = await fetch('/data/config/instruments.json');
    if (!response.ok) throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    instrumentsConfig.value = await response.json();
  } catch (error) {
    console.error('Failed to load instruments config:', error);
  }
}

async function loadRateData(rateIndex: string) {
  try {
    // Convert rate index to file name (e.g., "USD-SOFR" -> "usd-sofr")
    const fileName = rateIndex.toLowerCase().replace('_', '-');
    const response = await fetch(`/data/input/rates/${fileName}.json`);
    if (!response.ok) throw new Error(`Failed to load rate data for ${rateIndex}`);
    rateData.value = await response.json();
  } catch (error) {
    console.error('Failed to load rate data:', error);
    rateData.value = null;
  }
}

function buildInstrumentId(type: string, tenor: string, currency: string): string {
  const typeMap: Record<string, string> = {
    'deposit': 'Depo',
    'ois': 'OIS',
    'fra': 'FRA',
    'future': 'Future',
    'swap': 'Swap',
  };
  const typeLabel = typeMap[type] || type.toUpperCase();
  // Normalize tenor: "O/N" -> "ON" to match curve config format
  const normalizedTenor = tenor === 'O/N' ? 'ON' : tenor;
  return `${currency}-${typeLabel}-${normalizedTenor}`;
}

function loadInstrumentsForCurve() {
  if (!selectedCurve.value || !rateData.value) {
    instruments.value = [];
    return;
  }

  const currency = rateData.value.currency;
  const referenceDate = new Date(rateData.value.reference_date);

  // Get the set of instrument IDs that should be enabled by default (from curve config)
  const defaultEnabledIds = new Set(selectedCurve.value.instruments || []);

  // Build display instruments from rate data
  const displayInstruments: DisplayInstrument[] = [];

  for (const rateInst of rateData.value.instruments) {
    // Handle event type instruments from rate input file
    if (rateInst.type === 'event') {
      const eventDate = new Date(rateInst.event_date || '');

      // Skip past events
      if (eventDate < referenceDate) continue;

      // Approximate tenor for display sorting only (not used for pricing)
      const tenorYears = (eventDate.getTime() - referenceDate.getTime()) / (365.25 * 86_400_000);

      const id = rateInst.id || '';
      // Only include if in curve definition
      if (!defaultEnabledIds.has(id)) continue;

      displayInstruments.push({
        id,
        type: 'event',
        tenor: 'EVENT',
        tenorYears,
        rate: rateInst.expected_rate_spike || 0,
        originalRate: rateInst.expected_rate_spike || 0,
        enabled: true,
        eventDate: rateInst.event_date,
        endDate: rateInst.end_date,
      });
    } else {
      // Handle regular instruments (deposit, ois, fra, etc.)
      const tenor = rateInst.tenor || '';
      const id = buildInstrumentId(rateInst.type, tenor, currency);

      displayInstruments.push({
        id,
        type: rateInst.type,
        tenor,
        tenorYears: rateInst.tenor_years || 0,
        rate: rateInst.rate || 0,
        originalRate: rateInst.rate || 0,
        enabled: defaultEnabledIds.has(id),
      });
    }
  }

  // Sort by tenor years
  displayInstruments.sort((a, b) => a.tenorYears - b.tenorYears);

  instruments.value = displayInstruments;
}

async function onCurveSelected() {
  if (!selectedCurveName.value || !curvesConfig.value) {
    selectedCurve.value = null;
    instruments.value = [];
    buildResult.value = null;
    return;
  }

  isLoading.value = true;

  try {
    // Find selected curve config
    const curve = curvesConfig.value.curves.find(c => c.name === selectedCurveName.value);
    if (!curve) return;

    selectedCurve.value = curve;

    // Set build settings from curve config
    calibrationMethod.value = normaliseCalibrationMethod(curve.calibrationMethod);
    interpolation.value = normaliseInterpolation(curve.interpolation);
    allowExtrapolation.value = curve.allowExtrapolation;

    // Load rate data for this curve's rate index
    await loadRateData(curve.rateIndex);

    // Build instruments list
    loadInstrumentsForCurve();

    // Clear previous build result
    buildResult.value = null;
  } finally {
    isLoading.value = false;
  }
}

async function buildCurve() {
  if (!selectedCurve.value || enabledInstruments.value.length === 0) return;

  isBuilding.value = true;
  buildError.value = null;
  try {
    // Build instrument payload including events
    const instrumentPayload = enabledInstruments.value.map(inst => {
      if (inst.type === 'event') {
        const payload: Record<string, unknown> = {
          instrument_type: 'event',
          tenor: '',
          rate: 0,
          event_date: inst.eventDate,
          expected_rate_spike: inst.rate, // rate field stores the spike for events
        };
        // Turn events: include end_date so the spike reverts
        if (inst.endDate) {
          payload.end_date = inst.endDate;
        }
        return payload;
      } else {
        return {
          instrument_type: inst.type.toLowerCase(),
          tenor: inst.tenor,
          rate: inst.rate,
        };
      }
    });

    const response = await fetch('/api/curves/build', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        index: selectedCurve.value.rateIndex,
        currency: rateData.value?.currency || 'USD',
        reference_date: rateData.value?.reference_date,
        instruments: instrumentPayload,
        bootstrap_method: calibrationMethod.value,
        interpolation: interpolation.value,
      }),
    });

    if (!response.ok) {
      const errorData = await response.json();
      throw new Error(errorData.error || errorData.message || 'Build failed');
    }

    buildResult.value = await response.json();

    // Snapshot current state as "last built"
    instruments.value.forEach(inst => {
      inst.originalRate = inst.rate;
    });
    lastBuiltSettings.value = {
      calibrationMethod: calibrationMethod.value,
      interpolation: interpolation.value,
      allowExtrapolation: allowExtrapolation.value,
    };

    // Update charts
    await nextTick();
    updateCharts();
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Unknown error';
    console.error('Build failed:', message);
    buildError.value = message;
  } finally {
    isBuilding.value = false;
  }
}

function resetSettings() {
  if (!selectedCurve.value) return;

  // Reset build settings
  calibrationMethod.value = normaliseCalibrationMethod(selectedCurve.value.calibrationMethod);
  interpolation.value = normaliseInterpolation(selectedCurve.value.interpolation);
  allowExtrapolation.value = selectedCurve.value.allowExtrapolation;

  // Reset rates
  instruments.value.forEach(inst => {
    inst.rate = inst.originalRate;
  });
}

function exportRates() {
  if (instruments.value.length === 0) return;

  const csv = [
    'ID,Type,Tenor,Rate(%),EventDate,Enabled',
    ...instruments.value.map(
      inst => `${inst.id},${inst.type},${inst.tenor},${inst.type === 'event' ? '' : (inst.rate * 100).toFixed(4)},${inst.eventDate || ''},${inst.enabled}`
    ),
  ].join('\n');

  const blob = new Blob([csv], { type: 'text/csv' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = `curve_instruments_${selectedCurveName.value || 'unknown'}.csv`;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

function updateRate(index: number, value: string) {
  instruments.value[index].rate = parseFloat(value) / 100;
}

function updateSpike(index: number, value: string) {
  // Convert basis points to decimal (e.g., -25bp = -0.0025)
  instruments.value[index].rate = parseFloat(value) / 10000;
}

function toggleEnabled(index: number) {
  instruments.value[index].enabled = !instruments.value[index].enabled;
}

function toggleAll(enabled: boolean) {
  instruments.value.forEach(inst => inst.enabled = enabled);
}

// Watch for curve selection change
watch(selectedCurveName, () => {
  onCurveSelected();
});

// Watch for chart type change
watch(chartType, () => {
  if (buildResult.value?.short_term_grid) {
    updateCharts();
  }
});

// Lifecycle
onMounted(async () => {
  await Promise.all([loadCurvesConfig(), loadInstrumentsConfig()]);
  // Set default selection to USD-SOFR
  if (curvesConfig.value?.curves.some(c => c.name === 'USD-SOFR')) {
    selectedCurveName.value = 'USD-SOFR';
  }
});

onUnmounted(() => {
  if (shortTermChartInstance) {
    shortTermChartInstance.destroy();
  }
  if (longTermChartInstance) {
    longTermChartInstance.destroy();
  }
});
</script>

<template>
  <div class="curve-builder-view">
    <!-- Summary Stats -->
    <div class="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
      <div
        v-for="stat in summaryStats"
        :key="stat.label"
        class="glass-card p-4"
      >
        <div class="flex items-start justify-between">
          <div>
            <p class="text-sm text-[var(--text-muted)] mb-1">{{ stat.label }}</p>
            <p class="text-xl font-semibold text-[var(--text-primary)]">{{ stat.value }}</p>
          </div>
          <div
            class="w-9 h-9 rounded-lg flex items-center justify-center"
            :style="{ backgroundColor: `${stat.color}1a` }"
          >
            <i :class="['fas', stat.icon, 'text-sm']" :style="{ color: stat.color }"></i>
          </div>
        </div>
      </div>
    </div>

    <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <!-- Left Panel: Settings -->
      <div class="space-y-4">
        <!-- Curve Selector -->
        <div class="glass-card p-5">
          <h3 class="text-base font-semibold text-[var(--text-primary)] mb-3">Curve Selection</h3>

          <!-- Error Message -->
          <div v-if="loadError" class="mb-3 p-2 rounded bg-red-500/20 border border-red-500/50">
            <p class="text-xs text-red-400">{{ loadError }}</p>
          </div>

          <select
            v-model="selectedCurveName"
            :disabled="!curvesConfig"
            class="w-full px-3 py-2 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-[var(--primary)] disabled:opacity-50"
          >
            <option value="">{{ curvesConfig ? 'Select curve...' : 'Loading...' }}</option>
            <option v-for="curve in curveOptions" :key="curve.name" :value="curve.name">
              {{ curve.name }}
            </option>
          </select>

        </div>

        <!-- Instruments Table (Compact) -->
        <div class="glass-card p-5">
          <div class="flex items-center justify-between mb-3">
            <h3 class="text-base font-semibold text-[var(--text-primary)]">Instruments</h3>
            <div v-if="instruments.length > 0" class="flex gap-1">
              <button
                class="px-2 py-1 text-xs rounded bg-[var(--surface)] text-[var(--text-muted)] hover:bg-[var(--surface-hover)]"
                @click="toggleAll(true)"
              >All</button>
              <button
                class="px-2 py-1 text-xs rounded bg-[var(--surface)] text-[var(--text-muted)] hover:bg-[var(--surface-hover)]"
                @click="toggleAll(false)"
              >None</button>
            </div>
          </div>

          <div v-if="isLoading" class="text-center py-8">
            <i class="fas fa-spinner fa-spin text-[var(--primary)]"></i>
          </div>

          <div v-else-if="instruments.length === 0" class="text-center py-8 text-[var(--text-muted)] text-sm">
            Select a curve
          </div>

          <div v-else class="max-h-64 overflow-y-auto space-y-1">
            <div
              v-for="(inst, idx) in instruments"
              :key="inst.id"
              :class="[
                'flex items-center gap-2 px-2 py-1.5 rounded text-sm',
                inst.enabled ? 'bg-[var(--surface)]' : 'opacity-40',
                inst.type === 'event' ? 'border-l-2 border-amber-500' : ''
              ]"
            >
              <input
                type="checkbox"
                :checked="inst.enabled"
                class="w-3.5 h-3.5 rounded border-[var(--glass-border)]"
                @change="toggleEnabled(idx)"
              >
              <span class="flex-1 font-mono text-xs text-[var(--text-secondary)] truncate" :title="inst.id">{{ inst.id }}</span>
              <!-- Event instruments show date and expected spike input -->
              <template v-if="inst.type === 'event'">
                <span
                  v-if="inst.endDate"
                  class="px-1 py-0.5 text-[10px] rounded bg-cyan-500/20 text-cyan-400"
                  title="Turn event (temporary spike)"
                >TURN</span>
                <span
                  v-else
                  class="px-1 py-0.5 text-[10px] rounded bg-amber-500/20 text-amber-400"
                  title="Jump event (permanent shift)"
                >JUMP</span>
                <span
                  class="px-1.5 py-0.5 text-xs rounded bg-amber-500/20 text-amber-400 font-mono"
                  :title="inst.endDate ? `Turn: ${inst.eventDate} → ${inst.endDate}` : 'Event Date'"
                >{{ inst.eventDate }}</span>
                <input
                  type="number"
                  :value="(inst.rate * 10000).toFixed(1)"
                  step="0.5"
                  class="w-14 px-1.5 py-0.5 text-right text-xs rounded bg-amber-500/10 border border-amber-500/30 text-amber-400"
                  :title="inst.endDate ? `Turn spike in bp (reverts ${inst.endDate})` : 'Expected rate spike in basis points'"
                  @change="updateSpike(idx, ($event.target as HTMLInputElement).value)"
                >
                <span class="text-xs text-amber-400/60">bp</span>
              </template>
              <!-- Regular instruments show rate input -->
              <input
                v-else
                type="number"
                :value="(inst.rate * 100).toFixed(2)"
                step="0.01"
                class="w-16 px-1.5 py-0.5 text-right text-xs rounded bg-[var(--glass-bg)] border border-[var(--glass-border)] text-[var(--text-primary)]"
                @change="updateRate(idx, ($event.target as HTMLInputElement).value)"
              >
            </div>
          </div>
        </div>

        <!-- Build Settings -->
        <div class="glass-card p-5">
          <h3 class="text-base font-semibold text-[var(--text-primary)] mb-3">Build Settings</h3>
          <div class="space-y-3">
            <div>
              <label class="block text-xs text-[var(--text-muted)] mb-1">Calibration</label>
              <select
                v-model="calibrationMethod"
                class="w-full px-2 py-1.5 rounded bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm"
              >
                <option v-for="m in calibrationMethods" :key="m.value" :value="m.value">{{ m.label }}</option>
              </select>
            </div>
            <div>
              <label class="block text-xs text-[var(--text-muted)] mb-1">Interpolation</label>
              <select
                v-model="interpolation"
                class="w-full px-2 py-1.5 rounded bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm"
              >
                <option v-for="m in interpolationMethods" :key="m.value" :value="m.value">{{ m.label }}</option>
              </select>
            </div>
            <label class="flex items-center gap-2 cursor-pointer">
              <input v-model="allowExtrapolation" type="checkbox" class="w-4 h-4 rounded">
              <span class="text-sm text-[var(--text-secondary)]">Extrapolation</span>
            </label>
          </div>
        </div>

        <!-- Actions -->
        <div class="glass-card p-5">
          <button
            :disabled="!selectedCurve || enabledInstruments.length === 0 || isBuilding"
            class="w-full px-4 py-2.5 rounded-lg bg-[var(--primary)] text-white font-medium hover:opacity-90 disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
            @click="buildCurve"
          >
            <i :class="['fas', isBuilding ? 'fa-spinner fa-spin' : 'fa-hammer']"></i>
            {{ isBuilding ? 'Building...' : 'Build Curve' }}
          </button>
          <div class="grid grid-cols-2 gap-2 mt-2">
            <button
              :disabled="!hasChanges"
              class="px-3 py-1.5 rounded bg-[var(--surface)] text-[var(--text-secondary)] text-sm hover:bg-[var(--surface-hover)] disabled:opacity-50"
              @click="resetSettings"
            >
              <i class="fas fa-undo mr-1"></i>Reset
            </button>
            <button
              :disabled="instruments.length === 0"
              class="px-3 py-1.5 rounded bg-[var(--surface)] text-[var(--text-secondary)] text-sm hover:bg-[var(--surface-hover)] disabled:opacity-50"
              @click="exportRates"
            >
              <i class="fas fa-download mr-1"></i>Export
            </button>
          </div>

          <div v-if="hasChanges" class="mt-3 p-2 rounded bg-[#f59e0b1a] border border-[var(--warning)]">
            <p class="text-xs text-[var(--warning)] flex items-center gap-1">
              <i class="fas fa-exclamation-triangle"></i>
              Rebuild required
            </p>
          </div>
        </div>
      </div>

      <!-- Right Panel: Curve Chart + Jacobian -->
      <div class="lg:col-span-2 space-y-6">
        <div class="glass-card p-6">
          <div class="flex items-center justify-between mb-4">
            <h3 class="text-lg font-semibold text-[var(--text-primary)]">Yield Curve</h3>
            <div v-if="buildResult?.short_term_grid" class="flex gap-2">
              <button
                :class="[
                  'px-3 py-1.5 text-xs rounded-lg transition-colors',
                  chartType === 'forward_rate' ? 'bg-emerald-500 text-white' : 'bg-[var(--surface)] text-[var(--text-secondary)]'
                ]"
                @click="chartType = 'forward_rate'"
              >
                Forward Rate
              </button>
              <button
                :class="[
                  'px-3 py-1.5 text-xs rounded-lg transition-colors',
                  chartType === 'discount_factor' ? 'bg-[var(--primary)] text-white' : 'bg-[var(--surface)] text-[var(--text-secondary)]'
                ]"
                @click="chartType = 'discount_factor'"
              >
                Discount Factor
              </button>
            </div>
          </div>

          <!-- Build Error -->
          <div v-if="buildError" class="mb-4 p-3 rounded-lg bg-red-500/20 border border-red-500/50">
            <p class="text-sm text-red-400 flex items-center gap-2">
              <i class="fas fa-exclamation-circle"></i>
              {{ buildError }}
            </p>
          </div>

          <!-- Empty State -->
          <div v-if="!buildResult && !buildError" class="flex flex-col items-center justify-center h-[500px] text-[var(--text-muted)]">
            <i class="fas fa-chart-line text-5xl mb-4 opacity-30"></i>
            <p class="text-sm">Build a curve to see the chart</p>
          </div>

          <!-- Charts: Short-term (top) and Long-term (bottom) -->
          <div v-else class="space-y-4">
            <div>
              <h4 class="text-sm font-medium text-[var(--text-secondary)] mb-2">
                <i class="fas fa-clock text-xs mr-1"></i>Short Term (0-1Y)
              </h4>
              <div class="h-48 bg-[var(--surface)] rounded-lg p-2">
                <canvas ref="shortTermChartCanvas"></canvas>
              </div>
            </div>

            <div>
              <h4 class="text-sm font-medium text-[var(--text-secondary)] mb-2">
                <i class="fas fa-calendar-alt text-xs mr-1"></i>Long Term (0-30Y)
              </h4>
              <div class="h-48 bg-[var(--surface)] rounded-lg p-2">
                <canvas ref="longTermChartCanvas"></canvas>
              </div>
            </div>
          </div>

          <!-- Build Info -->
          <div v-if="buildResult" class="mt-4 pt-4 border-t border-[var(--glass-border)]">
            <div class="grid grid-cols-4 gap-4 text-sm">
              <div>
                <span class="text-[var(--text-muted)]">Instruments:</span>
                <span class="ml-2 text-[var(--text-primary)] font-medium">{{ buildResult.instrument_count }}</span>
              </div>
              <div>
                <span class="text-[var(--text-muted)]">Method:</span>
                <span class="ml-2 text-[var(--text-primary)] font-medium">{{ calibrationMethods.find(m => m.value === (buildResult?.bootstrap_method ?? calibrationMethod))?.label ?? buildResult?.bootstrap_method }}</span>
              </div>
              <div>
                <span class="text-[var(--text-muted)]">Interpolation:</span>
                <span class="ml-2 text-[var(--text-primary)] font-medium">{{ buildResult.interpolation }}</span>
              </div>
              <div>
                <span class="text-[var(--text-muted)]">Time:</span>
                <span class="ml-2 text-[var(--text-primary)] font-medium">{{ buildResult.calculation_time_ms?.toFixed(2) }} ms</span>
              </div>
            </div>
          </div>

          <!-- Pillar Data Table -->
          <div v-if="curveTableRows.length > 0" class="mt-4 pt-4 border-t border-[var(--glass-border)]">
            <h4 class="text-sm font-medium text-[var(--text-secondary)] mb-3">
              <i class="fas fa-table text-xs mr-1"></i>
              Curve Data ({{ curveTableRows.length }} points)
            </h4>
            <div class="max-h-64 overflow-y-auto">
              <table class="w-full text-sm">
                <thead class="sticky top-0 z-10">
                  <tr class="border-b border-[var(--glass-border)] curve-table-header">
                    <th class="text-left py-2 px-2 text-xs font-medium text-[var(--text-muted)]">Date</th>
                    <th class="text-right py-2 px-2 text-xs font-medium text-[var(--text-muted)]">Time (Y)</th>
                    <th class="text-right py-2 px-2 text-xs font-medium text-[var(--text-muted)]">Fwd Rate (%)</th>
                    <th class="text-right py-2 px-2 text-xs font-medium text-[var(--text-muted)]">DF</th>
                  </tr>
                </thead>
                <tbody>
                  <tr
                    v-for="(row, idx) in curveTableRows"
                    :key="idx"
                    class="border-b border-[var(--glass-border)] hover:bg-[var(--surface-hover)] transition-colors"
                  >
                    <td class="py-1.5 px-2 text-xs text-[var(--text-primary)] font-mono">{{ row.date }}</td>
                    <td class="py-1.5 px-2 text-xs text-right text-[var(--text-secondary)] font-mono">{{ row.time.toFixed(4) }}</td>
                    <td class="py-1.5 px-2 text-xs text-right font-mono text-emerald-400">{{ (row.fwd * 100).toFixed(4) }}</td>
                    <td class="py-1.5 px-2 text-xs text-right font-mono text-[var(--text-primary)]">{{ row.df.toFixed(8) }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>

        <!-- Jacobian Card (below Yield Curve, same width) -->
        <div v-if="buildResult?.jacobian" class="glass-card p-6">
          <div class="flex items-center justify-between mb-4">
            <h3 class="text-lg font-semibold text-[var(--text-primary)]">
              <i class="fas fa-th text-sm mr-2 text-[var(--primary)]"></i>
              Jacobian <span class="text-sm font-normal text-[var(--text-muted)]">d(log DF)/T / dr &approx; &minus;dz/dr</span>
            </h3>
            <span class="text-xs text-[var(--text-muted)] font-mono">
              {{ buildResult.jacobian.size }} &times; {{ buildResult.jacobian.size }}
            </span>
          </div>

          <div class="overflow-x-auto">
            <table class="w-full border-collapse">
              <thead>
                <tr>
                  <th class="sticky left-0 z-10 py-2 px-3 text-xs font-medium text-[var(--text-muted)] jacobian-sticky-cell border-b border-r border-[var(--glass-border)] text-left">
                    &minus;dz \ Rate
                  </th>
                  <th
                    v-for="label in buildResult.jacobian.col_labels"
                    :key="'jh-' + label"
                    class="py-2 px-2 text-xs font-medium text-[var(--text-muted)] text-center border-b border-[var(--glass-border)] whitespace-nowrap"
                  >
                    {{ label }}
                  </th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="(row, i) in buildResult.jacobian.matrix"
                  :key="'jr-' + i"
                  class="hover:bg-[var(--surface-hover)] transition-colors"
                >
                  <td class="sticky left-0 z-10 py-1.5 px-3 text-xs font-medium text-[var(--text-muted)] jacobian-sticky-cell border-r border-b border-[var(--glass-border)] whitespace-nowrap">
                    {{ buildResult.jacobian.row_labels[i] }}
                  </td>
                  <td
                    v-for="(val, j) in row"
                    :key="'jc-' + i + '-' + j"
                    class="py-1.5 px-1 text-center text-xs font-mono border-b border-[var(--glass-border)]"
                    :style="{ backgroundColor: jacobianHeatmapColour(val) }"
                  >
                    <span :style="{ color: jacobianTextColour(val) }">
                      {{ val === 0 ? '--' : val.toPrecision(2) }}
                    </span>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <p class="mt-3 text-xs text-[var(--text-muted)]">
            <i class="fas fa-info-circle mr-1"></i>
            Normalised by T<sub>i</sub>: diagonal &approx; &minus;1 (zero rate moves 1:1 with market rate). Lower-triangular in bootstrapping.
          </p>
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

.curve-table-header {
  background: var(--surface);
  box-shadow: 0 1px 0 var(--glass-border);
}

.curve-table-header th {
  background: inherit;
}

.jacobian-sticky-cell {
  background: var(--glass-bg);
}
</style>
