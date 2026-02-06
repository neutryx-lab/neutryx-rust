<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue';
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

interface Scenario {
  id: string;
  name: string;
  description: string;
  type: ScenarioType;
  params: ScenarioParams;
  pnl: number | null;
}

// Default scenarios
const defaultScenarios: Scenario[] = [
  { id: 'base', name: 'Base Case', description: 'Current market conditions', type: 'parametric',
    params: { rateShift: 0, volShift: 0, fxShift: 0, creditSpread: 0 }, pnl: null },
  { id: 'rates_up', name: 'Rates +100bp', description: 'Parallel shift up', type: 'parametric',
    params: { rateShift: 100, volShift: 0, fxShift: 0, creditSpread: 0 }, pnl: null },
  { id: 'rates_down', name: 'Rates -100bp', description: 'Parallel shift down', type: 'parametric',
    params: { rateShift: -100, volShift: 0, fxShift: 0, creditSpread: 0 }, pnl: null },
  { id: 'vol_up', name: 'Vol +25%', description: 'Volatility increase', type: 'parametric',
    params: { rateShift: 0, volShift: 25, fxShift: 0, creditSpread: 0 }, pnl: null },
  { id: 'fx_stress', name: 'FX Stress', description: 'USD +10% vs all', type: 'parametric',
    params: { rateShift: 0, volShift: 0, fxShift: 10, creditSpread: 0 }, pnl: null },
  { id: 'crisis_2008', name: '2008 Crisis', description: 'Historical replay', type: 'historical',
    params: { rateShift: -150, volShift: 80, fxShift: 15, creditSpread: 200 }, pnl: null },
  { id: 'covid_2020', name: 'COVID-19', description: 'March 2020 shock', type: 'historical',
    params: { rateShift: -100, volShift: 120, fxShift: 8, creditSpread: 150 }, pnl: null },
  { id: 'euro_crisis', name: 'Euro Crisis 2011', description: 'European debt crisis', type: 'historical',
    params: { rateShift: 50, volShift: 40, fxShift: -12, creditSpread: 180 }, pnl: null },
  { id: 'reverse_var', name: 'Reverse VaR', description: 'Find -$5M scenario', type: 'reverse',
    params: { rateShift: 0, volShift: 0, fxShift: 0, creditSpread: 0 }, pnl: null },
];

// State
const chart = ref<Chart | null>(null);
const chartContainer = ref<HTMLDivElement | null>(null);
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

function calculatePnL(params: ScenarioParams): number {
  const ratePnL = params.rateShift * -24000;
  const volPnL = params.volShift * 26000;
  const fxPnL = params.fxShift * -89000;
  const creditPnL = params.creditSpread * -15000;
  const noise = (Math.random() - 0.5) * 100000;
  return ratePnL + volPnL + fxPnL + creditPnL + noise;
}

// Chart rendering
function renderChart() {
  if (!chartContainer.value) return;

  if (chart.value) {
    chart.value.destroy();
    chart.value = null;
  }

  const calculatedScenarios = scenarios.value.filter(
    s => s.type === selectedType.value && s.pnl !== null
  );

  if (calculatedScenarios.length === 0) return;

  // Create canvas
  chartContainer.value.innerHTML = '';
  const canvas = document.createElement('canvas');
  chartContainer.value.appendChild(canvas);

  const labels = calculatedScenarios.map(s => s.name);
  const data = calculatedScenarios.map(s => (s.pnl ?? 0) / 1000000);
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
        borderWidth: 2,
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
          ticks: { color: '#94a3b8' },
        },
      },
    },
  };

  chart.value = new Chart(canvas, config);
}

// Actions
function selectType(type: ScenarioType) {
  selectedType.value = type;
  const firstOfType = scenarios.value.find(s => s.type === type);
  selectedScenarioId.value = firstOfType?.id ?? '';
  if (hasResults.value) renderChart();
}

function selectScenario(id: string) {
  selectedScenarioId.value = id;
}

function updateParam(key: keyof ScenarioParams, value: number) {
  const scenario = scenarios.value.find(s => s.id === selectedScenarioId.value);
  if (scenario) {
    scenario.params[key] = value;
    scenario.pnl = null;
  }
}

async function runScenarios() {
  if (isRunning.value) return;

  isRunning.value = true;
  await new Promise(resolve => setTimeout(resolve, 800));

  scenarios.value = scenarios.value.map(s => {
    if (s.type === selectedType.value) {
      return { ...s, pnl: calculatePnL(s.params) };
    }
    return s;
  });

  renderChart();
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
  });
  selectedScenarioId.value = newId;
  toast.success('New scenario added');
}

onMounted(() => {
  if (hasResults.value) renderChart();
});

onUnmounted(() => {
  if (chart.value) {
    chart.value.destroy();
    chart.value = null;
  }
});
</script>

<template>
  <div class="scenarios-view">
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
      <div class="lg:col-span-2">
        <div class="glass-card p-6 h-full">
          <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Results</h3>

          <div v-if="!hasResults" class="flex flex-col items-center justify-center h-80 text-center">
            <i class="fas fa-chart-bar text-4xl text-[var(--text-muted)] mb-4"></i>
            <p class="text-[var(--text-muted)]">Run scenarios to view results</p>
          </div>

          <div v-else ref="chartContainer" class="h-80"></div>

          <!-- Results Summary -->
          <div v-if="hasResults" class="grid grid-cols-2 md:grid-cols-4 gap-4 mt-6">
            <div class="text-center">
              <p class="text-xs text-[var(--text-muted)] mb-1">Best Case</p>
              <p class="text-lg font-semibold text-[var(--success)]">
                {{ formatPnl(Math.max(...filteredScenarios.filter(s => s.pnl !== null).map(s => s.pnl!))) }}
              </p>
            </div>
            <div class="text-center">
              <p class="text-xs text-[var(--text-muted)] mb-1">Worst Case</p>
              <p class="text-lg font-semibold text-[var(--danger)]">
                {{ formatPnl(Math.min(...filteredScenarios.filter(s => s.pnl !== null).map(s => s.pnl!))) }}
              </p>
            </div>
            <div class="text-center">
              <p class="text-xs text-[var(--text-muted)] mb-1">Average</p>
              <p class="text-lg font-semibold text-[var(--text-primary)]">
                {{ formatPnl(filteredScenarios.filter(s => s.pnl !== null).reduce((a, b) => a + (b.pnl ?? 0), 0) / filteredScenarios.filter(s => s.pnl !== null).length) }}
              </p>
            </div>
            <div class="text-center">
              <p class="text-xs text-[var(--text-muted)] mb-1">Scenarios</p>
              <p class="text-lg font-semibold text-[var(--text-primary)]">
                {{ filteredScenarios.filter(s => s.pnl !== null).length }}
              </p>
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
