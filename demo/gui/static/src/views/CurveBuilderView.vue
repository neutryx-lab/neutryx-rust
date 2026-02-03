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
  tenor: string;
  tenor_years: number;
  rate: number;
  frequency?: string;
  description?: string;
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
}

interface CurvePillar {
  time: number;
  discount_factor: number;
  zero_rate: number;
}

interface BuildResult {
  curve_id?: string;
  instrument_count?: number;
  interpolation?: string;
  calculation_time_ms?: number;
  pillars?: CurvePillar[];
  converged?: boolean;
}

// State
const curvesConfig = ref<CurvesData | null>(null);
const selectedCurveName = ref<string>('');
const selectedCurve = ref<CurveConfig | null>(null);
const rateData = ref<RateData | null>(null);
const instruments = ref<DisplayInstrument[]>([]);
const buildResult = ref<BuildResult | null>(null);
const isLoading = ref(false);
const isBuilding = ref(false);
const loadError = ref<string | null>(null);

// Build settings (editable)
const calibrationMethod = ref<string>('sequential');
const interpolation = ref<string>('loglinear');
const allowExtrapolation = ref<boolean>(true);

// Chart
const chartCanvas = ref<HTMLCanvasElement | null>(null);
let chartInstance: Chart | null = null;
const chartType = ref<'zero_rate' | 'discount_factor'>('zero_rate');

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

const summaryStats = computed(() => [
  { label: 'Instruments', value: `${enabledInstruments.value.length}/${instruments.value.length}`, icon: 'fa-list-alt', color: '#3b82f6' },
  { label: 'Avg Rate', value: enabledInstruments.value.length > 0
      ? `${(enabledInstruments.value.reduce((sum, i) => sum + i.rate, 0) / enabledInstruments.value.length * 100).toFixed(2)}%`
      : '-', icon: 'fa-percent', color: '#8b5cf6' },
  { label: 'Interpolation', value: interpolation.value, icon: 'fa-wave-square', color: '#10b981' },
  { label: 'Status', value: buildResult.value ? 'Built' : 'Pending', icon: 'fa-info-circle', color: buildResult.value ? '#10b981' : '#f59e0b' },
]);

// Chart functions
function updateChart() {
  if (!chartCanvas.value || !buildResult.value?.pillars) return;

  const pillars = buildResult.value.pillars;
  const labels = pillars.map(p => p.time.toFixed(2));
  const data = chartType.value === 'zero_rate'
    ? pillars.map(p => p.zero_rate * 100)
    : pillars.map(p => p.discount_factor);

  if (chartInstance) {
    chartInstance.destroy();
  }

  const ctx = chartCanvas.value.getContext('2d');
  if (!ctx) return;

  chartInstance = new Chart(ctx, {
    type: 'line',
    data: {
      labels,
      datasets: [{
        label: chartType.value === 'zero_rate' ? 'Zero Rate (%)' : 'Discount Factor',
        data,
        borderColor: '#6366f1',
        backgroundColor: 'rgba(99, 102, 241, 0.1)',
        borderWidth: 2,
        fill: true,
        tension: 0.3,
        pointRadius: 3,
        pointBackgroundColor: '#6366f1',
      }],
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      plugins: {
        legend: {
          display: false,
        },
        tooltip: {
          backgroundColor: 'rgba(0, 0, 0, 0.8)',
          titleColor: '#fff',
          bodyColor: '#fff',
          callbacks: {
            title: (items) => `Time: ${items[0].label}Y`,
            label: (item) => {
              const value = item.raw as number;
              return chartType.value === 'zero_rate'
                ? `Zero Rate: ${value.toFixed(4)}%`
                : `DF: ${value.toFixed(6)}`;
            },
          },
        },
      },
      scales: {
        x: {
          title: {
            display: true,
            text: 'Time (Years)',
            color: 'rgba(255, 255, 255, 0.6)',
          },
          ticks: { color: 'rgba(255, 255, 255, 0.6)' },
          grid: { color: 'rgba(255, 255, 255, 0.1)' },
        },
        y: {
          title: {
            display: true,
            text: chartType.value === 'zero_rate' ? 'Zero Rate (%)' : 'Discount Factor',
            color: 'rgba(255, 255, 255, 0.6)',
          },
          ticks: { color: 'rgba(255, 255, 255, 0.6)' },
          grid: { color: 'rgba(255, 255, 255, 0.1)' },
        },
      },
    },
  });
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
  return `${currency}-${typeLabel}-${tenor}`;
}

function loadInstrumentsForCurve() {
  if (!selectedCurve.value || !rateData.value) {
    instruments.value = [];
    return;
  }

  const curveInstrumentIds = new Set(selectedCurve.value.instruments);
  const currency = rateData.value.currency;

  // Build display instruments from rate data
  const displayInstruments: DisplayInstrument[] = [];

  for (const rateInst of rateData.value.instruments) {
    const id = buildInstrumentId(rateInst.type, rateInst.tenor, currency);

    displayInstruments.push({
      id,
      type: rateInst.type,
      tenor: rateInst.tenor,
      tenorYears: rateInst.tenor_years,
      rate: rateInst.rate,
      originalRate: rateInst.rate,
      enabled: curveInstrumentIds.has(id),
    });
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
  try {
    const response = await fetch('/api/curves/build', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        curveName: selectedCurve.value.name,
        rateIndex: selectedCurve.value.rateIndex,
        instruments: enabledInstruments.value.map(inst => ({
          id: inst.id,
          type: inst.type,
          tenor: inst.tenor,
          rate: inst.rate,
        })),
        calibrationMethod: calibrationMethod.value,
        interpolation: interpolation.value,
        allowExtrapolation: allowExtrapolation.value,
      }),
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.message || 'Build failed');
    }

    buildResult.value = await response.json();

    // Update original rates after successful build
    instruments.value.forEach(inst => {
      inst.originalRate = inst.rate;
    });

    // Update chart
    await nextTick();
    updateChart();
  } catch (error) {
    console.error('Build failed:', error);
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
    'ID,Type,Tenor,Rate(%),Enabled',
    ...instruments.value.map(
      inst => `${inst.id},${inst.type},${inst.tenor},${(inst.rate * 100).toFixed(4)},${inst.enabled}`
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
    updateChart();
  }
});

// Lifecycle
onMounted(() => {
  loadCurvesConfig();
});

onUnmounted(() => {
  if (chartInstance) {
    chartInstance.destroy();
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
                inst.enabled ? 'bg-[var(--surface)]' : 'opacity-40'
              ]"
            >
              <input
                type="checkbox"
                :checked="inst.enabled"
                class="w-3.5 h-3.5 rounded border-[var(--glass-border)]"
                @change="toggleEnabled(idx)"
              >
              <span class="flex-1 font-mono text-xs text-[var(--text-secondary)] truncate">{{ inst.id }}</span>
              <input
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
                  chartType === 'discount_factor' ? 'bg-[var(--primary)] text-white' : 'bg-[var(--surface)] text-[var(--text-secondary)]'
                ]"
                @click="chartType = 'discount_factor'"
              >
                Discount Factor
              </button>
            </div>
          </div>

          <!-- Empty State -->
          <div v-if="!buildResult" class="flex flex-col items-center justify-center h-80 text-[var(--text-muted)]">
            <i class="fas fa-chart-line text-5xl mb-4 opacity-30"></i>
            <p class="text-sm">Build a curve to see the chart</p>
          </div>

          <!-- Chart -->
          <div v-else class="h-80">
            <canvas ref="chartCanvas"></canvas>
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
