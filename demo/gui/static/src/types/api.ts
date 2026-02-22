/**
 * API Response Types for Neutryx Ergodic Bank Dashboard
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

// Mirrors `PricingMethodHint`
export type PricingMethod = 'auto' | 'analytical' | 'monteCarlo' | 'tree';

// Mirrors `TreeType`
export type TreeTypeOption = 'binomial' | 'trinomial';

export interface McConfig {
  numPaths: number;
  numSteps: number;
  seed?: number | null;
}

export interface TreeConfig {
  numSteps?: number;
  treeType?: TreeTypeOption;
}

export interface PricingRequest {
  valuationDate: DateString;
  reportingCurrency: Currency;
  legs: PricingLeg[];
  method: PricingMethod;
  computeGreeks: boolean;
  mcConfig?: McConfig | null;
  treeConfig?: TreeConfig | null;
}

export interface PricingLeg {
  currency: Currency;
  direction: 'payer' | 'receiver';
  cashflows: PricingCashflow[];
}

export interface PricingCashflow {
  paymentDate: DateString;
  notional: number;
  rate: number | null;
  yearFraction: number;
  payoffType?: string;
  rateIndex?: string | null;
  accrualStart?: DateString;
  accrualEnd?: DateString;
}

// Mirrors `PricingResult` from result.rs
export interface PricingResult {
  totalPv: number;
  reportingCurrency: Currency;
  legs: LegResult[];
  pathDistribution?: PathDistribution | null;
  method?: string;
  greeks?: GreeksInline | null;
  computationTimeMs?: number;
}

export interface LegResult {
  direction: string;
  pv: number;
  currency: Currency;
  pvOriginal?: number;
  fxRate?: number;
  cashflows?: CashflowPvResult[];
}

export interface CashflowPvResult {
  pv: number;
  discountFactor: number;
  paymentDate: DateString;
}

export interface PathDistribution {
  mean: number;
  stdDev: number;
  percentiles: [number, number][];
  pathCount: number;
}

export interface GreeksInline {
  delta?: number | null;
  gamma?: number | null;
  vega?: number | null;
  theta?: number | null;
  rho?: number | null;
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
// Advanced Greeks Types (mirrors pricer_risk::GreeksConfig / GreeksResult)
// =============================================================================

export type AdvancedGreeksMode = 'bumpRevalue' | 'enzymeAad';

export type AdvancedGreeksConfig =
  | {
      mode: 'bumpRevalue';
      spotBumpRelative: number;
      volBumpAbsolute: number;
      timeBumpYears: number;
      rateBumpAbsolute: number;
    }
  | {
      mode: 'enzymeAad';
    };

export interface AdvancedGreeksRequest {
  valuationDate: DateString;
  reportingCurrency: Currency;
  legs: PricingLeg[];
  config: AdvancedGreeksConfig;
}

export interface RiskFactor {
  factorType: string;
  name: string;
}

export interface FactorGreeks {
  delta?: number | null;
  gamma?: number | null;
  vega?: number | null;
  theta?: number | null;
  rho?: number | null;
  vanna?: number | null;
  volga?: number | null;
}

export interface FactorGreeksEntry {
  factor: RiskFactor;
  greeks: FactorGreeks;
}

export interface AdvancedGreeksResult {
  price: number;
  currency: Currency;
  mode: string;
  computationTimeMs: number;
  factors: FactorGreeksEntry[];
  totals: FactorGreeks;
}

// =============================================================================
// Utility Types
// =============================================================================

export interface ResolveTenorRequest {
  tenor: string;
  base?: DateString;
}

export interface ResolveTenorResponse {
  date: DateString;
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
  expiryLabel: string;
  atmVol: number;
  rr25d: number;
  bf25d: number;
  rr10d?: number;
  bf10d?: number;
  forward?: number;
}

export interface FxVolQuotesResponse {
  quotes: FxVolQuote[];
  spot?: number;
  domesticRate?: number;
  foreignRate?: number;
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

export interface PricerGraphRequest {
  instrumentType: string;
  params: Record<string, unknown>;
  detailLevel?: 'operation' | 'scope';
}

export interface PricerGraphResponse {
  nodes: GraphNode[];
  links: GraphEdge[];
  metadata: GraphMetadata & {
    trade_id?: string;
    source_locations?: Record<string, string>;
  };
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

// =============================================================================
// Volcube Types
// =============================================================================

export interface VolcubeIndicesResponse {
  indices: string[];
}

export interface VolcubeModelsResponse {
  models: string[];
}

export interface SwaptionInstrument {
  expiry: string;
  tenor: string;
  strike: string;
  atmVol: number;
  smile: SmilePoint[];
  enabled: boolean;
}

export interface VolcubeInstrumentsResponse {
  instruments: SwaptionInstrument[];
  referenceDate?: string;
}

export interface SabrInitialParams {
  alpha?: number;
  beta?: number;
  rho?: number;
  nu?: number;
}

export interface SabrFixedParams {
  alpha?: boolean;
  beta?: boolean;
  rho?: boolean;
  nu?: boolean;
}

export interface VolcubeCalibrateRequest {
  index: string;
  referenceDate?: string;
  model?: string;
  forwardRates?: Record<string, number>;
  initialParams?: SabrInitialParams;
  fixedParams?: SabrFixedParams;
}

export interface CalibrationParameters {
  alpha: number;
  beta: number;
  rho: number;
  nu: number;
}

export interface CalibrationMetadata {
  instrumentCount: number;
  processingTimeMs: number;
  convergedCount?: number;
  maxRmse?: number;
}

export interface CellDiagnostics {
  converged: boolean;
  iterations: number;
  rmse: number;
}

export interface CellJacobian {
  rowLabels: string[];
  colLabels: string[];
  matrix: number[][];
}

export interface VolcubeCalibrateResponse {
  model: string;
  metadata: CalibrationMetadata;
  parameters: CalibrationParameters;
  cellParameters: Record<string, CalibrationParameters>;
  cellDiagnostics?: Record<string, CellDiagnostics>;
  cellJacobians?: Record<string, CellJacobian>;
}

export interface FxVolCalibrateRequest {
  pair: string;
  spot: number;
  domesticRate: number;
  foreignRate: number;
  forwardRates?: Record<string, number>;
  initialParams?: SabrInitialParams;
  fixedParams?: SabrFixedParams;
}

export interface SabrSmileRequest {
  alpha: number;
  beta: number;
  rho: number;
  nu: number;
  forward: number;
  expiry_years: number;
  n_points?: number;
  range_bp?: number;
}

export interface SabrSmileResponse {
  offsets: number[];
  vols: number[];
  density: number[];
}

/** Generic smile response (same shape as SABR, shared by all models). */
export type SmileResponse = SabrSmileResponse;

