<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick } from 'vue';
import { Chart, registerables, type TooltipItem, type ChartConfiguration } from 'chart.js';
import { useToast } from '@/composables/useToast';

Chart.register(...registerables);
const toast = useToast();

// Types
type ScenarioType = 'parametric' | 'historical' | 'reverse';

interface ScenarioParams {
  rateShift: number;
  volShift: number;
  fxShift: number;
  creditSpread: number;
}

interface PnLDecomposition {
  rates: number;
  vol: number;
  fx: number;
  credit: number;
}

interface Scenario {
  id: string;
  name: string;
  description: string;
  type: ScenarioType;
  params: ScenarioParams;
  pnl: number | null;
  decomposition: PnLDecomposition | null;
}

// Default scenarios
const defaultScenarios: Scenario[] = [
  { id: 'base', name: 'Base Case', description: 'Current market conditions', type: 'parametric',
    params: { rateShift: 0, volShift: 0, fxShift: 0, creditSpread: 0 }, pnl: null, decomposition: null },
  { id: 'rates_up', name: 'Rates +100bp', description: 'Parallel shift up', type: 'parametric',
    params: { rateShift: 100, volShift: 0, fxShift: 0, creditSpread: 0 }, pnl: null, decomposition: null },
  { id: 'rates_down', name: 'Rates -100bp', description: 'Parallel shift down', type: 'parametric',
    params: { rateShift: -100, volShift: 0, fxShift: 0, creditSpread: 0 }, pnl: null, decomposition: null },
  { id: 'vol_up', name: 'Vol +25%', description: 'Volatility increase', type: 'parametric',
    params: { rateShift: 0, volShift: 25, fxShift: 0, creditSpread: 0 }, pnl: null, decomposition: null },
  { id: 'fx_stress', name: 'FX Stress', description: 'USD +10% vs all', type: 'parametric',
    params: { rateShift: 0, volShift: 0, fxShift: 10, creditSpread: 0 }, pnl: null, decomposition: null },
  { id: 'crisis_2008', name: '2008 Crisis', description: 'Historical replay', type: 'historical',
    params: { rateShift: -150, volShift: 80, fxShift: 15, creditSpread: 200 }, pnl: null, decomposition: null },
  { id: 'covid_2020', name: 'COVID-19', description: 'March 2020 shock', type: 'historical',
    params: { rateShift: -100, volShift: 120, fxShift: 8, creditSpread: 150 }, pnl: null, decomposition: null },
  { id: 'euro_crisis', name: 'Euro Crisis 2011', description: 'European debt crisis', type: 'historical',
    params: { rateShift: 50, volShift: 40, fxShift: -12, creditSpread: 180 }, pnl: null, decomposition: null },
  { id: 'reverse_var', name: 'Reverse VaR', description: 'Find -$5M scenario', type: 'reverse',
    params: { rateShift: 0, volShift: 0, fxShift: 0, creditSpread: 0 }, pnl: null, decomposition: null },
];

// Risk factor config (shared across charts and legend)
const riskFactors = [
  { key: 'rates', label: 'Rates', color: '#3b82f6' },
  { key: 'vol', label: 'Volatility', color: '#10b981' },
  { key: 'fx', label: 'FX', color: '#8b5cf6' },
  { key: 'credit', label: 'Credit', color: '#f59e0b' },
];

// State
const pnlChart = ref<Chart | null>(null);
const pnlChartContainer = ref<HTMLDivElement | null>(null);
const decompChart = ref<Chart | null>(null);
const decompChartContainer = ref<HTMLDivElement | null>(null);
const scenarios = ref<Scenario[]>(JSON.parse(JSON.stringify(defaultScenarios)));
const selectedType = ref<ScenarioType>('parametric');
const selectedScenarioId = ref<string>('base');
const isRunning = ref(false);

// Computed
const filteredScenarios = computed(() =>
  scenarios.value.filter(s => s.type === selectedType.value)
);

