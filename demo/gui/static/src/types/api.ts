/**
 * API Response Types for Neutryx FrictionalBank Dashboard
 * These types mirror the Rust backend API responses.
 */

// =============================================================================
// Common Types
// =============================================================================

export type Currency = string;
export type Tenor = string;
export type DateString = string; // ISO 8601 date string

// =============================================================================
// Configuration Types
// =============================================================================

export interface AppConfig {
  enums: Record<string, EnumValue[]>;
  defaults: Record<string, unknown>;
  rateIndexByCurrency: Record<Currency, string>;
}

export type EnumValue = string | { code: string; name?: string };

// =============================================================================
// Instrument Types
// =============================================================================

export interface Instrument {
  instrumentType: string;
  id?: string;
  type?: string;
  displayName?: string;
  name?: string;
  assetClassName?: string;
  assetClass?: string;
  requiredParams: ParameterDef[];
  optionalParams: ParameterDef[];
}

export interface ParameterDef {
  name: string;
  label?: string;
  fieldType: 'string' | 'number' | 'date' | 'select';
  defaultValue?: string | number | null;
  options?: ParameterOption[];
  validation?: {
    min?: number;
    max?: number;
  };
}

export interface ParameterOption {
  value: string;
  label: string;
}

export interface InstrumentsResponse {
  instruments: Instrument[];
}

// =============================================================================
// Trade Expansion Types
// =============================================================================

export interface TradeExpandRequest {
  instrumentType: string;
  params: {
    type: string;
    [key: string]: unknown;
  };
}

export interface ExpandedTrade {
  tradeId: string;
  tradeType: string;
  legs: TradeLeg[];
  metadata: TradeMetadata;
}

export interface TradeLeg {
  direction: 'Payer' | 'Receiver';
  currency: Currency;
  legType: string;
  rateIndex?: string;
  cashflows: Cashflow[];
}

export interface Cashflow {
  paymentDate: DateString;
  accrualStart: DateString;
  accrualEnd: DateString;
  yearFraction: number;
  notional: number;
  rate: number | null;
  payoffType: string;
  rateIndex?: string;
}

export interface TradeMetadata {
  totalLegs: number;
  totalCashflows: number;
  processingTimeMs: number;
}

// =============================================================================
// Pricing Types
// =============================================================================

export interface PricingRequest {
  valuationDate: DateString;
  reportingCurrency: Currency;
  legs: PricingLeg[];
  modelConfig?: ModelConfig | null;
}

export interface PricingLeg {
  currency: Currency;
  direction: 'payer' | 'receiver';
  cashflows: PricingCashflow[];
}

export interface PricingCashflow {
  paymentDate: DateString;
  amount: number;
}

export interface ModelConfig {
  numPaths: number;
  numSteps: number;
  seed?: number | null;
}

export interface PricingResult {
  totalPv?: number;
  pv?: number;
  currency: Currency;
  legs?: LegResult[];
}

export interface LegResult {
  direction: string;
  pv: number;
}

// =============================================================================
// Greeks Types
// =============================================================================

export interface GreeksRequest extends PricingRequest {
  bumpSizes: BumpSizes;
}

export interface BumpSizes {
  rateBumpBp: number;
  fxBumpPct: number;
  volBumpPct: number;
}

export interface GreeksResult {
  currency: Currency;
  delta: number;
  gamma: number | null;
  theta: number | null;
  vega: number | null;
}

// =============================================================================
// Market Data Types
// =============================================================================

export interface MarketRate {
  id: string;
  currency: Currency;
  tenor: Tenor;
  rateType: string;
  value: number;
  rateIndex?: string;
  quoteType?: string;
  source: string;
  timestamp: string;
  isStale: boolean;
}

export interface MarketRatesResponse {
  rates: MarketRate[];
  lastUpdated: string;
}

export interface MarketConfigResponse {
  tenorOrder: string[];
}

export interface Convention {
  id: string;
  conventionType: string;
  currency: Currency;
  isDefault?: boolean;
  fields?: ConventionField[];
}

export interface ConventionField {
  label: string;
  value: string;
}

export interface ConventionsResponse {
  conventions: Convention[];
}

// =============================================================================
// Rate Instrument Types (market-convention-instrument)
// =============================================================================

export interface RateInstrumentResponse {
  rateId: string;
  rateValue: number;
  instrumentType: string;
  convention?: ConventionDetail;
  effectiveDate: DateString;
  maturityDate: DateString;
  notional: number;
  processingTimeMs: number;
}

export interface ConventionDetail {
  conventionType: string;
  dayCount?: string;
  frequency?: string;
  businessDayConvention?: string;
  spotLag?: number;
  calendar?: string;
}

export interface RateCashflowsResponse {
  rateId: string;
  legs: LegCashflows[];
  processingTimeMs: number;
}

export interface LegCashflows {
  legType: string;
  direction: string;
  currency: Currency;
  rateIndex?: string;
  cashflows: CashflowDetail[];
}

export interface CashflowDetail {
  paymentDate: DateString;
  accrualStart: DateString;
  accrualEnd: DateString;
  yearFraction: number;
  notional: number;
  rate?: number;
  spread?: number;
  payoffType: string;
}

// =============================================================================
// Rate Index Types (market-convention-instrument)
// =============================================================================

export interface RateIndexInfo {
  code: string;
  name: string;
  currency: Currency;
  tenor: Tenor;
  dayCounter?: string;
  isOvernight: boolean;
  associatedRatesCount: number;
  associatedConventionsCount: number;
}

