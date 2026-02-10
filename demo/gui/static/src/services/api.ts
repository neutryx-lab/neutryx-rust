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
  RateInstrumentResponse,
  RateCashflowsResponse,
  RateIndicesResponse,
  RateIndexDetailResponse,
  IndexRatesResponse,
  IndexConventionsResponse,
  VolcubeIndicesResponse,
  VolcubeModelsResponse,
  VolcubeInstrumentsResponse,
  VolcubeCalibrateRequest,
  VolcubeCalibrateResponse,
  FxVolCalibrateRequest,
  SabrSmileRequest,
  SabrSmileResponse,
  ImpliedPdfRequest,
  ImpliedPdfResponse,
  PricerGraphRequest,
  PricerGraphResponse,
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
// Rate Instrument API (market-convention-instrument)
// =============================================================================

export async function fetchRateInstrument(rateId: string): Promise<RateInstrumentResponse> {
  return fetchJson<RateInstrumentResponse>(
    `${API_BASE}/market/rates/${encodeURIComponent(rateId)}/instrument`
  );
}

export async function fetchRateCashflows(rateId: string): Promise<RateCashflowsResponse> {
  return fetchJson<RateCashflowsResponse>(
    `${API_BASE}/market/rates/${encodeURIComponent(rateId)}/cashflows`
  );
}

// =============================================================================
// Rate Index API (market-convention-instrument)
// =============================================================================

export async function fetchRateIndices(): Promise<RateIndicesResponse> {
  return fetchJson<RateIndicesResponse>(`${API_BASE}/market/indices`);
}

export async function fetchRateIndexDetail(code: string): Promise<RateIndexDetailResponse> {
  return fetchJson<RateIndexDetailResponse>(
    `${API_BASE}/market/indices/${encodeURIComponent(code)}`
  );
}

export async function fetchIndexRates(code: string): Promise<IndexRatesResponse> {
  return fetchJson<IndexRatesResponse>(
    `${API_BASE}/market/indices/${encodeURIComponent(code)}/rates`
  );
}

export async function fetchIndexConventions(code: string): Promise<IndexConventionsResponse> {
  return fetchJson<IndexConventionsResponse>(
    `${API_BASE}/market/indices/${encodeURIComponent(code)}/conventions`
  );
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
// Volcube API
// =============================================================================

export async function fetchVolcubeIndices(): Promise<VolcubeIndicesResponse> {
  return fetchJson<VolcubeIndicesResponse>(`${API_BASE}/volcube/indices`);
}

export async function fetchVolcubeModels(): Promise<VolcubeModelsResponse> {
  return fetchJson<VolcubeModelsResponse>(`${API_BASE}/volcube/models`);
}

export async function fetchVolcubeInstruments(currency: string): Promise<VolcubeInstrumentsResponse> {
  return fetchJson<VolcubeInstrumentsResponse>(`${API_BASE}/volcube/instruments/${encodeURIComponent(currency)}`);
}

export async function calibrateVolcube(request: VolcubeCalibrateRequest): Promise<VolcubeCalibrateResponse> {
  return postJson<VolcubeCalibrateRequest, VolcubeCalibrateResponse>(`${API_BASE}/volcube/calibrate`, request);
}

export async function computeSabrSmile(request: SabrSmileRequest): Promise<SabrSmileResponse> {
  return postJson<SabrSmileRequest, SabrSmileResponse>(`${API_BASE}/volcube/sabr-smile`, request);
}

export async function computeImpliedPdf(request: ImpliedPdfRequest): Promise<ImpliedPdfResponse> {
  return postJson<ImpliedPdfRequest, ImpliedPdfResponse>(`${API_BASE}/volcube/implied-pdf`, request);
}

export async function calibrateFxVol(request: FxVolCalibrateRequest): Promise<VolcubeCalibrateResponse> {
  return postJson<FxVolCalibrateRequest, VolcubeCalibrateResponse>(`${API_BASE}/fxvol/calibrate`, request);
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

// =============================================================================
// Pricer Graph API
// =============================================================================

export async function fetchPricerGraph(request: PricerGraphRequest): Promise<PricerGraphResponse> {
  return postJson<PricerGraphRequest, PricerGraphResponse>(`${API_BASE}/pricer/graph`, request);
}