const selectedScenario = computed(() =>
  scenarios.value.find(s => s.id === selectedScenarioId.value)
);

const hasResults = computed(() =>
  scenarios.value.some(s => s.type === selectedType.value && s.pnl !== null)
);

const calculatedScenarios = computed(() =>
  filteredScenarios.value.filter(s => s.pnl !== null)
);

// Top summary stat cards
const topStats = computed(() => {
  if (!hasResults.value) return [];
  const pnls = calculatedScenarios.value.map(s => s.pnl!);
  if (pnls.length === 0) return [];

  const best = Math.max(...pnls);
  const worst = Math.min(...pnls);
  const sorted = [...pnls].sort((a, b) => a - b);
  const var95Idx = Math.floor(sorted.length * 0.05);
  const var95 = sorted[var95Idx] ?? sorted[0];
  const lossCount = pnls.filter(p => p < 0).length;
  const lossPct = ((lossCount / pnls.length) * 100).toFixed(0);

  return [
    { label: 'Best Case', value: formatPnl(best), icon: 'fa-arrow-up', color: '#10b981' },
    { label: 'Worst Case', value: formatPnl(worst), icon: 'fa-arrow-down', color: '#ef4444' },
    { label: 'VaR (95%)', value: formatPnl(var95), icon: 'fa-shield-alt', color: '#8b5cf6' },
    { label: 'Loss Scenarios', value: `${lossPct}%`, icon: 'fa-exclamation-triangle', color: '#f59e0b' },
  ];
});

// Risk metrics
const riskMetrics = computed(() => {
  const pnls = calculatedScenarios.value.map(s => s.pnl!);
  if (pnls.length < 2) return [];

  const mean = pnls.reduce((a, b) => a + b, 0) / pnls.length;
  const variance = pnls.reduce((a, b) => a + (b - mean) ** 2, 0) / pnls.length;
  const stdDev = Math.sqrt(variance);

  const sorted = [...pnls].sort((a, b) => a - b);
  const var95Idx = Math.max(0, Math.floor(sorted.length * 0.05));
  const var95 = sorted[var95Idx];
  const tailValues = sorted.filter(v => v <= var95);
  const cvar = tailValues.length > 0 ? tailValues.reduce((a, b) => a + b, 0) / tailValues.length : var95;

  const sharpe = stdDev > 0 ? mean / stdDev : 0;
  const maxDrawdown = Math.min(...pnls);

  return [
    { label: 'VaR (95%)', value: formatPnl(var95), negative: var95 < 0 },
    { label: 'CVaR (95%)', value: formatPnl(cvar), negative: cvar < 0 },
    { label: 'P&L Volatility', value: formatPnl(stdDev), negative: false },
    { label: 'Mean P&L', value: formatPnl(mean), negative: mean < 0 },
    { label: 'Sharpe Ratio', value: sharpe.toFixed(2), negative: sharpe < 0 },
    { label: 'Max Drawdown', value: formatPnl(maxDrawdown), negative: true },
  ];
});

// Sensitivity ranking
const sensitivityRanking = computed(() => {
  const cs = calculatedScenarios.value;
  if (cs.length < 2) return [];

  const absContributions = riskFactors.map(f => {
    const total = cs.reduce((sum, s) => {
      if (!s.decomposition) return sum;
      return sum + Math.abs(s.decomposition[f.key as keyof PnLDecomposition]);
    }, 0);
    return { ...f, total };
  });

  const maxTotal = Math.max(...absContributions.map(c => c.total), 1);

  return absContributions
    .sort((a, b) => b.total - a.total)
    .map(c => ({
      label: c.label,
      color: c.color,
      pct: Math.round((c.total / maxTotal) * 100),
    }));
});

