/**
 * Dashboard Module
 * Handles dashboard charts and interactions
 */

import { Chart, registerables, TooltipItem } from 'chart.js';
import { createScopedLogger } from '@/utils/logger';

// Register Chart.js components
Chart.register(...registerables);

const log = createScopedLogger('Dashboard');

// =============================================================================
// State
// =============================================================================

interface DashboardState {
  exposureChart: Chart | null;
  visibleSeries: Set<string>;
}

const state: DashboardState = {
  exposureChart: null,
  visibleSeries: new Set(['pfe', 'ee', 'epe', 'ene']),
};

let initialised = false;

// =============================================================================
// Mock Data Generation
// =============================================================================

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

// =============================================================================
// Exposure Chart
// =============================================================================

function renderExposureChart(): void {
  const canvas = document.getElementById('exposure-chart') as HTMLCanvasElement | null;
  if (!canvas) {
    log.warn('Dashboard exposure chart canvas not found');
    return;
  }

  // Destroy existing chart
  if (state.exposureChart) {
    state.exposureChart.destroy();
    state.exposureChart = null;
  }

  const data = generateExposureData();
  const labels = data.map(d => `${d.time.toFixed(1)}Y`);

  const ctx = canvas.getContext('2d');
  if (!ctx) {
    log.error('Failed to get canvas context');
    return;
  }

  state.exposureChart = new Chart(ctx, {
    type: 'line',
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
  });

  log.info('Dashboard exposure chart rendered');
}

function updateChartVisibility(): void {
  if (!state.exposureChart) return;

  const seriesNames = ['pfe', 'ee', 'epe', 'ene'];
  state.exposureChart.data.datasets.forEach((dataset, index) => {
    dataset.hidden = !state.visibleSeries.has(seriesNames[index]);
  });

  state.exposureChart.update();
}

// =============================================================================
// Event Handlers
// =============================================================================

function setupEventListeners(): void {
  // Dashboard legend items
  const legendItems = document.querySelectorAll('#dashboard-view .chart-legend .legend-item');
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

  // Time range buttons
  const rangeButtons = document.querySelectorAll('#dashboard-view .chip-btn[data-range]');
  rangeButtons.forEach(btn => {
    btn.addEventListener('click', () => {
      rangeButtons.forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
      // Re-render with new range (simplified - just re-render)
      renderExposureChart();
    });
  });
}

// =============================================================================
// Public API
// =============================================================================

export async function init(): Promise<void> {
  if (initialised) {
    // Re-render chart when navigating back
    renderExposureChart();
    return;
  }

  setupEventListeners();
  renderExposureChart();
  initialised = true;
  log.info('Dashboard module initialised');
}

export const dashboard = {
  init,
  state,
  renderExposureChart,
};

export default dashboard;
