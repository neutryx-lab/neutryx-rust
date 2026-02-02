/**
 * Exposure View Module
 * Handles exposure chart rendering and interactions
 */

import { Chart, registerables, TooltipItem } from 'chart.js';
import { createScopedLogger } from '@/utils/logger';

// Register Chart.js components
Chart.register(...registerables);

const log = createScopedLogger('ExposureView');

// =============================================================================
// Types
// =============================================================================

interface ExposureDataPoint {
  time: number;
  pfe: number;
  ee: number;
  epe: number;
  ene: number;
}

interface ExposureState {
  chart: Chart | null;
  data: ExposureDataPoint[];
  visibleSeries: Set<string>;
  selectedTimeRange: string;
}

// =============================================================================
// State
// =============================================================================

const state: ExposureState = {
  chart: null,
  data: [],
  visibleSeries: new Set(['pfe', 'ee', 'epe']),
  selectedTimeRange: '10y',
};

let initialised = false;

// =============================================================================
// Mock Data Generation
// =============================================================================

function generateExposureData(): ExposureDataPoint[] {
  const data: ExposureDataPoint[] = [];
  const maxTime = 10; // 10 years
  const steps = 40;

  for (let i = 0; i <= steps; i++) {
    const time = (i / steps) * maxTime;
    // Simulate realistic exposure profiles
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

// =============================================================================
// Chart Configuration
// =============================================================================

function createChartConfig(data: ExposureDataPoint[]) {
  const labels = data.map(d => `${d.time.toFixed(1)}Y`);

  return {
    type: 'line' as const,
    data: {
      labels,
      datasets: [
        {
          label: 'PFE',
          data: data.map(d => d.pfe),
          borderColor: '#ef4444',
          backgroundColor: 'rgba(239, 68, 68, 0.1)',
          fill: true,
          tension: 0.4,
          hidden: !state.visibleSeries.has('pfe'),
        },
        {
          label: 'EE',
          data: data.map(d => d.ee),
          borderColor: '#3b82f6',
          backgroundColor: 'rgba(59, 130, 246, 0.1)',
          fill: true,
          tension: 0.4,
          hidden: !state.visibleSeries.has('ee'),
        },
        {
          label: 'EPE',
          data: data.map(d => d.epe),
          borderColor: '#10b981',
          backgroundColor: 'rgba(16, 185, 129, 0.1)',
          fill: true,
          tension: 0.4,
          hidden: !state.visibleSeries.has('epe'),
        },
        {
          label: 'ENE',
          data: data.map(d => d.ene),
          borderColor: '#8b5cf6',
          backgroundColor: 'rgba(139, 92, 246, 0.1)',
          fill: true,
          tension: 0.4,
          hidden: !state.visibleSeries.has('ene'),
        },
      ],
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      interaction: {
        intersect: false,
        mode: 'index' as const,
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
          },
        },
        y: {
          grid: {
            color: 'rgba(255, 255, 255, 0.05)',
          },
          ticks: {
            color: '#94a3b8',
            callback: (value: number | string) => `$${value}M`,
          },
        },
      },
    },
  };
}

// =============================================================================
// Chart Rendering
// =============================================================================

function renderChart(): void {
  const canvas = document.getElementById('exposure-view-chart') as HTMLCanvasElement | null;
  if (!canvas) {
    log.warn('Exposure view chart canvas not found');
    return;
  }

  // Destroy existing chart
  if (state.chart) {
    state.chart.destroy();
    state.chart = null;
  }

  // Generate data
  state.data = generateExposureData();

  // Create chart
  const ctx = canvas.getContext('2d');
  if (!ctx) {
    log.error('Failed to get canvas context');
    return;
  }

  state.chart = new Chart(ctx, createChartConfig(state.data));
  log.info('Exposure chart rendered');
}

function updateChartVisibility(): void {
  if (!state.chart) return;

  state.chart.data.datasets.forEach((dataset, index) => {
    const seriesName = ['pfe', 'ee', 'epe', 'ene'][index];
    dataset.hidden = !state.visibleSeries.has(seriesName);
  });

  state.chart.update();
}

// =============================================================================
// Event Handlers
// =============================================================================

function setupEventListeners(): void {
  // Toggle buttons
  const toggleBtns = document.querySelectorAll('#exposure-view .toggle-btn');
  toggleBtns.forEach(btn => {
    btn.addEventListener('click', () => {
      const metric = (btn as HTMLElement).dataset.metric;
      if (!metric) return;

      // Update active state
      toggleBtns.forEach(b => b.classList.remove('active'));
      btn.classList.add('active');

      // Update visibility
      if (metric === 'all') {
        state.visibleSeries = new Set(['pfe', 'ee', 'epe', 'ene']);
      } else {
        state.visibleSeries = new Set([metric]);
      }

      updateChartVisibility();
    });
  });

  // Legend items
  const legendItems = document.querySelectorAll('#exposure-view .legend-item');
  legendItems.forEach(item => {
    item.addEventListener('click', () => {
      const series = (item as HTMLElement).dataset.series;
      if (!series) return;

      // Toggle series visibility
      if (state.visibleSeries.has(series)) {
        if (state.visibleSeries.size > 1) {
          state.visibleSeries.delete(series);
          item.classList.remove('active');
        }
      } else {
        state.visibleSeries.add(series);
        item.classList.add('active');
      }

      updateChartVisibility();
    });
  });
}

// =============================================================================
// Public API
// =============================================================================

export async function init(): Promise<void> {
  if (initialised) {
    // Re-render chart when navigating back
    renderChart();
    return;
  }

  setupEventListeners();
  renderChart();
  initialised = true;
  log.info('Exposure view module initialised');
}

export const exposureView = {
  init,
  state,
  renderChart,
};

export default exposureView;
