<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue';
import { Chart, registerables } from 'chart.js';

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
  time: number;
  discount_factor: number;
  zero_rate: number;
}

interface ForwardRatePoint {
  time: number;
  forward_rate: number;
}

interface BuildResult {
  curve_id?: string;
  instrument_count?: number;
  interpolation?: string;
  calculation_time_ms?: number;
  pillars?: CurvePillar[];
  forward_curve?: ForwardRatePoint[];
  converged?: boolean;
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
const calibrationMethod = ref<string>('sequential');
const interpolation = ref<string>('loglinear');
const allowExtrapolation = ref<boolean>(true);

// Chart
const shortTermChartCanvas = ref<HTMLCanvasElement | null>(null);
const longTermChartCanvas = ref<HTMLCanvasElement | null>(null);
let shortTermChartInstance: Chart | null = null;
let longTermChartInstance: Chart | null = null;
const chartType = ref<'zero_rate' | 'discount_factor' | 'forward_rate'>('forward_rate');

// Options for build settings
const calibrationMethods = ['sequential', 'global', 'bootstrap'];
const interpolationMethods = ['linear', 'loglinear', 'cubic', 'monotone_cubic', 'flat_forward'];

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
  if (!selectedCurve.value) return false;

  // Check if any rate changed
  const rateChanged = instruments.value.some(inst => inst.rate !== inst.originalRate);

  // Check if build settings changed
  const settingsChanged =
    calibrationMethod.value !== selectedCurve.value.calibrationMethod ||
    interpolation.value !== selectedCurve.value.interpolation ||
    allowExtrapolation.value !== selectedCurve.value.allowExtrapolation;

  return rateChanged || settingsChanged;
});

const summaryStats = computed(() => {
  const eventCount = enabledInstruments.value.filter(i => i.type === 'event').length;

  return [
    { label: 'Valuation Date', value: rateData.value?.reference_date || '-', icon: 'fa-calendar', color: '#8b5cf6' },
    { label: 'Instruments', value: `${enabledInstruments.value.length}/${instruments.value.length}${eventCount > 0 ? ` (${eventCount} events)` : ''}`, icon: 'fa-list-alt', color: '#3b82f6' },
    { label: 'Interpolation', value: interpolation.value, icon: 'fa-wave-square', color: '#10b981' },
    { label: 'Status', value: buildResult.value ? 'Built' : 'Pending', icon: 'fa-info-circle', color: buildResult.value ? '#10b981' : '#f59e0b' },
  ];
});

// Chart helper functions
function interpolateValue(time: number, pillars: CurvePillar[], type: 'zero_rate' | 'discount_factor'): number {
  if (pillars.length === 0) return 0;
  if (time <= pillars[0].time) return type === 'zero_rate' ? pillars[0].zero_rate : pillars[0].discount_factor;
  if (time >= pillars[pillars.length - 1].time) {
    return type === 'zero_rate' ? pillars[pillars.length - 1].zero_rate : pillars[pillars.length - 1].discount_factor;
  }

  // Find surrounding pillars
  for (let i = 0; i < pillars.length - 1; i++) {
    if (time >= pillars[i].time && time <= pillars[i + 1].time) {
      const t0 = pillars[i].time;
      const t1 = pillars[i + 1].time;
      const v0 = type === 'zero_rate' ? pillars[i].zero_rate : pillars[i].discount_factor;
      const v1 = type === 'zero_rate' ? pillars[i + 1].zero_rate : pillars[i + 1].discount_factor;
      const ratio = (time - t0) / (t1 - t0);
      return v0 + ratio * (v1 - v0);
    }
  }
  return 0;
}

