<script setup lang="ts">
import { ref, watch, onUnmounted } from 'vue';
import { Chart, registerables, type ChartConfiguration } from 'chart.js';
import { getChartColors } from '@/composables/useChartTheme';
import { useJyInflationStore } from '@/stores/jyInflation';
import type { JySimulationResponse } from '@/types/api';

Chart.register(...registerables);

const props = defineProps<{
  result: JySimulationResponse | null;
}>();

const store = useJyInflationStore();
const nominalCanvas = ref<HTMLCanvasElement | null>(null);
const realCanvas = ref<HTMLCanvasElement | null>(null);
const indexCanvas = ref<HTMLCanvasElement | null>(null);
let nominalChart: Chart | null = null;
let realChart: Chart | null = null;
let indexChart: Chart | null = null;

watch(() => props.result, () => {
  if (props.result) {
    renderFanChart(nominalCanvas.value, props.result.timeGrid, props.result.nominalRate, props.result.samplePaths.map(p => p.nominalRate), 'Nominal Rate', '#3b82f6', (c) => { nominalChart = c; });
    renderFanChart(realCanvas.value, props.result.timeGrid, props.result.realRate, props.result.samplePaths.map(p => p.realRate), 'Real Rate', '#10b981', (c) => { realChart = c; });
    renderFanChart(indexCanvas.value, props.result.timeGrid, props.result.inflationIndex, props.result.samplePaths.map(p => p.inflationIndex), 'Inflation Index', '#f59e0b', (c) => { indexChart = c; });
  }
}, { immediate: true });

onUnmounted(() => {
  nominalChart?.destroy();
  realChart?.destroy();
  indexChart?.destroy();
});

function renderFanChart(
  canvas: HTMLCanvasElement | null,
  timeGrid: number[],
  stats: { mean: number[]; percentile5: number[]; percentile25: number[]; percentile75: number[]; percentile95: number[] },
  samplePaths: number[][],
  label: string,
  color: string,
  setter: (c: Chart) => void,
) {
  if (!canvas) return;

  const cc = getChartColors();
  const labels = timeGrid.map(t => `${t.toFixed(2)}Y`);

  const datasets: ChartConfiguration<'line'>['data']['datasets'] = [
    // Fan: 5-95 percentile band
    {
      label: '5th-95th',
      data: stats.percentile95,
      borderColor: 'transparent',
      backgroundColor: `${color}15`,
      fill: '+1',
      tension: 0.3,
      pointRadius: 0,
    },
    {
      label: '',
      data: stats.percentile5,
      borderColor: 'transparent',
      backgroundColor: 'transparent',
      fill: false,
      tension: 0.3,
      pointRadius: 0,
    },
    // Fan: 25-75 percentile band
    {
      label: '25th-75th',
      data: stats.percentile75,
      borderColor: 'transparent',
      backgroundColor: `${color}30`,
      fill: '+1',
      tension: 0.3,
      pointRadius: 0,
    },
    {
      label: '',
      data: stats.percentile25,
      borderColor: 'transparent',
      backgroundColor: 'transparent',
      fill: false,
      tension: 0.3,
      pointRadius: 0,
    },
    // Mean
    {
      label: `${label} (Mean)`,
      data: stats.mean,
      borderColor: color,
      backgroundColor: 'transparent',
      borderWidth: 2,
      fill: false,
      tension: 0.3,
      pointRadius: 0,
    },
  ];

  // Sample paths (thin, semi-transparent)
  samplePaths.forEach((path, i) => {
    datasets.push({
      label: i === 0 ? 'Sample Paths' : '',
      data: path,
      borderColor: `${color}50`,
      backgroundColor: 'transparent',
      borderWidth: 0.5,
      fill: false,
      tension: 0.3,
      pointRadius: 0,
    });
  });

  const config: ChartConfiguration<'line'> = {
    type: 'line',
    data: { labels, datasets },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      interaction: { intersect: false, mode: 'index' },
      plugins: {
        legend: { display: true, position: 'top', labels: { color: cc.tick, usePointStyle: true, filter: (item) => !!item.text } },
        tooltip: { backgroundColor: cc.tooltipBg, titleColor: cc.tooltipTitle, bodyColor: cc.tooltipBody },
      },
      scales: {
        x: { grid: { color: cc.grid }, ticks: { color: cc.tick, maxTicksLimit: 10 } },
        y: { grid: { color: cc.grid }, ticks: { color: cc.tick } },
      },
    },
  };

  setter(new Chart(canvas, config));
}
</script>