// Type buttons config
const typeButtons = [
  { type: 'parametric' as const, label: 'Parametric', icon: 'fa-sliders-h' },
  { type: 'historical' as const, label: 'Historical', icon: 'fa-history' },
  { type: 'reverse' as const, label: 'Reverse', icon: 'fa-undo' },
];

// Parameter sliders config
const paramSliders = [
  { key: 'rateShift', label: 'Rate Shift', unit: 'bp', min: -200, max: 200 },
  { key: 'volShift', label: 'Vol Shift', unit: '%', min: -50, max: 150 },
  { key: 'fxShift', label: 'FX Shift', unit: '%', min: -30, max: 30 },
  { key: 'creditSpread', label: 'Credit Spread', unit: 'bp', min: 0, max: 300 },
];

// Utility functions
function formatPnl(pnl: number | null): string {
  if (pnl === null) return '--';
  const value = pnl / 1000000;
  if (Math.abs(value) >= 1) {
    return value > 0 ? `+$${value.toFixed(1)}M` : `-$${Math.abs(value).toFixed(1)}M`;
  }
  const kValue = pnl / 1000;
  return kValue > 0 ? `+$${kValue.toFixed(0)}K` : `-$${Math.abs(kValue).toFixed(0)}K`;
}

function formatParamValue(value: number): string {
  return value > 0 ? `+${value}` : String(value);
}

function calculatePnL(params: ScenarioParams): { pnl: number; decomposition: PnLDecomposition } {
  const rates = params.rateShift * -24000;
  const vol = params.volShift * 26000;
  const fx = params.fxShift * -89000;
  const credit = params.creditSpread * -15000;
  const noise = (Math.random() - 0.5) * 100000;
  return {
    pnl: rates + vol + fx + credit + noise,
    decomposition: { rates, vol, fx, credit },
  };
}

// Chart rendering
function renderPnlChart() {
  if (!pnlChartContainer.value) return;

  if (pnlChart.value) {
    pnlChart.value.destroy();
    pnlChart.value = null;
  }

  const cs = calculatedScenarios.value;
  if (cs.length === 0) return;

  pnlChartContainer.value.innerHTML = '';
  const canvas = document.createElement('canvas');
  pnlChartContainer.value.appendChild(canvas);

  const labels = cs.map(s => s.name);
  const data = cs.map(s => (s.pnl ?? 0) / 1000000);
  const colors = data.map(v => v >= 0 ? 'rgba(16, 185, 129, 0.8)' : 'rgba(239, 68, 68, 0.8)');

  const config: ChartConfiguration<'bar'> = {
    type: 'bar',
    data: {
      labels,
      datasets: [{
        label: 'P&L Impact',
        data,
        backgroundColor: colors,
        borderColor: colors.map(c => c.replace('0.8', '1')),
        borderWidth: 1,
        borderRadius: 4,
      }],
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      indexAxis: 'y',
      plugins: {
        legend: { display: false },
        tooltip: {
          backgroundColor: 'rgba(0, 0, 0, 0.8)',
          titleColor: '#fff',
          bodyColor: '#fff',
          padding: 12,
          cornerRadius: 8,
          callbacks: {
            label: (context: TooltipItem<'bar'>) => {
              const value = context.parsed.x ?? 0;
              const sign = value >= 0 ? '+' : '';
              return `P&L: ${sign}$${value.toFixed(2)}M`;
            },
          },
        },
      },
      scales: {
        x: {
          grid: { color: 'rgba(255, 255, 255, 0.05)' },
          ticks: {
            color: '#94a3b8',
            callback: (value) => `$${value}M`,
          },
        },
        y: {
          grid: { display: false },
          ticks: { color: '#94a3b8', font: { size: 11 } },
        },
      },
    },
  };

  pnlChart.value = new Chart(canvas, config);
}

