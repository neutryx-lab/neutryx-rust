<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue';
import { Chart, registerables, type TooltipItem, type ChartConfiguration } from 'chart.js';

// Register Chart.js components
Chart.register(...registerables);

// State
const exposureChart = ref<Chart | null>(null);
const visibleSeries = ref(new Set(['pfe', 'ee', 'epe', 'ene']));
const selectedRange = ref<'1M' | '3M' | '6M' | '1Y' | '5Y' | '10Y'>('10Y');
const chartCanvas = ref<HTMLCanvasElement | null>(null);

// Series configuration
const seriesConfig = [
  { key: 'pfe', label: 'PFE', color: '#ef4444', description: 'Potential Future Exposure' },
  { key: 'ee', label: 'EE', color: '#3b82f6', description: 'Expected Exposure' },
  { key: 'epe', label: 'EPE', color: '#10b981', description: 'Expected Positive Exposure' },
  { key: 'ene', label: 'ENE', color: '#8b5cf6', description: 'Expected Negative Exposure' },
];

// Stats cards data
const statsCards = computed(() => [
  { label: 'Total Notional', value: '$2.4B', change: '+12.5%', positive: true, icon: 'fa-dollar-sign' },
  { label: 'Peak PFE', value: '$48.5M', change: '-3.2%', positive: false, icon: 'fa-chart-line' },
  { label: 'Active Trades', value: '1,247', change: '+28', positive: true, icon: 'fa-exchange-alt' },
  { label: 'CVA', value: '$12.3M', change: '-5.1%', positive: true, icon: 'fa-shield-alt' },
]);

// Mock data generation
function generateExposureData() {
  const data: { time: number; pfe: number; ee: number; epe: number; ene: number }[] = [];
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

  // Destroy existing chart
  if (exposureChart.value) {
    exposureChart.value.destroy();
    exposureChart.value = null;
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
        data: data.map(d => d[series.key as keyof typeof d] as number),
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
      interaction: {
        intersect: false,
        mode: 'index',
      },
      plugins: {
        legend: {
          display: false,
        },
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
          grid: {
            color: 'rgba(255, 255, 255, 0.05)',
          },
          ticks: {
            color: '#94a3b8',
            maxTicksLimit: 10,
          },
        },
        y: {
          grid: {
            color: 'rgba(255, 255, 255, 0.05)',
          },
          ticks: {
            color: '#94a3b8',
            callback: (value) => `$${value}M`,
          },
        },
      },
    },
  };

  exposureChart.value = new Chart(ctx, config);
}

// Toggle series visibility
function toggleSeries(key: string) {
  if (visibleSeries.value.has(key)) {
    if (visibleSeries.value.size > 1) {
      visibleSeries.value.delete(key);
    }
  } else {
    visibleSeries.value.add(key);
  }
  updateChartVisibility();
}

function updateChartVisibility() {
  if (!exposureChart.value) return;

  exposureChart.value.data.datasets.forEach((dataset, index) => {
    dataset.hidden = !visibleSeries.value.has(seriesConfig[index].key);
  });

  exposureChart.value.update();
}

// Change time range
function setRange(range: typeof selectedRange.value) {
  selectedRange.value = range;
  renderChart();
}

// Lifecycle
onMounted(() => {
  renderChart();
});

onUnmounted(() => {
  if (exposureChart.value) {
    exposureChart.value.destroy();
    exposureChart.value = null;
  }
});
</script>

