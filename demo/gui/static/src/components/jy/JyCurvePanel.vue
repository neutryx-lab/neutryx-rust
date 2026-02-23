<script setup lang="ts">
import { ref, watch, onUnmounted } from 'vue';
import { Chart, registerables, type ChartConfiguration } from 'chart.js';
import { getChartColors } from '@/composables/useChartTheme';
import type { JyCurveBuildResponse } from '@/types/api';

Chart.register(...registerables);

const props = defineProps<{
  result: JyCurveBuildResponse | null;
}>();

const curveCanvas = ref<HTMLCanvasElement | null>(null);
const dfCanvas = ref<HTMLCanvasElement | null>(null);
let curveChart: Chart | null = null;
let dfChart: Chart | null = null;

watch(() => props.result, () => {
  if (props.result) {
    renderCurveChart();
    renderDfChart();
  }
}, { immediate: true });

onUnmounted(() => {
  curveChart?.destroy();
  dfChart?.destroy();
});

function renderCurveChart() {
  if (!curveCanvas.value || !props.result) return;
  curveChart?.destroy();

  const cc = getChartColors();
  const labels = props.result.nominalCurve.map(p => `${p.tenor}Y`);

  const config: ChartConfiguration<'line'> = {
    type: 'line',
    data: {
      labels,
      datasets: [
        {
          label: 'Nominal',
          data: props.result.nominalCurve.map(p => p.value * 100),
          borderColor: '#3b82f6',
          backgroundColor: '#3b82f61a',
          tension: 0.4,
          fill: false,
        },
        {
          label: 'Real',
          data: props.result.realCurve.map(p => p.value * 100),
          borderColor: '#10b981',
          backgroundColor: '#10b9811a',
          tension: 0.4,
          fill: false,
        },
        {
          label: 'Breakeven',
          data: props.result.breakevenCurve.map(p => p.value * 100),
          borderColor: '#f59e0b',
          backgroundColor: '#f59e0b1a',
          tension: 0.4,
          borderDash: [5, 5],
          fill: false,
        },
      ],
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      plugins: {
        legend: { display: true, position: 'top', labels: { color: cc.tick, usePointStyle: true } },
        tooltip: { backgroundColor: cc.tooltipBg, titleColor: cc.tooltipTitle, bodyColor: cc.tooltipBody },
      },
      scales: {
        x: { grid: { color: cc.grid }, ticks: { color: cc.tick }, title: { display: true, text: 'Tenor', color: cc.tick } },
        y: { grid: { color: cc.grid }, ticks: { color: cc.tick, callback: v => `${(v as number).toFixed(2)}%` }, title: { display: true, text: 'Rate (%)', color: cc.tick } },
      },
    },
  };

  curveChart = new Chart(curveCanvas.value, config);
}

function renderDfChart() {
  if (!dfCanvas.value || !props.result) return;
  dfChart?.destroy();

  const cc = getChartColors();
  const labels = props.result.nominalDf.map(p => `${p.tenor}Y`);

  const config: ChartConfiguration<'line'> = {
    type: 'line',
    data: {
      labels,
      datasets: [
        {
          label: 'Nominal DF',
          data: props.result.nominalDf.map(p => p.value),
          borderColor: '#3b82f6',
          tension: 0.4,
          fill: false,
        },
        {
          label: 'Real DF',
          data: props.result.realDf.map(p => p.value),
          borderColor: '#10b981',
          tension: 0.4,
          fill: false,
        },
      ],
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      plugins: {
        legend: { display: true, position: 'top', labels: { color: cc.tick, usePointStyle: true } },
        tooltip: { backgroundColor: cc.tooltipBg, titleColor: cc.tooltipTitle, bodyColor: cc.tooltipBody },
      },
      scales: {
        x: { grid: { color: cc.grid }, ticks: { color: cc.tick } },
        y: { grid: { color: cc.grid }, ticks: { color: cc.tick, callback: v => (v as number).toFixed(4) } },
      },
    },
  };

  dfChart = new Chart(dfCanvas.value, config);
}
</script>

<template>
  <div class="space-y-6">
    <div v-if="result" class="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <!-- Zero Rate Chart -->
      <div>
        <h4 class="text-sm font-semibold text-[var(--text-primary)] mb-3 flex items-center gap-2">
          <i class="fas fa-chart-line text-blue-500"></i>
          Zero Rate Curves
        </h4>
        <div class="h-64">
          <canvas ref="curveCanvas"></canvas>
        </div>
      </div>

      <!-- Discount Factor Chart -->
      <div>
        <h4 class="text-sm font-semibold text-[var(--text-primary)] mb-3 flex items-center gap-2">
          <i class="fas fa-chart-area text-green-500"></i>
          Discount Factors
        </h4>
        <div class="h-64">
          <canvas ref="dfCanvas"></canvas>
        </div>
      </div>
    </div>

    <!-- Curve Data Table -->
    <div v-if="result">
      <h4 class="text-sm font-semibold text-[var(--text-primary)] mb-3 flex items-center gap-2">
        <i class="fas fa-table text-[var(--primary)]"></i>
        Curve Data
      </h4>
      <div class="overflow-auto max-h-60">
        <table class="w-full text-xs">
          <thead>
            <tr class="text-[var(--text-muted)] border-b border-[var(--border)]">
              <th class="text-left py-2 px-2">Tenor</th>
              <th class="text-right py-2 px-2">Nominal (%)</th>
              <th class="text-right py-2 px-2">Real (%)</th>
              <th class="text-right py-2 px-2">Breakeven (%)</th>
              <th class="text-right py-2 px-2">Nominal DF</th>
              <th class="text-right py-2 px-2">Real DF</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(pt, i) in result.nominalCurve" :key="i" class="border-b border-[var(--border)] border-opacity-50">
              <td class="py-1.5 px-2">{{ pt.tenor.toFixed(1) }}Y</td>
              <td class="py-1.5 px-2 text-right text-blue-400">{{ (pt.value * 100).toFixed(3) }}</td>
              <td class="py-1.5 px-2 text-right text-green-400">{{ result.realCurve[i] ? (result.realCurve[i].value * 100).toFixed(3) : '-' }}</td>
              <td class="py-1.5 px-2 text-right text-yellow-400">{{ result.breakevenCurve[i] ? (result.breakevenCurve[i].value * 100).toFixed(3) : '-' }}</td>
              <td class="py-1.5 px-2 text-right">{{ result.nominalDf[i]?.value.toFixed(6) ?? '-' }}</td>
              <td class="py-1.5 px-2 text-right">{{ result.realDf[i]?.value.toFixed(6) ?? '-' }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <div v-else class="flex items-center justify-center h-40 text-[var(--text-muted)] text-sm">
      <i class="fas fa-info-circle mr-2"></i>Click "Curves" to build nominal, real, and breakeven curves
    </div>
  </div>
</template>