function renderDecompositionChart() {
  if (!decompChartContainer.value) return;

  if (decompChart.value) {
    decompChart.value.destroy();
    decompChart.value = null;
  }

  const cs = calculatedScenarios.value.filter(s => s.decomposition);
  if (cs.length === 0) return;

  decompChartContainer.value.innerHTML = '';
  const canvas = document.createElement('canvas');
  decompChartContainer.value.appendChild(canvas);

  const labels = cs.map(s => s.name);

  const datasets = riskFactors.map(f => ({
    label: f.label,
    data: cs.map(s => (s.decomposition![f.key as keyof PnLDecomposition]) / 1000000),
    backgroundColor: `${f.color}cc`,
    borderColor: f.color,
    borderWidth: 1,
    borderRadius: 3,
  }));

  const config: ChartConfiguration<'bar'> = {
    type: 'bar',
    data: { labels, datasets },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      plugins: {
        legend: { display: false },
        tooltip: {
          backgroundColor: 'rgba(0, 0, 0, 0.8)',
          titleColor: '#fff',
          bodyColor: '#fff',
          padding: 12,
          cornerRadius: 8,
          callbacks: {
            label: (context: TooltipItem<'bar'>) => {
              const value = context.parsed.y ?? 0;
              const sign = value >= 0 ? '+' : '';
              return `${context.dataset.label}: ${sign}$${value.toFixed(2)}M`;
            },
          },
        },
      },
      scales: {
        x: {
          stacked: true,
          grid: { display: false },
          ticks: { color: '#94a3b8', font: { size: 10 }, maxRotation: 45 },
        },
        y: {
          stacked: true,
          grid: { color: 'rgba(255, 255, 255, 0.05)' },
          ticks: {
            color: '#94a3b8',
            callback: (value) => `$${value}M`,
          },
        },
      },
    },
  };

  decompChart.value = new Chart(canvas, config);
}

function renderCharts() {
  renderPnlChart();
  nextTick(() => renderDecompositionChart());
}

// Actions
function selectType(type: ScenarioType) {
  selectedType.value = type;
  const firstOfType = scenarios.value.find(s => s.type === type);
  selectedScenarioId.value = firstOfType?.id ?? '';
  if (hasResults.value) nextTick(() => renderCharts());
}

function selectScenario(id: string) {
  selectedScenarioId.value = id;
}

function updateParam(key: keyof ScenarioParams, value: number) {
  const scenario = scenarios.value.find(s => s.id === selectedScenarioId.value);
  if (scenario) {
    scenario.params[key] = value;
    scenario.pnl = null;
    scenario.decomposition = null;
  }
}

async function runScenarios() {
  if (isRunning.value) return;

  isRunning.value = true;
  await new Promise(resolve => setTimeout(resolve, 800));

  scenarios.value = scenarios.value.map(s => {
    if (s.type === selectedType.value) {
      const result = calculatePnL(s.params);
      return { ...s, pnl: result.pnl, decomposition: result.decomposition };
    }
    return s;
  });

  await nextTick();
  renderCharts();
  isRunning.value = false;
  toast.success('Scenario calculation completed');
}

function addScenario() {
  const newId = `custom_${Date.now()}`;
  const count = filteredScenarios.value.length + 1;
  scenarios.value.push({
    id: newId,
    name: `Custom ${count}`,
    description: 'User-defined scenario',
    type: selectedType.value,
    params: { rateShift: 0, volShift: 0, fxShift: 0, creditSpread: 0 },
    pnl: null,
    decomposition: null,
  });
  selectedScenarioId.value = newId;
  toast.success('New scenario added');
}

onMounted(() => {
  if (hasResults.value) renderCharts();
});

onUnmounted(() => {
  if (pnlChart.value) { pnlChart.value.destroy(); pnlChart.value = null; }
  if (decompChart.value) { decompChart.value.destroy(); decompChart.value = null; }
});
</script>