<template>
  <div class="dashboard-view">
    <!-- Stats Cards -->
    <div class="stats-grid grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
      <div
        v-for="stat in statsCards"
        :key="stat.label"
        class="stat-card glass-card p-4"
      >
        <div class="flex items-start justify-between">
          <div>
            <p class="text-sm text-[var(--text-muted)] mb-1">{{ stat.label }}</p>
            <p class="text-2xl font-semibold text-[var(--text-primary)]">{{ stat.value }}</p>
          </div>
          <div class="stat-icon w-10 h-10 rounded-lg bg-[var(--primary)]/10 flex items-center justify-center">
            <i :class="['fas', stat.icon, 'text-[var(--primary)]']"></i>
          </div>
        </div>
        <div class="mt-2 flex items-center gap-1 text-sm">
          <span :class="stat.positive ? 'text-[var(--success)]' : 'text-[var(--danger)]'">
            {{ stat.change }}
          </span>
          <span class="text-[var(--text-muted)]">vs last month</span>
        </div>
      </div>
    </div>

    <!-- Exposure Chart Card -->
    <div class="chart-card glass-card p-6">
      <div class="chart-header flex items-center justify-between mb-6">
        <div>
          <h3 class="text-lg font-semibold text-[var(--text-primary)]">Exposure Profile</h3>
          <p class="text-sm text-[var(--text-muted)]">Portfolio exposure over time</p>
        </div>

        <!-- Time Range Selector -->
        <div class="range-selector flex gap-2">
          <button
            v-for="range in ['1M', '3M', '6M', '1Y', '5Y', '10Y'] as const"
            :key="range"
            :class="[
              'px-3 py-1.5 rounded-lg text-sm font-medium transition-all duration-200',
              selectedRange === range
                ? 'bg-[var(--primary)] text-white'
                : 'text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]'
            ]"
            @click="setRange(range)"
          >
            {{ range }}
          </button>
        </div>
      </div>

      <!-- Chart Legend -->
      <div class="chart-legend flex flex-wrap gap-4 mb-4">
        <button
          v-for="series in seriesConfig"
          :key="series.key"
          :class="[
            'legend-item flex items-center gap-2 px-3 py-1.5 rounded-lg transition-all duration-200',
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

      <!-- Chart Canvas -->
      <div class="chart-container h-80">
        <canvas ref="chartCanvas"></canvas>
      </div>
    </div>

    <!-- Additional Cards Row -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6 mt-6">
      <!-- Recent Activity -->
      <div class="glass-card p-6">
        <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Recent Activity</h3>
        <div class="space-y-3">
          <div v-for="i in 4" :key="i" class="flex items-center gap-3 p-3 rounded-lg hover:bg-[var(--surface-hover)] transition-colors">
            <div class="w-8 h-8 rounded-full bg-[var(--primary)]/10 flex items-center justify-center">
              <i class="fas fa-exchange-alt text-[var(--primary)] text-sm"></i>
            </div>
            <div class="flex-1">
              <p class="text-sm text-[var(--text-primary)]">Trade #{{ 1000 + i }} executed</p>
              <p class="text-xs text-[var(--text-muted)]">2 hours ago</p>
            </div>
            <span class="text-sm text-[var(--success)]">+$1.2M</span>
          </div>
        </div>
      </div>

      <!-- Risk Alerts -->
      <div class="glass-card p-6">
        <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Risk Alerts</h3>
        <div class="space-y-3">
          <div class="flex items-start gap-3 p-3 rounded-lg bg-[var(--warning)]/10 border border-[var(--warning)]/20">
            <i class="fas fa-exclamation-triangle text-[var(--warning)] mt-0.5"></i>
            <div>
              <p class="text-sm text-[var(--text-primary)]">Credit limit approaching</p>
              <p class="text-xs text-[var(--text-muted)]">Counterparty XYZ at 85% utilisation</p>
            </div>
          </div>
          <div class="flex items-start gap-3 p-3 rounded-lg bg-[var(--primary)]/10 border border-[var(--primary)]/20">
            <i class="fas fa-info-circle text-[var(--primary)] mt-0.5"></i>
            <div>
              <p class="text-sm text-[var(--text-primary)]">Market data refresh</p>
              <p class="text-xs text-[var(--text-muted)]">Scheduled for 18:00 UTC</p>
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

.chart-container {
  position: relative;
}
</style>
