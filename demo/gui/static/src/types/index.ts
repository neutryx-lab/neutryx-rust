/**
 * Type definitions barrel export
 */

export * from './api';

// =============================================================================
// Global Window Extensions
// =============================================================================

declare global {
  interface Window {
    __FB_CONFIG__?: {
      debugMode: boolean;
      logLevel: string;
    };
    FB_Logger?: Logger;
    showToast?: (message: string, type: 'success' | 'error' | 'warning' | 'info') => void;
    navigateTo?: (viewId: string) => void;
  }
}

// =============================================================================
// Logger Interface
// =============================================================================

export interface Logger {
  debug(component: string, message: string, data?: unknown): void;
  info(component: string, message: string, data?: unknown): void;
  warn(component: string, message: string, data?: unknown): void;
  error(component: string, message: string, data?: unknown): void;
  isDebugEnabled(): boolean;
}

// =============================================================================
// UI State Types
// =============================================================================

export type SortDirection = 'asc' | 'desc';

export interface TableSortState {
  column: string;
  direction: SortDirection;
}

export interface FilterState {
  currency?: string;
  assetClass?: string;
}

// =============================================================================
// Module State Types
// =============================================================================

export interface PricerState {
  instruments: import('./api').Instrument[];
  selectedInstrument: string | null;
  instrumentParams: Record<string, unknown>;
  expandedTrade: import('./api').ExpandedTrade | null;
  editedCashflows: Record<string, CashflowEdit>;
  cashflows: import('./api').Cashflow[];
  pricingResult: import('./api').PricingResult | null;
  greeksResult: import('./api').GreeksResult | null;
}

export interface CashflowEdit {
  notional?: number;
  rate?: number;
}

export interface MarketDataState {
  rates: import('./api').MarketRate[];
  filteredRates: import('./api').MarketRate[];
  selectedRateId: string | null;
  sortColumn: string;
  sortDirection: SortDirection;
  lastUpdated: string | null;
  previousValues: Map<string, number>;
  isInitialised: boolean;
  assetClass: AssetClass;
  allConventions: import('./api').Convention[];
  filteredConventions: import('./api').Convention[];
  selectedConventionId: string | null;
  irVolCurrencies: import('./api').IrVolCurrency[];
  irVolQuotes: IrVolQuoteFlat[];
  selectedIrVolCurrency: string | null;
  fxVolPairs: import('./api').FxVolPair[];
  fxVolQuotes: FxVolQuoteFlat[];
  selectedFxVolPair: string | null;
  events: import('./api').MarketEvent[];
  filteredEvents: import('./api').MarketEvent[];
  eventTypes: string[];
  selectedEventId: string | null;
}

export type AssetClass = 'Rates' | 'FX' | 'IRVol' | 'FXVol' | 'Events';

export interface IrVolQuoteFlat {
  id: string;
  currency: string;
  expiry: string;
  tenor: string;
  atmVol: number;
  volType: string;
  smile: import('./api').SmilePoint[];
  source: string;
}

export interface FxVolQuoteFlat {
  id: string;
  pair: string;
  expiry: number;
  expiryLabel: string;
  atmVol: number;
  rr25d: number;
  bf25d: number;
  rr10d?: number;
  bf10d?: number;
  spot?: number;
  source: string;
}

// =============================================================================
// DOM Element Cache Types
// =============================================================================

export type ElementCache<T extends string> = Partial<Record<T, HTMLElement | null>>;
