<script setup lang="ts">
import { ref, computed, watch, onUnmounted } from 'vue';
import { Chart, registerables, type ChartConfiguration } from 'chart.js';
import { getChartColors } from '@/composables/useChartTheme';
import type { JyPricingResponse } from '@/types/api';

Chart.register(...registerables);

const props = defineProps<{
  result: JyPricingResponse | null;
}>();

const legCanvas = ref<HTMLCanvasElement | null>(null);
let legChart: Chart | null = null;

watch(() => props.result, () => {
  if (props.result) renderLegChart();
}, { immediate: true });

onUnmounted(() => {
  legChart?.destroy();
});

const greeksRows = computed(() => {
  if (!props.result) return [];
  const g = props.result.greeks;
  return [
    { label: 'DV01 Nominal', description: 'Sensitivity to nominal curve (1bp)', value: g.dv01Nominal },
    { label: 'DV01 Real', description: 'Sensitivity to real curve (1bp)', value: g.dv01Real },
    { label: 'Vega Nominal', description: 'Sensitivity to nominal vol (1%)', value: g.vegaNominal },
    { label: 'Vega Real', description: 'Sensitivity to real vol (1%)', value: g.vegaReal },
    { label: 'Vega Inflation', description: 'Sensitivity to inflation vol (1%)', value: g.vegaInflation },
    { label: 'Theta', description: 'Time decay (1 day)', value: g.theta },
  ];
});

function renderLegChart() {
  if (!legCanvas.value || !props.result) return;
  legChart?.destroy();

  const cc = getChartColors();

  const config: ChartConfiguration<'bar'> = {
    type: 'bar',
    data: {
      labels: ['Inflation Leg', 'Fixed Leg', 'Net MtM'],
      datasets: [{
        data: [
          props.result.inflationLegPv,
          -props.result.fixedLegPv,
          props.result.mtm,
        ],
        backgroundColor: [
          '#10b981',
          '#ef4444',
          props.result.mtm >= 0 ? '#3b82f6' : '#f59e0b',
        ],
        borderRadius: 6,
      }],
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      indexAxis: 'y',
      plugins: {
        legend: { display: false },
        tooltip: {
          backgroundColor: cc.tooltipBg,
          titleColor: cc.tooltipTitle,
          bodyColor: cc.tooltipBody,
          callbacks: {
            label: (ctx) => formatCcy(ctx.parsed.x ?? 0),
          },
        },
      },
      scales: {
        x: { grid: { color: cc.grid }, ticks: { color: cc.tick, callback: v => formatCcyShort(v as number) } },
        y: { grid: { display: false }, ticks: { color: cc.tick } },
      },
    },
  };

  legChart = new Chart(legCanvas.value, config);
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
    <div v-if="result" class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <!-- MtM Display -->
      <div class="space-y-4">
        <h4 class="text-sm font-semibold text-[var(--text-primary)] flex items-center gap-2">
          <i class="fas fa-dollar-sign text-[var(--primary)]"></i>
          Mark-to-Market
        </h4>

        <div class="p-6 rounded-xl text-center" :class="result.mtm >= 0 ? 'bg-green-500/10 border border-green-500/30' : 'bg-red-500/10 border border-red-500/30'">
          <div class="text-3xl font-bold" :class="result.mtm >= 0 ? 'text-green-400' : 'text-red-400'">
            {{ formatCcy(result.mtm) }}
          </div>
          <div class="text-xs text-[var(--text-muted)] mt-1">ZCIS Analytical Price</div>
        </div>

        <div class="grid grid-cols-2 gap-3">
          <div class="p-3 rounded-lg bg-[var(--surface-hover)]">
            <div class="text-xs text-[var(--text-muted)]">Inflation Leg</div>
            <div class="text-sm font-semibold text-green-400">{{ formatCcy(result.inflationLegPv) }}</div>
          </div>
          <div class="p-3 rounded-lg bg-[var(--surface-hover)]">
            <div class="text-xs text-[var(--text-muted)]">Fixed Leg</div>
            <div class="text-sm font-semibold text-red-400">{{ formatCcy(result.fixedLegPv) }}</div>
          </div>
        </div>
      </div>

      <!-- Leg Chart -->
      <div>
        <h4 class="text-sm font-semibold text-[var(--text-primary)] mb-3 flex items-center gap-2">
          <i class="fas fa-chart-bar text-blue-500"></i>
          Leg Decomposition
        </h4>
        <div class="h-48"><canvas ref="legCanvas"></canvas></div>
      </div>

      <!-- Greeks Table -->
      <div class="space-y-4">
        <h4 class="text-sm font-semibold text-[var(--text-primary)] flex items-center gap-2">
          <i class="fas fa-wave-square text-purple-500"></i>
          Risk Sensitivities (Greeks)
        </h4>

        <div class="space-y-2">
          <div v-for="greek in greeksRows" :key="greek.label"
            class="flex items-center justify-between p-2.5 rounded-lg bg-[var(--surface-hover)]">
            <div>
              <div class="text-xs font-medium text-[var(--text-primary)]">{{ greek.label }}</div>
              <div class="text-[10px] text-[var(--text-muted)]">{{ greek.description }}</div>
            </div>
            <div class="text-sm font-mono font-semibold" :class="greek.value >= 0 ? 'text-green-400' : 'text-red-400'">
              {{ formatCcy(greek.value) }}
            </div>
          </div>
        </div>
      </div>
    </div>

    <div v-else class="flex items-center justify-center h-40 text-[var(--text-muted)] text-sm">
      <i class="fas fa-info-circle mr-2"></i>Click "Pricing" to compute analytical ZCIS price and Greeks
    </div>
  </div>
</template>
