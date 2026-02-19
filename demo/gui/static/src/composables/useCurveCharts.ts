/**
 * Composable encapsulating Chart.js rendering logic for the Curve Builder.
 *
 * Manages short-term and long-term chart instances, milestone-based tick
 * labels, and chart type toggling (forward rate vs discount factor).
 */

import { ref, onUnmounted } from 'vue';
import { Chart, registerables } from 'chart.js';
import { getChartColors } from '@/composables/useChartTheme';
import type { BuildResult, ChartGridPoint } from '@/composables/useCurveBuilder';

// Spot stored per-render for FX basis computation
let _fxSpot = 0;

Chart.register(...registerables);

// ---------------------------------------------------------------------------
// Milestone definitions for term tick labels
// ---------------------------------------------------------------------------

const SHORT_MILESTONES = [
  { time: 7 / 365, term: '1W' }, { time: 14 / 365, term: '2W' },
  { time: 1 / 12, term: '1M' }, { time: 2 / 12, term: '2M' }, { time: 3 / 12, term: '3M' },
  { time: 6 / 12, term: '6M' }, { time: 9 / 12, term: '9M' }, { time: 1.0, term: '1Y' },
];

const LONG_MILESTONES = [
  { time: 1, term: '1Y' }, { time: 2, term: '2Y' }, { time: 3, term: '3Y' },
  { time: 5, term: '5Y' }, { time: 7, term: '7Y' }, { time: 10, term: '10Y' },
  { time: 15, term: '15Y' }, { time: 20, term: '20Y' }, { time: 25, term: '25Y' },
  { time: 30, term: '30Y' },
];

// ---------------------------------------------------------------------------
// Composable
// ---------------------------------------------------------------------------

