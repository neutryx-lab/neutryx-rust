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
} from '@/types/api';

export const useJyInflationStore = defineStore('jyInflation', () => {
  // ---------------------------------------------------------------------------
  // State
  // ---------------------------------------------------------------------------

  // Active step (0-5)
  const activeStep = ref(0);

  // Model Parameters (Step 0: Input)
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

  // Market data (default USD swap rates)
  const nominalRates = ref<CurveRatePoint[]>([
    { instrumentType: 'Deposit', tenor: '3M', rate: 0.032 },
    { instrumentType: 'Swap', tenor: '1Y', rate: 0.035 },
    { instrumentType: 'Swap', tenor: '2Y', rate: 0.037 },
    { instrumentType: 'Swap', tenor: '3Y', rate: 0.038 },
    { instrumentType: 'Swap', tenor: '5Y', rate: 0.04 },
    { instrumentType: 'Swap', tenor: '7Y', rate: 0.041 },
    { instrumentType: 'Swap', tenor: '10Y', rate: 0.042 },
    { instrumentType: 'Swap', tenor: '15Y', rate: 0.043 },
    { instrumentType: 'Swap', tenor: '20Y', rate: 0.044 },
    { instrumentType: 'Swap', tenor: '30Y', rate: 0.045 },
  ]);

  // Real rates (TIPS yields)
  const realRates = ref<CurveRatePoint[]>([
    { instrumentType: 'TIPS', tenor: '1Y', rate: 0.008 },
    { instrumentType: 'TIPS', tenor: '2Y', rate: 0.009 },
    { instrumentType: 'TIPS', tenor: '3Y', rate: 0.01 },
    { instrumentType: 'TIPS', tenor: '5Y', rate: 0.012 },
    { instrumentType: 'TIPS', tenor: '7Y', rate: 0.014 },
    { instrumentType: 'TIPS', tenor: '10Y', rate: 0.015 },
    { instrumentType: 'TIPS', tenor: '20Y', rate: 0.017 },
    { instrumentType: 'TIPS', tenor: '30Y', rate: 0.018 },
  ]);

  // Instrument (Step 1)
  const instrumentType = ref('ZCIS');
  const notional = ref(10_000_000);
  const fixedRate = ref(0.025);
  const startDate = ref(new Date().toISOString().split('T')[0]);
  const maturityYears = ref(5);
  const paymentFrequency = ref('annual');

  // Simulation (Step 3)
  const numPaths = ref(5000);
  const numSteps = ref(100);
  const horizon = ref(5.0);
  const numSamplePaths = ref(5);

  // XVA (Step 5)
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
    const step = ['Input', 'Instrument', 'Curves', 'Simulation', 'Pricing', 'XVA'][activeStep.value];
    const inst = instrumentType.value;
    const mtm = pricingResult.value ? formatCcy(pricingResult.value.mtm) : '-';
    const cva = xvaResult.value ? formatCcy(xvaResult.value.cva) : '-';

    return [
      { label: 'Step', value: step, icon: 'fa-list-ol', color: '#10b981' },
      { label: 'Instrument', value: `${inst} ${maturityYears.value}Y`, icon: 'fa-file-contract', color: '#3b82f6' },
      { label: 'MtM', value: mtm, icon: 'fa-dollar-sign', color: '#8b5cf6' },
      { label: 'CVA', value: cva, icon: 'fa-shield-alt', color: '#ef4444' },
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
  // Return
  // ---------------------------------------------------------------------------

  return {
    // State
    activeStep,
    modelParams,
    correlation,
    initialNominalRate,
    initialRealRate,
    initialIndex,
    valuationDate,
    nominalRates,
    realRates,
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
    // Helpers
    formatCcy,
  };
});
