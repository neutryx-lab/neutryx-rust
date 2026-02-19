/**
 * Pricer-specific constants and type definitions.
 *
 * Extracted from the monolithic PricerView.vue to enable reuse
 * across composables, stores, and components.
 */

// =============================================================================
// Type Definitions
// =============================================================================

export interface ModelParamDef {
  name: string;
  label: string;
  defaultValue: number;
  min?: number;
  max?: number;
  step?: number;
}

export interface StochasticModelConfig {
  type: string;
  label: string;
  params: ModelParamDef[];
}

export interface CurveOption {
  index: string;
  label: string;
  currency: string;
}

export interface ValidationError {
  field: string;
  message: string;
}

export interface ComputationMetrics {
  pricingTimeMs: number;
  method: string;
  timestamp: number;
}

export interface SummaryStat {
  label: string;
  value: string;
  icon: string;
  color: string;
}

export interface CurrencyAgg {
  ccy: string;
  pv: number;
}

export interface CashflowEdit {
  notional?: number;
  rate?: number;
}

export interface HistoryEntry {
  id: string;
  timestamp: number;
  instrumentId: string;
  instrumentName: string;
  valuationDate: string;
  reportingCcy: string;
  totalPv: number;
  legs: import('@/types/api').PricingLeg[];
  pricingResult: import('@/types/api').PricingResult;
}

// =============================================================================
// Constants
// =============================================================================

export const STOCHASTIC_MODELS: StochasticModelConfig[] = [
  {
    type: 'GBM',
    label: 'Geometric Brownian Motion',
    params: [
      { name: 'drift', label: 'Drift', defaultValue: 0.05, min: -0.5, max: 0.5, step: 0.01 },
      { name: 'vol', label: 'Volatility', defaultValue: 0.2, min: 0.01, max: 2.0, step: 0.01 },
    ],
  },
  {
    type: 'Heston',
    label: 'Heston Stochastic Vol',
    params: [
      { name: 'v0', label: 'Initial Vol', defaultValue: 0.04, min: 0.001, max: 1.0, step: 0.001 },
      { name: 'kappa', label: 'Mean Reversion', defaultValue: 2.0, min: 0.01, max: 10.0, step: 0.1 },
      { name: 'theta', label: 'Long-run Vol', defaultValue: 0.04, min: 0.001, max: 1.0, step: 0.001 },
      { name: 'sigma', label: 'Vol of Vol', defaultValue: 0.3, min: 0.01, max: 2.0, step: 0.01 },
      { name: 'rho', label: 'Correlation', defaultValue: -0.7, min: -1.0, max: 1.0, step: 0.05 },
    ],
  },
  {
    type: 'HullWhite',
    label: 'Hull-White',
    params: [
      { name: 'a', label: 'Mean Reversion (a)', defaultValue: 0.1, min: 0.001, max: 1.0, step: 0.01 },
      { name: 'sigma', label: 'Volatility', defaultValue: 0.01, min: 0.001, max: 0.1, step: 0.001 },
    ],
  },
  {
    type: 'CIR',
    label: 'Cox-Ingersoll-Ross',
    params: [
      { name: 'kappa', label: 'Mean Reversion', defaultValue: 0.5, min: 0.01, max: 5.0, step: 0.1 },
      { name: 'theta', label: 'Long-run Mean', defaultValue: 0.05, min: 0.001, max: 0.5, step: 0.005 },
      { name: 'sigma', label: 'Volatility', defaultValue: 0.1, min: 0.001, max: 1.0, step: 0.01 },
    ],
  },
];

export const CURVE_OPTIONS: CurveOption[] = [
  { index: 'USD-SOFR', label: 'USD SOFR', currency: 'USD' },
  { index: 'EUR-ESTR', label: 'EUR ESTR', currency: 'EUR' },
  { index: 'JPY-TONA', label: 'JPY TONA', currency: 'JPY' },
  { index: 'GBP-SONIA', label: 'GBP SONIA', currency: 'GBP' },
];
