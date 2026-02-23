/**
 * Pinia store for JY Inflation Model state.
 *
 * Holds reactive state for the 6-step JY inflation workflow:
 * Input -> Instrument -> Curve Build -> Simulation -> Pricing -> XVA
 */

import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import type {
  JyModelParams,
  JyCorrelation,
  JyCurveBuildResponse,
  JyInstrumentResponse,
  JySimulationResponse,
  JyPricingResponse,
  JyXvaResponse,
  CurveRatePoint,
  InflationIndexData,
} from '@/types/api';
import { fetchInflationMarketData } from '@/services/api';

export const useJyInflationStore = defineStore('jyInflation', () => {
  // ---------------------------------------------------------------------------
  // State
  // ---------------------------------------------------------------------------

  // Model Parameters
  const modelParams = ref<JyModelParams>({
    aN: 0.03,
    sigmaN: 0.01,
    aR: 0.02,
    sigmaR: 0.008,
    sigmaI: 0.02,
  });

  const correlation = ref<JyCorrelation>({
    rhoNr: 0.5,
    rhoNi: -0.2,
    rhoRi: -0.3,
  });

  const initialNominalRate = ref(0.035);
  const initialRealRate = ref(0.01);
  const initialIndex = ref(100.0);
  const valuationDate = ref(new Date().toISOString().split('T')[0]);

  // Market data (loaded from input files via API)
  const inflationIndices = ref<InflationIndexData[]>([]);
  const realRates = ref<CurveRatePoint[]>([]);
  // Selected nominal curve from the Rates system (e.g. "USD-SOFR")
  const nominalCurveRef = ref('');
  const marketDataLoaded = ref(false);
  const inflationIndex = ref('CPI-U');
  const referenceDate = ref('');

  // Instrument
  const instrumentType = ref('ZCIS');
  const notional = ref(10_000_000);
  const fixedRate = ref(0.025);
  const startDate = ref(new Date().toISOString().split('T')[0]);
  const maturityYears = ref(5);
  const paymentFrequency = ref('annual');

  // Simulation
  const numPaths = ref(5000);
  const numSteps = ref(100);
  const horizon = ref(5.0);
  const numSamplePaths = ref(5);

  // XVA
  const counterpartyPd = ref(0.01);
  const counterpartyRecovery = ref(0.4);
  const ownPd = ref(0.005);
  const ownRecovery = ref(0.4);
  const fundingSpread = ref(0.005);
  const xvaNumPaths = ref(5000);
  const xvaNumSteps = ref(50);

  // Results
  const curveResult = ref<JyCurveBuildResponse | null>(null);
  const instrumentResult = ref<JyInstrumentResponse | null>(null);
  const simulationResult = ref<JySimulationResponse | null>(null);
  const pricingResult = ref<JyPricingResponse | null>(null);
  const xvaResult = ref<JyXvaResponse | null>(null);

  // Loading states
  const loading = ref(false);

  // ---------------------------------------------------------------------------
  // Computed
  // ---------------------------------------------------------------------------

  const maturityDate = computed(() => {
    const start = new Date(startDate.value);
    start.setFullYear(start.getFullYear() + maturityYears.value);
    return start.toISOString().split('T')[0];
  });

  const summaryStats = computed(() => {
    const instCount = realRates.value.length;
    const built = curveResult.value != null;

    return [
      { label: 'Valuation Date', value: valuationDate.value || '-', icon: 'fa-calendar', color: '#8b5cf6' },
      { label: 'Instruments', value: `${instCount} TIPS`, icon: 'fa-list-alt', color: '#3b82f6' },
      { label: 'Model', value: 'Jarrow-Yildirim', icon: 'fa-wave-square', color: '#10b981' },
      { label: 'Status', value: built ? 'Built' : 'Pending', icon: 'fa-info-circle', color: built ? '#10b981' : '#f59e0b' },
    ];
  });

  // ---------------------------------------------------------------------------
  // Helpers
  // ---------------------------------------------------------------------------

  function formatCcy(value: number): string {
    const abs = Math.abs(value);
    const sign = value < 0 ? '-' : '';
    if (abs >= 1e6) return `${sign}$${(abs / 1e6).toFixed(2)}M`;
    if (abs >= 1e3) return `${sign}$${(abs / 1e3).toFixed(1)}K`;
    return `${sign}$${abs.toFixed(2)}`;
  }

  // ---------------------------------------------------------------------------
  // Actions
  // ---------------------------------------------------------------------------

  let _loadingPromise: Promise<void> | null = null;

  async function loadMarketData() {
    if (marketDataLoaded.value) return;
    if (_loadingPromise) return _loadingPromise;
    _loadingPromise = (async () => {
      try {
        const data = await fetchInflationMarketData();
        inflationIndices.value = data.indices;
        // Use first index as default for Pricer workflow
        const first = data.indices[0];
        if (first) {
          realRates.value = first.instruments;
          inflationIndex.value = first.inflationIndex;
          referenceDate.value = first.referenceDate;
          if (first.referenceDate) {
            valuationDate.value = first.referenceDate;
          }
        }
        marketDataLoaded.value = true;
      } catch (e) {
        console.error('Failed to load inflation market data:', e);
      } finally {
        _loadingPromise = null;
      }
    })();
    return _loadingPromise;
  }

  async function refreshMarketData() {
    marketDataLoaded.value = false;
    _loadingPromise = null;
    await loadMarketData();
  }

  // ---------------------------------------------------------------------------
  // Return
  // ---------------------------------------------------------------------------

  return {
    // State
    modelParams,
    correlation,
    initialNominalRate,
    initialRealRate,
    initialIndex,
    valuationDate,
    inflationIndices,
    realRates,
    nominalCurveRef,
    marketDataLoaded,
    inflationIndex,
    referenceDate,
    instrumentType,
    notional,
    fixedRate,
    startDate,
    maturityYears,
    paymentFrequency,
    numPaths,
    numSteps,
    horizon,
    numSamplePaths,
    counterpartyPd,
    counterpartyRecovery,
    ownPd,
    ownRecovery,
    fundingSpread,
    xvaNumPaths,
    xvaNumSteps,
    curveResult,
    instrumentResult,
    simulationResult,
    pricingResult,
    xvaResult,
    loading,
    // Computed
    maturityDate,
    summaryStats,
    // Actions
    loadMarketData,
    refreshMarketData,
    // Helpers
    formatCcy,
  };
});
