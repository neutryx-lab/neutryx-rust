<script setup lang="ts">
import { ref, watch, onUnmounted } from 'vue';
import { Chart, registerables, type ChartConfiguration, type TooltipItem } from 'chart.js';
import { getChartColors } from '@/composables/useChartTheme';
import { useJyInflationStore } from '@/stores/jyInflation';
import type { JyXvaResponse } from '@/types/api';

Chart.register(...registerables);

const props = defineProps<{
  result: JyXvaResponse | null;
}>();

const store = useJyInflationStore();
const exposureCanvas = ref<HTMLCanvasElement | null>(null);
let exposureChart: Chart | null = null;

watch(() => props.result, () => {
  if (props.result) renderExposureChart();
}, { immediate: true });

onUnmounted(() => {
  exposureChart?.destroy();
});

function renderExposureChart() {
  if (!exposureCanvas.value || !props.result) return;
  exposureChart?.destroy();

  const cc = getChartColors();
  const ep = props.result.exposureProfile;
  const labels = ep.timeGrid.map(t => `${t.toFixed(2)}Y`);

  const config: ChartConfiguration<'line'> = {
    type: 'line',
    data: {
      labels,
      datasets: [
        {
          label: 'EE',
          data: ep.expectedExposure,
          borderColor: '#3b82f6',
          backgroundColor: '#3b82f61a',
          fill: true,
          tension: 0.4,
        },
        {
          label: 'ENE',
          data: ep.negativeExpectedExposure.map(v => -v),
          borderColor: '#8b5cf6',
          backgroundColor: '#8b5cf61a',
          fill: true,
          tension: 0.4,
        },
        {
          label: 'PFE 95%',
          data: ep.pfe95,
          borderColor: '#ef4444',
          backgroundColor: '#ef44441a',
          fill: false,
          tension: 0.4,
          borderDash: [5, 5],
        },
        {
          label: 'PFE 99%',
          data: ep.pfe99,
          borderColor: '#f59e0b',
          backgroundColor: '#f59e0b1a',
          fill: false,
          tension: 0.4,
          borderDash: [3, 3],
        },
      ],
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      interaction: { intersect: false, mode: 'index' },
      plugins: {
        legend: { display: true, position: 'top', labels: { color: cc.tick, usePointStyle: true } },
        tooltip: {
          backgroundColor: cc.tooltipBg,
          titleColor: cc.tooltipTitle,
          bodyColor: cc.tooltipBody,
          padding: 12,
          callbacks: {
            label: (context: TooltipItem<'line'>) => `${context.dataset.label ?? ''}: ${formatCcy(context.parsed.y ?? 0)}`,
          },
        },
      },
      scales: {
        x: { grid: { color: cc.grid }, ticks: { color: cc.tick, maxTicksLimit: 10 } },
        y: { grid: { color: cc.grid }, ticks: { color: cc.tick, callback: v => formatCcyShort(v as number) } },
      },
    },
  };

  exposureChart = new Chart(exposureCanvas.value, config);
}

function formatCcy(v: number): string {
  const abs = Math.abs(v);
  const sign = v < 0 ? '-' : '';
  if (abs >= 1e6) return `${sign}$${(abs / 1e6).toFixed(2)}M`;
  if (abs >= 1e3) return `${sign}$${(abs / 1e3).toFixed(1)}K`;
  return `${sign}$${abs.toFixed(2)}`;
}

function formatCcyShort(v: number): string {
  const abs = Math.abs(v);
  const sign = v < 0 ? '-' : '';
  if (abs >= 1e6) return `${sign}${(abs / 1e6).toFixed(1)}M`;
  if (abs >= 1e3) return `${sign}${(abs / 1e3).toFixed(0)}K`;
  return `${sign}${abs.toFixed(0)}`;
}
</script>