/** Generic smile request for any model via /volcube/model-smile. */
export interface VolSmileRequest {
  model: string;
  forward: number;
  expiryYears: number;
  nPoints?: number;
  rangeBp?: number;
  params: Record<string, unknown>;
}

export interface ImpliedPdfSmilePoint {
  strike_offset_bp: number;
  vol: number;
}

export interface ImpliedPdfRequest {
  expiry_years: number;
  atm_vol: number;
  smile: ImpliedPdfSmilePoint[];
  range_bp?: number;
  step_bp?: number;
}

export interface ImpliedPdfResponse {
  offsets: number[];
  density: number[];
}

// =============================================================================
// Exotic Product Types
// =============================================================================

export interface ExoticProductDef {
  productType: string;
  displayName: string;
  description: string;
  parameters: ExoticParameterDef[];
}

export interface ExoticParameterDef {
  name: string;
  displayName: string;
  fieldType: string;
  required: boolean;
  defaultValue?: any;
  description?: string;
}

export interface ExoticPricingResponse {
  price: number;
  currency: string;
  productType: string;
  mcStats?: MonteCarloStats;
  calculationTimeMs: number;
}

export interface MonteCarloStats {
  numPaths: number;
  stdError: number;
  confidence95: [number, number];
}

// =============================================================================
// MFM (Markov Functional Model) Types
// =============================================================================

export interface MfmProductDef {
  productType: string;
  displayName: string;
  description: string;
  parameters: MfmParameterDef[];
}