<template>
  <div class="space-y-6">
    <!-- Simulation Config -->
    <div class="grid grid-cols-2 md:grid-cols-4 gap-4 mb-4">
      <div>
        <label class="text-xs text-[var(--text-muted)] mb-1 block">MC Paths</label>
        <input v-model.number="store.numPaths" type="number" min="100" max="100000"
          class="w-full px-3 py-1.5 text-sm border border-[var(--border)] rounded-lg bg-[var(--surface)] text-[var(--text-primary)]" />
      </div>
      <div>
        <label class="text-xs text-[var(--text-muted)] mb-1 block">Time Steps</label>
        <input v-model.number="store.numSteps" type="number" min="10" max="5000"
          class="w-full px-3 py-1.5 text-sm border border-[var(--border)] rounded-lg bg-[var(--surface)] text-[var(--text-primary)]" />
      </div>
      <div>
        <label class="text-xs text-[var(--text-muted)] mb-1 block">Horizon (years)</label>
        <input v-model.number="store.horizon" type="number" min="0.1" max="50" step="0.5"
          class="w-full px-3 py-1.5 text-sm border border-[var(--border)] rounded-lg bg-[var(--surface)] text-[var(--text-primary)]" />
      </div>
      <div>
        <label class="text-xs text-[var(--text-muted)] mb-1 block">Sample Paths</label>
        <input v-model.number="store.numSamplePaths" type="number" min="0" max="20"
          class="w-full px-3 py-1.5 text-sm border border-[var(--border)] rounded-lg bg-[var(--surface)] text-[var(--text-primary)]" />
      </div>
    </div>

    <!-- Fan Charts -->
    <div v-if="result" class="space-y-6">
      <div>
        <h4 class="text-sm font-semibold text-[var(--text-primary)] mb-2 flex items-center gap-2">
          <i class="fas fa-chart-area text-blue-500"></i>
          Nominal Short Rate (n<sub>t</sub>)
        </h4>
        <div class="h-56"><canvas ref="nominalCanvas"></canvas></div>
      </div>

      <div>
        <h4 class="text-sm font-semibold text-[var(--text-primary)] mb-2 flex items-center gap-2">
          <i class="fas fa-chart-area text-green-500"></i>
          Real Short Rate (r<sub>t</sub>)
        </h4>
        <div class="h-56"><canvas ref="realCanvas"></canvas></div>
      </div>

      <div>
        <h4 class="text-sm font-semibold text-[var(--text-primary)] mb-2 flex items-center gap-2">
          <i class="fas fa-chart-area text-yellow-500"></i>
          Inflation Index (I<sub>t</sub>)
        </h4>
        <div class="h-56"><canvas ref="indexCanvas"></canvas></div>
      </div>

      <!-- Realized Correlation + PSD -->
      <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
        <div>
          <h4 class="text-sm font-semibold text-[var(--text-primary)] mb-2 flex items-center gap-2">
            <i class="fas fa-th text-purple-500"></i>
            Realized Correlation
          </h4>
          <div class="grid grid-cols-4 gap-1 text-xs text-center">
            <div></div>
            <div class="font-semibold text-[var(--text-muted)]">n</div>
            <div class="font-semibold text-[var(--text-muted)]">r</div>
            <div class="font-semibold text-[var(--text-muted)]">I</div>
            <div class="font-semibold text-[var(--text-muted)]">n</div>
            <div class="p-2 rounded bg-blue-500/20 text-blue-400">1.00</div>
            <div class="p-2 rounded" :class="corrColor(result.correlationRealized.rhoNr)">{{ result.correlationRealized.rhoNr.toFixed(3) }}</div>
            <div class="p-2 rounded" :class="corrColor(result.correlationRealized.rhoNi)">{{ result.correlationRealized.rhoNi.toFixed(3) }}</div>
            <div class="font-semibold text-[var(--text-muted)]">r</div>
            <div class="p-2 rounded" :class="corrColor(result.correlationRealized.rhoNr)">{{ result.correlationRealized.rhoNr.toFixed(3) }}</div>
            <div class="p-2 rounded bg-green-500/20 text-green-400">1.00</div>
            <div class="p-2 rounded" :class="corrColor(result.correlationRealized.rhoRi)">{{ result.correlationRealized.rhoRi.toFixed(3) }}</div>
            <div class="font-semibold text-[var(--text-muted)]">I</div>
            <div class="p-2 rounded" :class="corrColor(result.correlationRealized.rhoNi)">{{ result.correlationRealized.rhoNi.toFixed(3) }}</div>
            <div class="p-2 rounded" :class="corrColor(result.correlationRealized.rhoRi)">{{ result.correlationRealized.rhoRi.toFixed(3) }}</div>
            <div class="p-2 rounded bg-yellow-500/20 text-yellow-400">1.00</div>
          </div>
        </div>
        <div>
          <h4 class="text-sm font-semibold text-[var(--text-primary)] mb-2 flex items-center gap-2">
            <i class="fas fa-check-circle text-green-500"></i>
            PSD Status
          </h4>
          <div class="p-4 rounded-lg" :class="result.psdEnforced ? 'bg-yellow-500/10 border border-yellow-500/30' : 'bg-green-500/10 border border-green-500/30'">
            <div class="text-sm font-medium" :class="result.psdEnforced ? 'text-yellow-400' : 'text-green-400'">
              <i :class="['fas mr-2', result.psdEnforced ? 'fa-exclamation-triangle' : 'fa-check']"></i>
              {{ result.psdEnforced ? 'PSD Enforcement Applied' : 'Matrix is Positive Definite' }}
            </div>
            <div class="text-xs text-[var(--text-muted)] mt-1">
              {{ result.psdEnforced ? 'Correlation matrix was shrunk towards identity to ensure positive definiteness.' : 'No correction needed.' }}
            </div>
          </div>
        </div>
      </div>
    </div>

    <div v-else class="flex items-center justify-center h-40 text-[var(--text-muted)] text-sm">
      <i class="fas fa-info-circle mr-2"></i>Click "Simulation" to run Monte Carlo simulation
    </div>
  </div>
</template>

<script lang="ts">
function corrColor(rho: number): string {
  if (rho > 0.3) return 'bg-blue-500/20 text-blue-400';
  if (rho < -0.3) return 'bg-red-500/20 text-red-400';
  return 'bg-gray-500/20 text-[var(--text-muted)]';
}
</script>
