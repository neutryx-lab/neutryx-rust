<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue';
import { Chart, registerables, type TooltipItem, type ChartConfiguration } from 'chart.js';

Chart.register(...registerables);

// Types
interface ExposureDataPoint {
  time: number;
  pfe: number;
  ee: number;
  epe: number;
  ene: number;
}

// State
const chart = ref<Chart | null>(null);
const chartCanvas = ref<HTMLCanvasElement | null>(null);
const visibleSeries = ref(new Set(['pfe', 'ee', 'epe', 'ene']));
const selectedMetric = ref<'all' | 'pfe' | 'ee' | 'epe' | 'ene'>('all');

// Series configuration
const seriesConfig = [
  { key: 'pfe', label: 'PFE', color: '#ef4444', description: 'Potential Future Exposure' },
  { key: 'ee', label: 'EE', color: '#3b82f6', description: 'Expected Exposure' },
  { key: 'epe', label: 'EPE', color: '#10b981', description: 'Expected Positive Exposure' },
  { key: 'ene', label: 'ENE', color: '#8b5cf6', description: 'Expected Negative Exposure' },
];

// Summary stats
const summaryStats = computed(() => [
  { label: 'Peak PFE', value: '$48.5M', subtitle: 'at 3.2Y', icon: 'fa-arrow-up', color: '#ef4444' },
  { label: 'Current EE', value: '$28.3M', subtitle: 'Avg: $22.1M', icon: 'fa-chart-area', color: '#3b82f6' },
  { label: 'EPE/ENE Ratio', value: '2.8x', subtitle: 'Target: 2.0x', icon: 'fa-balance-scale', color: '#10b981' },
  { label: 'Time to Peak', value: '3.2Y', subtitle: 'of 10Y horizon', icon: 'fa-clock', color: '#8b5cf6' },
]);

// Generate mock data
function generateExposureData(): ExposureDataPoint[] {
  const data: ExposureDataPoint[] = [];
  const maxTime = 10;
  const steps = 40;

  for (let i = 0; i <= steps; i++) {
    const time = (i / steps) * maxTime;
    const decay = Math.exp(-0.05 * time);
    const growth = 1 - Math.exp(-0.3 * time);
    const peak = Math.exp(-Math.pow(time - 3, 2) / 8);

    data.push({
      time,
      pfe: 48.5 * peak * (0.8 + 0.4 * Math.random()) * (1 - time / maxTime * 0.3),
      ee: 28.3 * growth * decay * (0.9 + 0.2 * Math.random()),
      epe: 22.3 * growth * decay * (0.9 + 0.2 * Math.random()),
      ene: -8.1 * growth * decay * (0.8 + 0.4 * Math.random()),
    });
  }

  return data;
}

// Render chart
function renderChart() {
  if (!chartCanvas.value) return;

  if (chart.value) {
    chart.value.destroy();
    chart.value = null;
  }

  const data = generateExposureData();
  const labels = data.map(d => `${d.time.toFixed(1)}Y`);

  const ctx = chartCanvas.value.getContext('2d');
  if (!ctx) return;

  const config: ChartConfiguration<'line'> = {
    type: 'line',
    data: {
      labels,
      datasets: seriesConfig.map(series => ({
        label: series.label,
        data: data.map(d => d[series.key as keyof ExposureDataPoint] as number),
        borderColor: series.color,
        backgroundColor: `${series.color}1a`,
        fill: true,
        tension: 0.4,
        hidden: !visibleSeries.value.has(series.key),
      })),
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      interaction: { intersect: false, mode: 'index' },
      plugins: {
        legend: { display: false },
        tooltip: {
          backgroundColor: 'rgba(0, 0, 0, 0.8)',
          titleColor: '#fff',
          bodyColor: '#fff',
          padding: 12,
          cornerRadius: 8,
          callbacks: {
            label: (context: TooltipItem<'line'>) => {
              const value = context.parsed.y;
              return `${context.dataset.label ?? ''}: $${value?.toFixed(1) ?? '0'}M`;
            },
          },
        },
      },
      scales: {
        x: {
          grid: { color: 'rgba(255, 255, 255, 0.05)' },
          ticks: { color: '#94a3b8' },
        },
        y: {
          beginAtZero: false,
          grid: { color: 'rgba(255, 255, 255, 0.05)' },
          ticks: {
            color: '#94a3b8',
            callback: (value) => `$${value}M`,
          },
        },
      },
    },
  };

  chart.value = new Chart(ctx, config);
}

// Toggle metric filter
function selectMetric(metric: typeof selectedMetric.value) {
  selectedMetric.value = metric;
  if (metric === 'all') {
    visibleSeries.value = new Set(['pfe', 'ee', 'epe', 'ene']);
  } else {
    visibleSeries.value = new Set([metric]);
  }
  updateChartVisibility();
}

// Toggle individual series
function toggleSeries(key: string) {
  if (visibleSeries.value.has(key)) {
    if (visibleSeries.value.size > 1) {
      visibleSeries.value.delete(key);
    }
  } else {
    visibleSeries.value.add(key);
  }
  selectedMetric.value = visibleSeries.value.size === 4 ? 'all' :
    visibleSeries.value.size === 1 ? [...visibleSeries.value][0] as typeof selectedMetric.value : 'all';
  updateChartVisibility();
}