export interface MfmParameterDef {
  name: string;
  displayName: string;
  fieldType: string;
  required: boolean;
  defaultValue?: any;
  description?: string;
  group?: string;
}

// ── Calibration ──

export interface MfmCalibrateRequest {
  meanReversion: number;
  volatility: number;
  numGridPoints?: number;
  numStdDevs?: number;
  volType?: 'normal' | 'lognormal';
  exerciseTimes: number[];
  swapTenors: number[];
  paymentFrequencies: number[];
  fundingCurve: { rate: number };
  couponCurve: { rate: number };
  volSurfaceType?: 'flat' | 'sabr';
  flatVol?: { normalVolBp: number };
  sabrVol?: {
    expiries: number[];
    tenors: number[];
    alphas: number[];
    betas: number[];
    rhos: number[];
    nus: number[];
  };
}

export interface CalibratedSlice {
  exerciseTime: number;
  xGrid: number[];
  swapRates: number[];
  discountFactors: number[];
  annuities: number[];
}

export interface RateIndexCalibrationDto {
  rateIndex: string;
  slices: CalibratedSlice[];
}

export interface IntegralAdjusterDto {
  adders: number[];
  multipliers: number[];
}

export interface MfmCalibrateResponse {
  fundingCalibration: RateIndexCalibrationDto;
  couponSwapCalibration: RateIndexCalibrationDto;
  couponLiborCalibration: RateIndexCalibrationDto;
  adjuster: IntegralAdjusterDto;
  maxNrIterationsUsed: number;
  maxCalibrationError: number;
  computationTimeMs: number;
}

// ── Gaussian Tree ──

export interface GaussianTreeRequest {
  meanReversion: number;
  volatility: number;
  times: number[];
  numStdDevs?: number;
  numGridPoints?: number;
}

export interface GaussianTreeSliceDto {
  time: number;
  xGrid: number[];
  dx: number;
  conditionalVariance: number;
}

export interface GaussianTreeResponse {
  numSteps: number;
  numNodes: number;
  slices: GaussianTreeSliceDto[];
  arrowDebreuPrices: number[][];
  computationTimeMs: number;
}

// ── CIF Evaluation ──

export interface CifInstrumentDto {
  fixedRate: number;
  leverage: number;
  floorRate: number;
  capRate?: number;
  notional: number;
}

export interface CifEvaluateRequest {
  instrument: CifInstrumentDto;
  couponDates: number[];
  paymentDates: number[];
  yearFractions: number[];
  swapRates: number[][];
  liborRates: number[][];
  discountFactors: number[][];
  forwardSwapRates: number[];
  forwardLibors: number[];
  normalVols: number[];
}

export interface CifComponentsDto {
  dE: number[];
  dR: number[];
  dI: number[];
  dQ: number[];
  total: number[];
}

export interface CifCouponInfoDto {
  couponIdx: number;
  couponDateYf: number;
  paymentDateYf: number;
  yearFraction: number;
  forwardSwapRate: number;
  forwardLibor: number;
  normalVol: number;
  components: CifComponentsDto;
  discountedValues: number[];
}

export interface CifEvaluateResponse {
  coupons: CifCouponInfoDto[];
  computationTimeMs: number;
}

// ── Bermudan Pricing ──

export interface BermudanPriceRequest {
  meanReversion: number;
  volatility: number;
  numGridPoints?: number;
  numStdDevs?: number;
  volType?: 'normal' | 'lognormal';
  exerciseTimes: number[];
  swapTenors: number[];
  paymentFrequencies: number[];
  fundingCurve: { rate: number };
  couponCurve: { rate: number };
  volSurfaceType?: 'flat' | 'sabr';
  flatVol?: { normalVolBp: number };
  isCallable?: boolean;
  flatCoupon?: number;
}

export interface BermudanPriceResponse {
  pv: number;
  continuationValue: number;
  optionValue: number;
  exerciseBoundary: number[];
  computationTimeMs: number;
}

// ── TARN Pricing ──