<template>
  <div class="space-y-6">
    <!-- Credit Parameters -->
    <div class="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-7 gap-4">
      <div>
        <label class="text-xs text-[var(--text-muted)] mb-1 block">Cpty PD</label>
        <input v-model.number="store.counterpartyPd" type="number" step="0.001" min="0" max="1"
          class="w-full px-3 py-1.5 text-sm border border-[var(--border)] rounded-lg bg-[var(--surface)] text-[var(--text-primary)]" />
      </div>
      <div>
        <label class="text-xs text-[var(--text-muted)] mb-1 block">Cpty Recovery</label>
        <input v-model.number="store.counterpartyRecovery" type="number" step="0.05" min="0" max="1"
          class="w-full px-3 py-1.5 text-sm border border-[var(--border)] rounded-lg bg-[var(--surface)] text-[var(--text-primary)]" />
      </div>
      <div>
        <label class="text-xs text-[var(--text-muted)] mb-1 block">Own PD</label>
        <input v-model.number="store.ownPd" type="number" step="0.001" min="0" max="1"
          class="w-full px-3 py-1.5 text-sm border border-[var(--border)] rounded-lg bg-[var(--surface)] text-[var(--text-primary)]" />
      </div>
      <div>
        <label class="text-xs text-[var(--text-muted)] mb-1 block">Own Recovery</label>
        <input v-model.number="store.ownRecovery" type="number" step="0.05" min="0" max="1"
          class="w-full px-3 py-1.5 text-sm border border-[var(--border)] rounded-lg bg-[var(--surface)] text-[var(--text-primary)]" />
      </div>
      <div>
        <label class="text-xs text-[var(--text-muted)] mb-1 block">Funding Spread</label>
        <input v-model.number="store.fundingSpread" type="number" step="0.001"
          class="w-full px-3 py-1.5 text-sm border border-[var(--border)] rounded-lg bg-[var(--surface)] text-[var(--text-primary)]" />
      </div>
      <div>
        <label class="text-xs text-[var(--text-muted)] mb-1 block">MC Paths</label>
        <input v-model.number="store.xvaNumPaths" type="number" min="100" max="100000"
          class="w-full px-3 py-1.5 text-sm border border-[var(--border)] rounded-lg bg-[var(--surface)] text-[var(--text-primary)]" />
      </div>
      <div>
        <label class="text-xs text-[var(--text-muted)] mb-1 block">Time Steps</label>
        <input v-model.number="store.xvaNumSteps" type="number" min="10" max="5000"
          class="w-full px-3 py-1.5 text-sm border border-[var(--border)] rounded-lg bg-[var(--surface)] text-[var(--text-primary)]" />
      </div>
    </div>

    <div v-if="result" class="space-y-6">
      <!-- XVA Summary Cards -->
      <div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-3">
        <div class="p-4 rounded-xl bg-red-500/10 border border-red-500/20 text-center">
          <div class="text-xs text-[var(--text-muted)]">CVA</div>
          <div class="text-lg font-bold text-red-400">{{ formatCcy(result.cva) }}</div>
        </div>
        <div class="p-4 rounded-xl bg-blue-500/10 border border-blue-500/20 text-center">
          <div class="text-xs text-[var(--text-muted)]">DVA</div>
          <div class="text-lg font-bold text-blue-400">{{ formatCcy(result.dva) }}</div>
        </div>
        <div class="p-4 rounded-xl bg-yellow-500/10 border border-yellow-500/20 text-center">
          <div class="text-xs text-[var(--text-muted)]">FVA</div>
          <div class="text-lg font-bold text-yellow-400">{{ formatCcy(result.fva) }}</div>
        </div>
        <div class="p-4 rounded-xl bg-purple-500/10 border border-purple-500/20 text-center">
          <div class="text-xs text-[var(--text-muted)]">Total XVA</div>
          <div class="text-lg font-bold text-purple-400">{{ formatCcy(result.totalXva) }}</div>
        </div>
        <div class="p-4 rounded-xl bg-[var(--surface-hover)] text-center">
          <div class="text-xs text-[var(--text-muted)]">Clean MtM</div>
          <div class="text-lg font-bold text-[var(--text-primary)]">{{ formatCcy(result.cleanMtm) }}</div>
        </div>
        <div class="p-4 rounded-xl bg-green-500/10 border border-green-500/20 text-center">
          <div class="text-xs text-[var(--text-muted)]">Adjusted MtM</div>
          <div class="text-lg font-bold text-green-400">{{ formatCcy(result.adjustedMtm) }}</div>
        </div>
      </div>

      <!-- Exposure Profile Chart -->
      <div>
        <h4 class="text-sm font-semibold text-[var(--text-primary)] mb-3 flex items-center gap-2">
          <i class="fas fa-chart-area text-blue-500"></i>
          Exposure Profile
        </h4>
        <div class="h-72"><canvas ref="exposureCanvas"></canvas></div>
      </div>
    </div>

    <div v-else class="flex items-center justify-center h-40 text-[var(--text-muted)] text-sm">
      <i class="fas fa-info-circle mr-2"></i>Configure credit parameters and click "XVA" to compute CVA/DVA/FVA
    </div>
  </div>
</template>