<template>
  <div class="scenarios-view">
    <!-- Top Summary Stats -->
    <div v-if="hasResults" class="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
      <div
        v-for="stat in topStats"
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
      <!-- Left Panel: Scenario List -->
      <div class="lg:col-span-1 space-y-4">
        <!-- Type Selector -->
        <div class="glass-card p-4">
          <div class="flex gap-2">
            <button
              v-for="btn in typeButtons"
              :key="btn.type"
              :class="[
                'flex-1 px-3 py-2 rounded-lg text-sm font-medium transition-all duration-200',
                selectedType === btn.type
                  ? 'bg-[var(--primary)] text-white'
                  : 'text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]'
              ]"
              @click="selectType(btn.type)"
            >
              <i :class="['fas', btn.icon, 'mr-2']"></i>
              {{ btn.label }}
            </button>
          </div>
        </div>

        <!-- Scenario List -->
        <div class="glass-card p-4">
          <div class="flex items-center justify-between mb-3">
            <h3 class="text-sm font-semibold text-[var(--text-primary)]">Scenarios</h3>
            <button
              class="text-xs text-[var(--primary)] hover:text-[var(--primary-light)]"
              @click="addScenario"
            >
              <i class="fas fa-plus mr-1"></i>Add
            </button>
          </div>

          <div class="space-y-2 max-h-80 overflow-y-auto">
            <button
              v-for="scenario in filteredScenarios"
              :key="scenario.id"
              :class="[
                'w-full flex items-center justify-between p-3 rounded-lg transition-all duration-200 text-left',
                selectedScenarioId === scenario.id
                  ? 'bg-[var(--surface)] border border-[var(--primary)]'
                  : 'hover:bg-[var(--surface-hover)] border border-transparent'
              ]"
              @click="selectScenario(scenario.id)"
            >
              <div>
                <p class="text-sm font-medium text-[var(--text-primary)]">{{ scenario.name }}</p>
                <p class="text-xs text-[var(--text-muted)]">{{ scenario.description }}</p>
              </div>
              <span
                :class="[
                  'text-sm font-medium',
                  scenario.pnl === null ? 'text-[var(--text-muted)]' :
                    scenario.pnl >= 0 ? 'text-[var(--success)]' : 'text-[var(--danger)]'
                ]"
              >
                {{ formatPnl(scenario.pnl) }}
              </span>
            </button>
          </div>
        </div>

        <!-- Parameter Sliders -->
        <div class="glass-card p-4">
          <h3 class="text-sm font-semibold text-[var(--text-primary)] mb-4">Parameters</h3>

          <div class="space-y-4">
            <div v-for="slider in paramSliders" :key="slider.key" class="space-y-1">
              <div class="flex items-center justify-between">
                <label class="text-xs text-[var(--text-muted)]">{{ slider.label }}</label>
                <span class="text-xs font-medium text-[var(--text-primary)]">
                  {{ formatParamValue(selectedScenario?.params[slider.key as keyof ScenarioParams] ?? 0) }}{{ slider.unit }}
                </span>
              </div>
              <input
                type="range"
                :min="slider.min"
                :max="slider.max"
                :value="selectedScenario?.params[slider.key as keyof ScenarioParams] ?? 0"
                class="w-full h-2 bg-[var(--surface)] rounded-full appearance-none cursor-pointer"
                @input="updateParam(slider.key as keyof ScenarioParams, Number(($event.target as HTMLInputElement).value))"
              />
            </div>
          </div>

          <button
            :class="[
              'w-full mt-4 px-4 py-2 rounded-lg font-medium transition-all duration-200',
              isRunning
                ? 'bg-[var(--surface)] text-[var(--text-muted)] cursor-not-allowed'
                : 'bg-[var(--primary)] text-white hover:bg-[var(--primary-dark)]'
            ]"
            :disabled="isRunning"
            @click="runScenarios"
          >
            <i :class="['fas', isRunning ? 'fa-spinner fa-spin' : 'fa-play', 'mr-2']"></i>
            {{ isRunning ? 'Running...' : 'Run Scenarios' }}
          </button>
        </div>
      </div>

      <!-- Right Panel: Results -->
      <div class="lg:col-span-2 space-y-4">
        <!-- P&L Impact Chart -->
        <div class="glass-card p-5">
          <h3 class="text-base font-semibold text-[var(--text-primary)] mb-3">
            <i class="fas fa-chart-bar text-sm mr-2 text-[var(--text-muted)]"></i>
            P&L Impact
          </h3>

          <div v-if="!hasResults" class="flex flex-col items-center justify-center h-56 text-center">
            <i class="fas fa-chart-bar text-5xl mb-4 opacity-20 text-[var(--text-muted)]"></i>
            <p class="text-sm text-[var(--text-muted)]">Run scenarios to view results</p>
          </div>

          <div v-else ref="pnlChartContainer" class="h-56"></div>
        </div>

        <!-- Risk Factor Decomposition Chart -->
        <div v-if="hasResults" class="glass-card p-5">
          <h3 class="text-base font-semibold text-[var(--text-primary)] mb-3">
            <i class="fas fa-layer-group text-sm mr-2 text-[var(--text-muted)]"></i>
            Risk Factor Decomposition
          </h3>

          <!-- Legend -->
          <div class="flex flex-wrap gap-3 mb-3">
            <span
              v-for="f in riskFactors"
              :key="f.key"
              class="flex items-center gap-1.5 text-xs text-[var(--text-secondary)]"
            >
              <span class="w-2.5 h-2.5 rounded-sm" :style="{ backgroundColor: f.color }"></span>
              {{ f.label }}
            </span>
          </div>

          <div ref="decompChartContainer" class="h-48"></div>
        </div>

        <!-- Risk Metrics -->
        <div v-if="riskMetrics.length > 0" class="glass-card p-5">
          <h3 class="text-base font-semibold text-[var(--text-primary)] mb-3">
            <i class="fas fa-shield-alt text-sm mr-2 text-[var(--text-muted)]"></i>
            Risk Metrics
          </h3>
          <div class="grid grid-cols-2 md:grid-cols-3 gap-3">
            <div
              v-for="metric in riskMetrics"
              :key="metric.label"
              class="p-3 rounded-lg bg-[var(--surface)]"
            >
              <p class="text-xs text-[var(--text-muted)] mb-1">{{ metric.label }}</p>
              <p
                class="text-base font-semibold"
                :class="metric.negative ? 'text-[var(--danger)]' : 'text-[var(--text-primary)]'"
              >
                {{ metric.value }}
              </p>
            </div>
          </div>
        </div>

        <!-- Top Risk Drivers -->
        <div v-if="sensitivityRanking.length > 0" class="glass-card p-5">
          <h3 class="text-base font-semibold text-[var(--text-primary)] mb-3">
            <i class="fas fa-sort-amount-down text-sm mr-2 text-[var(--text-muted)]"></i>
            Top Risk Drivers
          </h3>
          <div class="space-y-3">
            <div
              v-for="driver in sensitivityRanking"
              :key="driver.label"
              class="flex items-center gap-3"
            >
              <span class="text-xs w-16 text-[var(--text-muted)] shrink-0">{{ driver.label }}</span>
              <div class="flex-1 h-2.5 rounded-full bg-[var(--surface)] overflow-hidden">
                <div
                  class="h-2.5 rounded-full transition-all duration-500"
                  :style="{ width: `${driver.pct}%`, backgroundColor: driver.color }"
                ></div>
              </div>
              <span class="text-xs font-medium text-[var(--text-primary)] w-10 text-right">{{ driver.pct }}%</span>
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

input[type="range"] {
  -webkit-appearance: none;
}

input[type="range"]::-webkit-slider-thumb {
  -webkit-appearance: none;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: var(--primary);
  cursor: pointer;
}

input[type="range"]::-moz-range-thumb {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: var(--primary);
  cursor: pointer;
  border: none;
}
</style>