export function useCurveCharts() {
  // Chart canvas template refs
  const shortTermChartCanvas = ref<HTMLCanvasElement | null>(null);
  const longTermChartCanvas = ref<HTMLCanvasElement | null>(null);

  // Chart instances (not reactive -- internal only)
  let shortTermChartInstance: Chart | null = null;
  let longTermChartInstance: Chart | null = null;

  // Chart display mode
  const chartType = ref<'discount_factor' | 'forward_rate' | 'fx_basis'>('forward_rate');

  // ------ Chart option factory ------

  function createChartOptions(yAxisLabel: string, isFx = false) {
    const cc = getChartColors();
    return {
      responsive: true,
      maintainAspectRatio: false,
      plugins: {
        legend: { display: false },
        tooltip: {
          backgroundColor: cc.tooltipBg,
          titleColor: cc.tooltipTitle,
          bodyColor: cc.tooltipBody,
          callbacks: {
            title: (items: { label: string }[]) => items[0].label,
            label: (item: { raw: unknown }) => {
              const value = item.raw as number;
              if (isFx && chartType.value === 'fx_basis') {
                return `Fwd Basis: ${value.toFixed(4)}`;
              } else if (isFx) {
                return `FX Forward: ${value.toFixed(4)}`;
              } else if (chartType.value === 'discount_factor') {
                return `DF: ${value.toFixed(6)}`;
              } else {
                return `Forward Rate: ${value.toFixed(4)}%`;
              }
            },
          },
        },
      },
      scales: {
        x: {
          ticks: { color: cc.tick, maxTicksLimit: 12 },
          grid: { color: cc.grid },
        },
        y: {
          title: { display: true, text: yAxisLabel, color: cc.tick },
          ticks: { color: cc.tick },
          grid: { color: cc.grid },
        },
      },
    };
  }

  // ------ Render a single chart ------

  function renderChart(
    canvas: HTMLCanvasElement | null,
    existing: Chart | null,
    grid: ChartGridPoint[],
    label: string,
    color: string,
    milestones: { time: number; term: string }[],
    interpolationValue: string,
    isFx = false,
  ): Chart | null {
    if (!canvas || grid.length === 0) return existing;
    if (existing) existing.destroy();

    const labels = grid.map(pt => pt.label);
    const data = isFx
      ? (chartType.value === 'fx_basis'
        ? grid.map(pt => pt.forward_rate - _fxSpot)
        : grid.map(pt => pt.forward_rate))
      : chartType.value === 'forward_rate'
        ? grid.map(pt => pt.forward_rate * 100)
        : grid.map(pt => pt.discount_factor);

    // Compute milestone index -> [dateLabel, term]
    const milestoneAt = new Map<number, string[]>();
    for (const ms of milestones) {
      let bestIdx = 0;
      let bestDist = Infinity;
      for (let i = 0; i < grid.length; i++) {
        const dist = Math.abs(grid[i].time - ms.time);
        if (dist < bestDist) { bestDist = dist; bestIdx = i; }
      }
      milestoneAt.set(bestIdx, [grid[bestIdx].label, ms.term]);
    }

    const opts = createChartOptions(label, isFx);
    const cc = getChartColors();
    (opts.scales.x as Record<string, unknown>).ticks = {
      autoSkip: false,
      maxRotation: 0,
      color: cc.tick,
      callback: (_value: unknown, index: number) => milestoneAt.get(index) ?? null,
    };

    const ctx = canvas.getContext('2d');
    if (!ctx) return null;

    // Flat Forward: use stepped line for forward rate charts to show flat segments
    const isFlatFwd = interpolationValue === 'flat_forward' && chartType.value === 'forward_rate';

    return new Chart(ctx, {
      type: 'line',
      data: {
        labels,
        datasets: [{
          label,
          data,
          borderColor: color,
          backgroundColor: `${color}1a`,
          borderWidth: 2,
          fill: true,
          tension: isFlatFwd ? 0 : 0.3,
          stepped: isFlatFwd ? 'before' : false,
          pointRadius: 1,
          pointBackgroundColor: color,
        }],
      },
      options: opts,
    });
  }

  // ------ Public update entry point ------

  function updateCharts(result: BuildResult, interpolationValue: string) {
    if (!result) return;

    const shortGrid = result.short_term_grid || [];
    const longGrid = result.long_term_grid || [];
    const isFx = result.curve_type === 'fx';

    if (isFx) {
      _fxSpot = result.spot ?? 0;
      const isBasis = chartType.value === 'fx_basis';
      const fxLabel = isBasis ? 'FX Fwd Basis' : 'FX Forward Rate';
      const fxColor = isBasis ? '#f59e0b' : '#06b6d4'; // amber / cyan

      shortTermChartInstance = renderChart(
        shortTermChartCanvas.value, shortTermChartInstance, shortGrid, fxLabel, fxColor, SHORT_MILESTONES, interpolationValue, true,
      );
      longTermChartInstance = renderChart(
        longTermChartCanvas.value, longTermChartInstance, longGrid, fxLabel, fxColor, LONG_MILESTONES, interpolationValue, true,
      );
    } else {
      const chartLabels: Record<string, string> = {
        discount_factor: 'Discount Factor',
        forward_rate: 'Forward Rate (%)',
      };
      const chartColors: Record<string, string> = {
        discount_factor: '#6366f1',
        forward_rate: '#10b981',
      };

      const currentLabel = chartLabels[chartType.value];
      const currentColor = chartColors[chartType.value];

      shortTermChartInstance = renderChart(
        shortTermChartCanvas.value, shortTermChartInstance, shortGrid, currentLabel, currentColor, SHORT_MILESTONES, interpolationValue,
      );
      longTermChartInstance = renderChart(
        longTermChartCanvas.value, longTermChartInstance, longGrid, currentLabel, currentColor, LONG_MILESTONES, interpolationValue,
      );
    }
  }

  // ------ Cleanup ------

  onUnmounted(() => {
    if (shortTermChartInstance) {
      shortTermChartInstance.destroy();
    }
    if (longTermChartInstance) {
      longTermChartInstance.destroy();
    }
  });

  return {
    shortTermChartCanvas,
    longTermChartCanvas,
    chartType,
    updateCharts,
  };
}