export interface RateIndicesResponse {
  indices: RateIndexInfo[];
}

export interface RateIndexDetailResponse {
  code: string;
  name: string;
  currency: Currency;
  tenor: Tenor;
  metadata?: RateIndexMetadata;
  associatedRates: string[];
  associatedConventions: string[];
}

export interface RateIndexMetadata {
  fixingLag?: number;
  settlementLag?: number;
  compoundingMethod?: string;
  fixingCalendar?: string;
}

export interface IndexRatesResponse {
  rates: MarketRate[];
}

export interface IndexConventionsResponse {
  conventions: Convention[];
}

// =============================================================================
// IR Volatility Types
// =============================================================================

export interface IrVolCurrency {
  currency: Currency;
}

export interface IrVolCurrenciesResponse {
  currencies: IrVolCurrency[];
}

export interface IrVolQuote {
  expiry: Tenor;
  tenor: Tenor;
  atmVol: number;
  smile?: SmilePoint[];
}

export interface SmilePoint {
  strikeOffsetBp: number;
  vol: number;
}

export interface IrVolQuotesResponse {
  quotes: IrVolQuote[];
  volType?: string;
  source?: string;
}

// =============================================================================
// FX Volatility Types
// =============================================================================

export interface FxVolPair {
  pair: string;
}

export interface FxVolPairsResponse {
  pairs: FxVolPair[];
}

export interface FxVolQuote {
  expiry: number; // Year fraction
  atmVol: number;
  rr25d: number;
  bf25d: number;
  rr10d?: number;
  bf10d?: number;
}

export interface FxVolQuotesResponse {
  quotes: FxVolQuote[];
  spot?: number;
}

// =============================================================================
// Events Types
// =============================================================================

export interface MarketEvent {
  id: string;
  date: DateString;
  eventType: EventType;
  title: string;
  description?: string;
  currency?: Currency;
  region?: string;
  importance: Importance;
  time?: string;
  timezone?: string;
  source: string;
  tags?: string[];
  centralBank?: CentralBank;
  previous?: string;
  forecast?: string;
  actual?: string;
  /** Expected rate spike in basis points (for turn events) */
  expectedSpikeBp?: number;
}

export type EventType =
  | 'central_bank_meeting'
  | 'economic_release'
  | 'holiday'
  | 'news'
  | 'expiry'
  | 'turn_of_year'
  | 'turn_of_quarter'
  | 'turn_of_month'
  | 'turn'
  | 'other';

export type Importance = 'critical' | 'high' | 'medium' | 'low';

export interface CentralBank {
  name: string;
  code: string;
  currency: Currency;
}

export interface EventsResponse {
  events: MarketEvent[];
}

export interface EventTypesResponse {
  types: string[];
}

// =============================================================================
// Curve Builder Types
// =============================================================================

export interface CurveBuilderRequest {
  currency: Currency;
  referenceDate: DateString;
  instruments: CurveInstrument[];
}

export interface CurveInstrument {
  type: string;
  tenor: Tenor;
  rate: number;
}

export interface CurveBuilderResponse {
  curveId: string;
  currency: Currency;
  referenceDate: DateString;
  discountFactors: DiscountFactor[];
  zeroRates: ZeroRate[];
  forwardRates: ForwardRate[];
  metadata: CurveMetadata;
}

export interface DiscountFactor {
  date: DateString;
  yearFraction: number;
  value: number;
}

export interface ZeroRate {
  date: DateString;
  yearFraction: number;
  rate: number;
}

export interface ForwardRate {
  startDate: DateString;
  endDate: DateString;
  rate: number;
}

export interface CurveMetadata {
  instrumentCount: number;
  interpolation: string;
  processingTimeMs: number;
}

// =============================================================================
// Portfolio Types
// =============================================================================

export interface PortfolioTrade {
  tradeId: string;
  instrumentType: string;
  counterparty?: string;
  currency: Currency;
  notional: number;
  startDate: DateString;
  endDate: DateString;
  nettingSet?: string;
}

export interface Counterparty {
  id: string;
  name: string;
  rating?: string;
}

export interface NettingSet {
  id: string;
  counterpartyId: string;
  csaType?: string;
}

// =============================================================================
// Graph Types
// =============================================================================

export interface GraphNode {
  id: string;
  type: string;
  label: string;
  value?: number;
  is_sensitivity_target: boolean;
  group: string;
  trade_ids: string[];
}

export interface GraphEdge {
  source: string;
  target: string;
  weight?: number;
}

export interface GraphMetadata {
  node_count: number;
  edge_count: number;
  depth: number;
  generated_at: string;
  trade_count: number;
  shared_node_count: number;
  optimisation_ratio: number;
  large_graph_warning?: boolean;
}

export interface PortfolioGraphResponse {
  nodes: GraphNode[];
  links: GraphEdge[];
  metadata: GraphMetadata;
}

export interface TradeSummary {
  id: string;
  instrument_type: string;
  currency: string;
  notional: number;
  /** Maturity date (last cashflow date) as ISO 8601 string, e.g., "2025-07-15" */
  maturity: string;
  /** Counterparty name */
  counterparty: string;
  /** Trading book */
  book: string;
}

export interface TradeStatistics {
  total_count: number;
  by_instrument_type: Record<string, number>;
  by_currency: Record<string, number>;
  total_notional: number;
}

export interface TradeListResponse {
  trades: TradeSummary[];
  statistics: TradeStatistics;
}