function interpolateForwardRate(time: number, forwardCurve: ForwardRatePoint[]): number {
  if (forwardCurve.length === 0) return 0;
  if (time <= forwardCurve[0].time) return forwardCurve[0].forward_rate;
  if (time >= forwardCurve[forwardCurve.length - 1].time) {
    return forwardCurve[forwardCurve.length - 1].forward_rate;
  }

  for (let i = 0; i < forwardCurve.length - 1; i++) {
    if (time >= forwardCurve[i].time && time <= forwardCurve[i + 1].time) {
      const t0 = forwardCurve[i].time;
      const t1 = forwardCurve[i + 1].time;
      const v0 = forwardCurve[i].forward_rate;
      const v1 = forwardCurve[i + 1].forward_rate;
      const ratio = (time - t0) / (t1 - t0);
      return v0 + ratio * (v1 - v0);
    }
  }
  return 0;
}

// Generate grid points for short-term chart (Daily up to 3M, Weekly up to 1Y)
function generateShortTermGrid(): number[] {
  const grid: number[] = [];
  const dayFraction = 1 / 365;
  const weekFraction = 7 / 365;
  const threeMonths = 0.25; // 3M in years
  const oneYear = 1.0;

  // Daily grid up to 3M
  for (let t = 0; t <= threeMonths; t += dayFraction) {
    grid.push(t);
  }

  // Weekly grid from 3M to 1Y
  for (let t = threeMonths + weekFraction; t <= oneYear; t += weekFraction) {
    grid.push(t);
  }

  return grid;
}

// Generate grid points for long-term chart (Monthly up to 20Y, Yearly from 20Y to 30Y)
function generateLongTermGrid(): number[] {
  const grid: number[] = [];
  const monthFraction = 1 / 12;

  // Monthly grid up to 20Y
  for (let t = monthFraction; t <= 20; t += monthFraction) {
    grid.push(t);
  }

  // Yearly grid from 20Y to 30Y
  for (let t = 21; t <= 30; t += 1) {
    grid.push(t);
  }

  return grid;
}

function formatTimeLabel(time: number, isShortTerm: boolean): string {
  if (isShortTerm) {
    const days = Math.round(time * 365);
    if (days < 7) return `${days}D`;
    if (days < 30) return `${Math.round(days / 7)}W`;
    return `${Math.round(days / 30)}M`;
  } else {
    if (time < 1) return `${Math.round(time * 12)}M`;
    return `${time.toFixed(1)}Y`;
  }
}