function updateChartVisibility() {
  if (!chart.value) return;
  chart.value.data.datasets.forEach((dataset, index) => {
    dataset.hidden = !visibleSeries.value.has(seriesConfig[index].key);
  });
  chart.value.update();
}

onMounted(() => renderChart());
onUnmounted(() => {
  if (chart.value) {
    chart.value.destroy();
    chart.value = null;
  }
});
</script>

<template>
  <div class="exposure-view">
    <!-- Summary Stats -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
      <div
        v-for="stat in summaryStats"
        :key="stat.label"
        class="glass-card p-4"
      >
        <div class="flex items-start justify-between">
          <div>
            <p class="text-sm text-[var(--text-muted)] mb-1">{{ stat.label }}</p>
            <p class="text-2xl font-semibold text-[var(--text-primary)]">{{ stat.value }}</p>
            <p class="text-xs text-[var(--text-muted)] mt-1">{{ stat.subtitle }}</p>
          </div>
          <div
            class="w-10 h-10 rounded-lg flex items-center justify-center"
            :style="{ backgroundColor: `${stat.color}1a` }"
          >
            <i :class="['fas', stat.icon]" :style="{ color: stat.color }"></i>
          </div>
        </div>
      </div>
    </div>

    <!-- Chart Card -->
    <div class="glass-card p-6">
      <div class="flex items-center justify-between mb-6">
        <div>
          <h3 class="text-lg font-semibold text-[var(--text-primary)]">Exposure Profile</h3>
          <p class="text-sm text-[var(--text-muted)]">Portfolio exposure over 10Y horizon</p>
        </div>

        <!-- Metric Toggle Buttons -->
        <div class="flex gap-2">
          <button
            v-for="metric in ['all', 'pfe', 'ee', 'epe', 'ene'] as const"
            :key="metric"
            :class="[
              'px-3 py-1.5 rounded-lg text-sm font-medium transition-all duration-200',
              selectedMetric === metric
                ? 'bg-[var(--primary)] text-white'
                : 'text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]'
            ]"
            @click="selectMetric(metric)"
          >
            {{ metric === 'all' ? 'All' : metric.toUpperCase() }}
          </button>
        </div>
      </div>

      <!-- Chart Legend -->
      <div class="flex flex-wrap gap-4 mb-4">
        <button
          v-for="series in seriesConfig"
          :key="series.key"
          :class="[
            'flex items-center gap-2 px-3 py-1.5 rounded-lg transition-all duration-200',
            visibleSeries.has(series.key)
              ? 'bg-[var(--surface)] text-[var(--text-primary)]'
              : 'text-[var(--text-muted)] opacity-50'
          ]"
          :title="series.description"
          @click="toggleSeries(series.key)"
        >
          <span
            class="w-3 h-3 rounded-full"
            :style="{ backgroundColor: series.color }"
          ></span>
          <span class="text-sm font-medium">{{ series.label }}</span>
        </button>
      </div>

      <!-- Chart -->
      <div class="h-96">
        <canvas ref="chartCanvas"></canvas>
      </div>
    </div>

    <!-- Additional Info Cards -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6 mt-6">
      <div class="glass-card p-6">
        <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Exposure Breakdown</h3>
        <div class="space-y-3">
          <div v-for="series in seriesConfig" :key="series.key" class="flex items-center justify-between">
            <div class="flex items-center gap-2">
              <span class="w-3 h-3 rounded-full" :style="{ backgroundColor: series.color }"></span>
              <span class="text-sm text-[var(--text-secondary)]">{{ series.description }}</span>
            </div>
            <span class="text-sm font-medium text-[var(--text-primary)]">
              {{ series.key === 'pfe' ? '$48.5M' : series.key === 'ee' ? '$28.3M' : series.key === 'epe' ? '$22.3M' : '-$8.1M' }}
            </span>
          </div>
        </div>
      </div>

      <div class="glass-card p-6">
        <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Risk Metrics</h3>
        <div class="space-y-3">
          <div class="flex items-center justify-between">
            <span class="text-sm text-[var(--text-secondary)]">VaR (99%, 10d)</span>
            <span class="text-sm font-medium text-[var(--text-primary)]">$12.4M</span>
          </div>
          <div class="flex items-center justify-between">
            <span class="text-sm text-[var(--text-secondary)]">Expected Shortfall</span>
            <span class="text-sm font-medium text-[var(--text-primary)]">$18.7M</span>
          </div>
          <div class="flex items-center justify-between">
            <span class="text-sm text-[var(--text-secondary)]">Stress Loss</span>
            <span class="text-sm font-medium text-[var(--danger)]">-$34.2M</span>
          </div>
          <div class="flex items-center justify-between">
            <span class="text-sm text-[var(--text-secondary)]">Capital Requirement</span>
            <span class="text-sm font-medium text-[var(--text-primary)]">$156.8M</span>
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