export interface TarnPriceRequest {
  meanReversion: number;
  volatility: number;
  numGridPoints?: number;
  numStdDevs?: number;
  volType?: 'normal' | 'lognormal';
  exerciseTimes: number[];
  swapTenors: number[];
  paymentFrequencies: number[];
  fundingCurve: { rate: number };
  couponCurve: { rate: number };
  volSurfaceType?: 'flat' | 'sabr';
  flatVol?: { normalVolBp: number };
  tarnAmount: number;
  numCouponGridPoints?: number;
  excessCouponFlag?: boolean;
  hasBermudanExercise?: boolean;
  isCallable?: boolean;
  flatCoupon?: number;
}

export interface TarnPriceResponse {
  pv: number;
  autoRedemptionProbability: number;
  expectedRedemptionTime: number;
  computationTimeMs: number;
}

// =============================================================================
// XVA Engine Types
// =============================================================================

export interface XvaDefaultConfigResponse {
  nPaths: number;
  horizonYears: number;
  timeStep: string;
  antithetic: boolean;
  bilateral: boolean;
  computeFva: boolean;
  pfePercentiles: number[];
  counterparties: DemoCounterparty[];
}

export interface DemoCounterparty {
  id: string;
  name: string;
  creditRating: string;
  hazardRate: number;
  lgd: number;
  nettingSets: DemoNettingSet[];
}

export interface DemoNettingSet {
  id: string;
  hasCsa: boolean;
  tradeCount: number;
  tradeTypes: string[];
}

export interface XvaSimulationRequest {
  nPaths?: number;
  horizonYears?: number;
  timeStep?: string;
  seed?: number;
  antithetic?: boolean;
  pfePercentiles?: number[];
  bilateral?: boolean;
  computeFva?: boolean;
  counterpartyId?: string;
}

export interface XvaSimulationResponse {
  config: XvaConfigSummary;
  timeGrid: number[];
  nPaths: number;
  nettingSets: NettingSetResult[];
  counterpartyResults: CounterpartyXvaResult[];
  hierarchy: HierarchySummary;
  computationTimeMs: number;
}

export interface XvaConfigSummary {
  nPaths: number;
  timePoints: number;
  horizonYears: number;
  antithetic: boolean;
  bilateral: boolean;
  computeFva: boolean;
  pfePercentiles: number[];
}

export interface NettingSetResult {
  nettingSetId: string;
  epe: number[];
  ene: number[];
  ecb: number[];
  pfe: PfeProfile[];
  peakEpe: number;
  peakEne: number;
  avgEpe: number;
  avgEne: number;
}

export interface PfeProfile {
  percentile: number;
  label: string;
  values: number[];
  peak: number;
}

export interface CounterpartyXvaResult {
  counterpartyId: string;
  creditRating: string;
  hazardRate: number;
  lgd: number;
  ucva: number;
  udva: number;
  bcva: number;
  bdva: number;
  fca: number;
  fba: number;
  fva: number;
  totalXva: number;
  nettingSetCount: number;
  tradeCount: number;
}

export interface HierarchySummary {
  counterparties: HierarchyCounterparty[];
  totalCounterparties: number;
  totalNettingSets: number;
  totalTrades: number;
}

export interface HierarchyCounterparty {
  id: string;
  creditRating: string;
  isdaAgreements: HierarchyIsda[];
  noDocTradeCount: number;
}

export interface HierarchyIsda {
  nettingSetId: string;
  vmCsas: HierarchyVmCsa[];
  nonCsaTradeCount: number;
}

export interface HierarchyVmCsa {
  csaId: string;
  thresholdSelf: number;
  thresholdCtpy: number;
  mtaSelf: number;
  mtaCtpy: number;
  mporDays: number;
  tradeCount: number;
}

export interface XvaBilateralRequest {
  epe: number[];
  ene: number[];
  timeGrid: number[];
  hazardRate: number;
  lgd: number;
  ownHazardRate: number;
  ownLgd: number;
  fundingSpread?: number;
  xccyBasis?: number;
}

export interface XvaBilateralResponse {
  ucva: number;
  udva: number;
  bcva: number;
  bdva: number;
  fca: number;
  fba: number;
  fva: number;
  totalXva: number;
  computationTimeMs: number;
}

export interface XvaCsvExportResponse {
  csvData: string;
  nettingSetId: string;
  rowCount: number;
}