function createChartOptions(yAxisLabel: string, xAxisLabel: string) {
  return {
    responsive: true,
    maintainAspectRatio: false,
    plugins: {
      legend: { display: false },
      tooltip: {
        backgroundColor: 'rgba(0, 0, 0, 0.8)',
        titleColor: '#fff',
        bodyColor: '#fff',
        callbacks: {
          title: (items: { label: string }[]) => `Time: ${items[0].label}`,
          label: (item: { raw: unknown }) => {
            const value = item.raw as number;
            if (chartType.value === 'zero_rate') {
              return `Zero Rate: ${value.toFixed(4)}%`;
            } else if (chartType.value === 'discount_factor') {
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
        title: { display: true, text: xAxisLabel, color: 'rgba(255, 255, 255, 0.6)' },
        ticks: { color: 'rgba(255, 255, 255, 0.6)', maxTicksLimit: 12 },
        grid: { color: 'rgba(255, 255, 255, 0.1)' },
      },
      y: {
        title: { display: true, text: yAxisLabel, color: 'rgba(255, 255, 255, 0.6)' },
        ticks: { color: 'rgba(255, 255, 255, 0.6)' },
        grid: { color: 'rgba(255, 255, 255, 0.1)' },
      },
    },
  };
}

// Chart functions
function updateCharts() {
  if (!buildResult.value?.pillars) return;

  const pillars = buildResult.value.pillars;
  const forwardCurve = buildResult.value.forward_curve || [];

  const chartLabels: Record<string, string> = {
    zero_rate: 'Zero Rate (%)',
    discount_factor: 'Discount Factor',
    forward_rate: 'Forward Rate (%)',
  };

  const chartColors: Record<string, string> = {
    zero_rate: '#6366f1',
    discount_factor: '#6366f1',
    forward_rate: '#10b981',
  };

  const currentLabel = chartLabels[chartType.value];
  const currentColor = chartColors[chartType.value];

  // Generate grids
  const shortTermGrid = generateShortTermGrid();
  const longTermGrid = generateLongTermGrid();

  // Prepare data for short-term chart
  let shortTermData: number[];
  if (chartType.value === 'forward_rate') {
    shortTermData = shortTermGrid.map(t => interpolateForwardRate(t, forwardCurve) * 100);
  } else if (chartType.value === 'zero_rate') {
    shortTermData = shortTermGrid.map(t => interpolateValue(t, pillars, 'zero_rate') * 100);
  } else {
    shortTermData = shortTermGrid.map(t => interpolateValue(t, pillars, 'discount_factor'));
  }
  const shortTermLabels = shortTermGrid.map(t => formatTimeLabel(t, true));

  // Prepare data for long-term chart
  let longTermData: number[];
  if (chartType.value === 'forward_rate') {
    longTermData = longTermGrid.map(t => interpolateForwardRate(t, forwardCurve) * 100);
  } else if (chartType.value === 'zero_rate') {
    longTermData = longTermGrid.map(t => interpolateValue(t, pillars, 'zero_rate') * 100);
  } else {
    longTermData = longTermGrid.map(t => interpolateValue(t, pillars, 'discount_factor'));
  }
  const longTermLabels = longTermGrid.map(t => formatTimeLabel(t, false));

  // Update short-term chart
  if (shortTermChartCanvas.value) {
    if (shortTermChartInstance) {
      shortTermChartInstance.destroy();
    }
    const ctx = shortTermChartCanvas.value.getContext('2d');
    if (ctx) {
      shortTermChartInstance = new Chart(ctx, {
        type: 'line',
        data: {
          labels: shortTermLabels,
          datasets: [{
            label: currentLabel,
            data: shortTermData,
            borderColor: currentColor,
            backgroundColor: `${currentColor}1a`,
            borderWidth: 2,
            fill: true,
            tension: 0.3,
            pointRadius: 1,
            pointBackgroundColor: currentColor,
          }],
        },
        options: createChartOptions(currentLabel, 'Short Term (0-1Y: Daily→Weekly)'),
      });
    }
  }

  // Update long-term chart
  if (longTermChartCanvas.value) {
    if (longTermChartInstance) {
      longTermChartInstance.destroy();
    }
    const ctx = longTermChartCanvas.value.getContext('2d');
    if (ctx) {
      longTermChartInstance = new Chart(ctx, {
        type: 'line',
        data: {
          labels: longTermLabels,
          datasets: [{
            label: currentLabel,
            data: longTermData,
            borderColor: currentColor,
            backgroundColor: `${currentColor}1a`,
            borderWidth: 2,
            fill: true,
            tension: 0.3,
            pointRadius: 1,
            pointBackgroundColor: currentColor,
          }],
        },
        options: createChartOptions(currentLabel, 'Long Term (0-30Y: Monthly→Yearly)'),
      });
    }
  }
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
      const tenorYears = (eventDate.getTime() - referenceDate.getTime()) / (365.25 * 24 * 60 * 60 * 1000);

      // Skip past events
      if (tenorYears < 0) continue;

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
    calibrationMethod.value = curve.calibrationMethod;
    interpolation.value = curve.interpolation;
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
    // Map frontend interpolation values to backend snake_case format
    const interpolationMap: Record<string, string> = {
      'linear': 'linear',
      'loglinear': 'log_linear',
      'cubic': 'cubic_spline',
      'monotone_cubic': 'cubic_spline',
      'flat_forward': 'linear',
    };

    // Build instrument payload including events
    const instrumentPayload = enabledInstruments.value.map(inst => {
      if (inst.type === 'event') {
        return {
          instrument_type: 'event',
          tenor: '',
          rate: 0,
          event_date: inst.eventDate,
          expected_rate_spike: inst.rate, // rate field stores the spike for events
        };
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
        interpolation: interpolationMap[interpolation.value] || 'log_linear',
      }),
    });

    if (!response.ok) {
      const errorData = await response.json();
      throw new Error(errorData.error || errorData.message || 'Build failed');
    }

    buildResult.value = await response.json();

    // Update original rates after successful build
    instruments.value.forEach(inst => {
      inst.originalRate = inst.rate;
    });

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
  calibrationMethod.value = selectedCurve.value.calibrationMethod;
  interpolation.value = selectedCurve.value.interpolation;
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
  if (buildResult.value?.pillars) {
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

          <div v-if="selectedCurve" class="mt-3 text-sm">
            <div class="flex justify-between text-xs">
              <span class="text-[var(--text-muted)]">Rate Index:</span>
              <span class="text-[var(--text-primary)] font-medium">{{ selectedCurve.rateIndex }}</span>
            </div>
          </div>
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
                  class="px-1.5 py-0.5 text-xs rounded bg-amber-500/20 text-amber-400 font-mono"
                  :title="'Event Date'"
                >{{ inst.eventDate }}</span>
                <input
                  type="number"
                  :value="(inst.rate * 10000).toFixed(0)"
                  step="1"
                  class="w-14 px-1.5 py-0.5 text-right text-xs rounded bg-amber-500/10 border border-amber-500/30 text-amber-400"
                  title="Expected rate spike in basis points"
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
                <option v-for="m in calibrationMethods" :key="m" :value="m">{{ m }}</option>
              </select>
            </div>
            <div>
              <label class="block text-xs text-[var(--text-muted)] mb-1">Interpolation</label>
              <select
                v-model="interpolation"
                class="w-full px-2 py-1.5 rounded bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm"
              >
                <option v-for="m in interpolationMethods" :key="m" :value="m">{{ m }}</option>
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

      <!-- Right Panel: Curve Chart -->
      <div class="lg:col-span-2">
        <div class="glass-card p-6 h-full">
          <div class="flex items-center justify-between mb-4">
            <h3 class="text-lg font-semibold text-[var(--text-primary)]">Yield Curve</h3>
            <div v-if="buildResult?.pillars" class="flex gap-2">
              <button
                :class="[
                  'px-3 py-1.5 text-xs rounded-lg transition-colors',
                  chartType === 'zero_rate' ? 'bg-[var(--primary)] text-white' : 'bg-[var(--surface)] text-[var(--text-secondary)]'
                ]"
                @click="chartType = 'zero_rate'"
              >
                Zero Rate
              </button>
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
            <!-- Short-term Chart (0-1Y) -->
            <div>
              <h4 class="text-sm font-medium text-[var(--text-secondary)] mb-2">
                <i class="fas fa-clock text-xs mr-1"></i>
                Short Term (0-1Y)
              </h4>
              <div class="h-48 bg-[var(--surface)] rounded-lg p-2">
                <canvas ref="shortTermChartCanvas"></canvas>
              </div>
            </div>

            <!-- Long-term Chart (0-30Y) -->
            <div>
              <h4 class="text-sm font-medium text-[var(--text-secondary)] mb-2">
                <i class="fas fa-calendar-alt text-xs mr-1"></i>
                Long Term (0-30Y)
              </h4>
              <div class="h-48 bg-[var(--surface)] rounded-lg p-2">
                <canvas ref="longTermChartCanvas"></canvas>
              </div>
            </div>
          </div>

          <!-- Build Info -->
          <div v-if="buildResult" class="mt-4 pt-4 border-t border-[var(--glass-border)]">
            <div class="grid grid-cols-3 gap-4 text-sm">
              <div>
                <span class="text-[var(--text-muted)]">Instruments:</span>
                <span class="ml-2 text-[var(--text-primary)] font-medium">{{ buildResult.instrument_count }}</span>
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
