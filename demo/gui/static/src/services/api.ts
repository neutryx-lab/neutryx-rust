/**
 * API Service for FrictionalBank Dashboard
 * Centralised HTTP client for all backend API calls.
 */

import type {
  AppConfig,
  InstrumentsResponse,
  TradeExpandRequest,
  ExpandedTrade,
  PricingRequest,
  PricingResult,
  GreeksRequest,
  GreeksResult,
  MarketRatesResponse,
  MarketConfigResponse,
  MarketRate,
  ConventionsResponse,
  Convention,
  IrVolCurrenciesResponse,
  IrVolQuotesResponse,
  FxVolPairsResponse,
  FxVolQuotesResponse,
  EventsResponse,
  EventTypesResponse,
  CurveBuilderRequest,
  CurveBuilderResponse,
  PortfolioGraphResponse,
  TradeListResponse,
} from '@/types';

const API_BASE = '/api';
const DATA_BASE = '/data/input';

// =============================================================================
// Generic Fetch Helpers
// =============================================================================

async function fetchJson<T>(url: string, options?: RequestInit): Promise<T> {
  const response = await fetch(url, options);
  if (!response.ok) {
    const error = await response.json().catch(() => ({})) as { message?: string; error?: string };
    throw new Error(error.message || error.error || `HTTP ${response.status}`);
  }
  return response.json() as Promise<T>;
}

async function postJson<TReq, TRes>(url: string, data: TReq): Promise<TRes> {
  return fetchJson<TRes>(url, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data),
  });
}

// =============================================================================
// Configuration API
// =============================================================================

export async function fetchConfig(): Promise<AppConfig> {
  return fetchJson<AppConfig>(`${API_BASE}/config`);
}

// =============================================================================
// Instruments API
// =============================================================================

export async function fetchInstruments(): Promise<InstrumentsResponse> {
  return fetchJson<InstrumentsResponse>(`${API_BASE}/instruments`);
}

// =============================================================================
// Trade Expansion API
// =============================================================================

export async function expandTrade(request: TradeExpandRequest): Promise<ExpandedTrade> {
  return postJson<TradeExpandRequest, ExpandedTrade>(`${API_BASE}/trade/expand`, request);
}

// =============================================================================
// Pricing API
// =============================================================================

export async function priceTrade(request: PricingRequest): Promise<PricingResult> {
  return postJson<PricingRequest, PricingResult>(`${API_BASE}/pricer/price`, request);
}

export async function calculateGreeks(request: GreeksRequest): Promise<GreeksResult> {
  return postJson<GreeksRequest, GreeksResult>(`${API_BASE}/pricer/greeks`, request);
}

// =============================================================================
// Market Data API
// =============================================================================

export async function fetchMarketRates(): Promise<MarketRatesResponse> {
  return fetchJson<MarketRatesResponse>(`${API_BASE}/market/rates`);
}

export async function fetchMarketConfig(): Promise<MarketConfigResponse> {
  return fetchJson<MarketConfigResponse>(`${API_BASE}/market/config`);
}

export async function fetchRateDetail(rateId: string): Promise<{
  rate: MarketRate;
  instrument?: unknown;
  convention?: Convention;
}> {
  return fetchJson(`${API_BASE}/market/rates/${encodeURIComponent(rateId)}`);
}

export async function refreshMarketRates(): Promise<void> {
  await fetch(`${API_BASE}/market/rates/refresh`, { method: 'POST' });
}

// =============================================================================
// Conventions API
// =============================================================================

export async function fetchConventions(): Promise<ConventionsResponse> {
  return fetchJson<ConventionsResponse>(`${API_BASE}/market/conventions`);
}

export async function fetchConventionDetail(id: string): Promise<Convention> {
  return fetchJson<Convention>(`${API_BASE}/market/conventions/${encodeURIComponent(id)}`);
}

// =============================================================================
// IR Volatility API
// =============================================================================

export async function fetchIrVolCurrencies(): Promise<IrVolCurrenciesResponse> {
  return fetchJson<IrVolCurrenciesResponse>(`${API_BASE}/irvol/currencies`);
}

export async function fetchIrVolQuotes(currency: string): Promise<IrVolQuotesResponse> {
  return fetchJson<IrVolQuotesResponse>(`${API_BASE}/irvol/quotes/${currency}`);
}

// =============================================================================
// FX Volatility API
// =============================================================================

export async function fetchFxVolPairs(): Promise<FxVolPairsResponse> {
  return fetchJson<FxVolPairsResponse>(`${API_BASE}/fxvol/pairs`);
}

export async function fetchFxVolQuotes(pair: string): Promise<FxVolQuotesResponse> {
  return fetchJson<FxVolQuotesResponse>(`${API_BASE}/fxvol/quotes/${pair}`);
}

// =============================================================================
// Events API
// =============================================================================

export async function fetchEvents(): Promise<EventsResponse> {
  return fetchJson<EventsResponse>(`${API_BASE}/market/events`);
}

export async function fetchEventTypes(): Promise<EventTypesResponse> {
  return fetchJson<EventTypesResponse>(`${API_BASE}/market/events/types`);
}

// =============================================================================
// Curve Builder API
// =============================================================================

export async function buildCurve(request: CurveBuilderRequest): Promise<CurveBuilderResponse> {
  return postJson<CurveBuilderRequest, CurveBuilderResponse>(`${API_BASE}/curves/build`, request);
}

export async function fetchAvailableCurves(): Promise<{ curves: string[] }> {
  return fetchJson<{ curves: string[] }>(`${API_BASE}/curves`);
}

// =============================================================================
// Static Data (JSON Files)
// =============================================================================

export async function fetchPortfolio(): Promise<unknown> {
  return fetchJson(`${DATA_BASE}/demo_portfolio.json`);
}

export async function fetchCounterparties(): Promise<unknown> {
  return fetchJson(`${DATA_BASE}/counterparties.json`);
}

export async function fetchNettingSets(): Promise<unknown> {
  return fetchJson(`${DATA_BASE}/netting_sets.json`);
}

// =============================================================================
// Export Helpers
// =============================================================================

export async function exportRatesCsv(): Promise<Blob> {
  const response = await fetch(`${API_BASE}/market/export/csv`);
  if (!response.ok) throw new Error('Export failed');
  return response.blob();
}

export async function exportRatesJson(): Promise<Blob> {
  const response = await fetch(`${API_BASE}/market/export/json`);
  if (!response.ok) throw new Error('Export failed');
  return response.blob();
}

// =============================================================================
// Portfolio Graph API
// =============================================================================

const PORTFOLIO_API_BASE = '/api/portfolio';

export async function fetchPortfolioGraph(tradeIds?: string[]): Promise<PortfolioGraphResponse> {
  const params = tradeIds && tradeIds.length > 0
    ? `?trade_ids=${tradeIds.join(',')}`
    : '';
  return fetchJson<PortfolioGraphResponse>(`${PORTFOLIO_API_BASE}/graph${params}`);
}

export async function fetchPortfolioTrades(): Promise<TradeListResponse> {
  return fetchJson<TradeListResponse>(`${PORTFOLIO_API_BASE}/trades`);
}
