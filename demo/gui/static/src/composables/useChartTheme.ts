/**
 * Provides Chart.js colour values derived from the current CSS theme variables.
 *
 * Call `getChartColors()` at chart-creation time (or on theme change) so the
 * resolved colours match whichever theme is active (dark / light / oled).
 */

function css(prop: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(prop).trim();
}

export interface ChartColors {
  /** Tick / axis-label colour  (--text-muted) */
  tick: string;
  /** Grid-line colour          (--glass-border) */
  grid: string;
  /** Tooltip background        (--glass-bg) */
  tooltipBg: string;
  /** Tooltip title colour      (--text-primary) */
  tooltipTitle: string;
  /** Tooltip body colour       (--text-secondary) */
  tooltipBody: string;
  /** Legend label colour        (--text-secondary) */
  legend: string;
}

export function getChartColors(): ChartColors {
  return {
    tick: css('--text-muted') || '#94a3b8',
    grid: css('--glass-border') || 'rgba(148,163,184,0.2)',
    tooltipBg: css('--glass-bg') || 'rgba(30,41,59,0.8)',
    tooltipTitle: css('--text-primary') || '#f1f5f9',
    tooltipBody: css('--text-secondary') || '#cbd5e1',
    legend: css('--text-secondary') || '#cbd5e1',
  };
}
