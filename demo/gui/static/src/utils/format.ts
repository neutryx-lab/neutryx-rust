/**
 * Formatting utilities for the dashboard
 */

/**
 * Format a number as currency.
 */
export function formatCurrency(value: number, currency = 'USD'): string {
  const num = Number(value) || 0;
  return new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency,
    minimumFractionDigits: 0,
    maximumFractionDigits: 0,
  }).format(num);
}

/**
 * Format a number with thousand separators.
 */
export function formatNumber(value: number): string {
  const num = Number(value) || 0;
  return new Intl.NumberFormat('en-US', {
    minimumFractionDigits: 0,
    maximumFractionDigits: 0,
  }).format(num);
}

/**
 * Format a number in compact form (e.g., 10M, 1.5B).
 */
export function formatNumberCompact(value: number): string {
  const num = Math.abs(value);
  const sign = value < 0 ? '-' : '';

  if (num >= 1e9) {
    return sign + (num / 1e9).toFixed(2) + 'B';
  }
  if (num >= 1e6) {
    return sign + (num / 1e6).toFixed(2) + 'M';
  }
  if (num >= 1e3) {
    return sign + (num / 1e3).toFixed(0) + 'K';
  }
  return sign + num.toFixed(0);
}

/**
 * Parse a formatted number string (e.g., "10M", "1.5B", "10,000").
 */
export function parseFormattedNumber(str: string): number {
  if (!str) return 0;
  const s = str.toString().toUpperCase().replace(/,/g, '').trim();

  const suffixes: Record<string, number> = { K: 1e3, M: 1e6, B: 1e9 };
  for (const [suffix, multiplier] of Object.entries(suffixes)) {
    if (s.endsWith(suffix)) {
      return parseFloat(s.slice(0, -1)) * multiplier;
    }
  }
  return parseFloat(s) || 0;
}

/**
 * Format a rate value based on rate type.
 */
export function formatRate(value: number, rateType: string): string {
  if (rateType === 'FxSpot') {
    return value.toFixed(4);
  }
  if (rateType === 'FxForward') {
    return value.toFixed(2) + ' pts';
  }
  if (rateType === 'XccyBasis') {
    return (value * 10000).toFixed(2) + ' bps';
  }
  // Interest rates as percentage
  return (value * 100).toFixed(4) + '%';
}

/**
 * Format a value as percentage.
 */
export function formatPercent(value: number): string {
  return (value * 100).toFixed(4) + '%';
}

/**
 * Format volatility as percentage (e.g., 0.12 -> 12.00%).
 */
export function formatVol(value: number | null | undefined): string {
  if (value == null) return '-';
  return (value * 100).toFixed(2) + '%';
}

/**
 * Format volatility difference as basis points.
 */
export function formatVolBps(value: number | null | undefined): string {
  if (value == null) return '-';
  const bps = value * 10000;
  const sign = bps >= 0 ? '+' : '';
  return sign + bps.toFixed(1) + ' bps';
}

/**
 * Format a date string (YYYY-MM-DD) for display.
 */
export function formatDate(dateStr: string): string {
  if (!dateStr) return '-';
  const date = new Date(dateStr);
  return date.toLocaleDateString('en-GB', {
    day: '2-digit',
    month: 'short',
    year: 'numeric',
  });
}

/**
 * Format a timestamp as locale string.
 */
export function formatTimestamp(ts: string): string {
  const date = new Date(ts);
  return date.toLocaleString();
}

/**
 * Format a date as locale time string.
 */
export function formatTime(ts: string): string {
  const date = new Date(ts);
  return date.toLocaleTimeString();
}

/**
 * Escape HTML special characters.
 */
export function escapeHtml(str: unknown): string {
  if (str == null) return '';
  return String(str)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}
