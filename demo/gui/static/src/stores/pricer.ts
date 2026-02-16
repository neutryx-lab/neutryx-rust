/**
 * Pinia store for Pricer state management.
 *
 * Centralises all Pricer-related reactive state using Composition API style.
 * Business logic is delegated to composables; this store holds state and computed getters only.
 */

import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import type { Instrument, ExpandedTrade, PricingResult, GreeksResult, PricingMethod, TreeTypeOption } from '@/types/api';
import { formatCurrency } from '@/utils/format';
import {
  STOCHASTIC_MODELS,
  type CashflowEdit,
  type ValidationError,
  type ComputationMetrics,
  type SummaryStat,
  type CurrencyAgg,
} from '@/constants/pricer';

export const usePricerStore = defineStore('pricer', () => {
  // ---------------------------------------------------------------------------
  // State
  // ---------------------------------------------------------------------------

  // Instrument
  const instruments = ref<Instrument[]>([]);
  const selectedInstrumentId = ref('');
  const instrumentParams = ref<Record<string, string | number>>({});

  // Trade Expansion
  const expandedTrade = ref<ExpandedTrade | null>(null);

  // Cashflow Edits
  const editedCashflows = ref<Record<string, CashflowEdit>>({});

  // Pricing Results
  const pricingResult = ref<PricingResult | null>(null);
  const greeksResult = ref<GreeksResult | null>(null);

  // CalcSetting (mirrors Rust CalcSetting)
  const pricingMethod = ref<PricingMethod>('auto');
  const computeGreeks = ref(false);
  const reportingCcy = ref('USD');

  // MonteCarloSetting
  const mcNumPaths = ref(10000);
  const mcNumSteps = ref(100);
  const mcSeed = ref<number | null>(null);

  // TreeSetting
  const treeNumSteps = ref(100);
  const treeType = ref<TreeTypeOption>('binomial');

  // Valuation
  const valuationDate = ref(new Date().toISOString().split('T')[0]);
  const useDefaults = ref(true);
  const numPaths = ref(10000);
  const numSteps = ref(100);
  const seed = ref<number | null>(null);

  // Bump Sizes
  const rateBump = ref(1);
  const fxBump = ref(1);
  const volBump = ref(1);

  // Market Data
  const selectedCurveIndex = ref('USD-SOFR');
  const selectedVolSurfaceId = ref('');

  // Stochastic Model
  const modelType = ref('GBM');
  const modelParams = ref<Record<string, number>>({ drift: 0.05, vol: 0.2 });

  // UI State
  const isExpanding = ref(false);
  const isCalculating = ref(false);
  const apiAvailable = ref(true);

  // Validation
  const validationErrors = ref<ValidationError[]>([]);

  // Metrics
  const computationMetrics = ref<ComputationMetrics | null>(null);

  // ---------------------------------------------------------------------------
  // Getters
  // ---------------------------------------------------------------------------

  const selectedInstrument = computed<Instrument | undefined>(() =>
    instruments.value.find(
      (inst) => (inst.instrumentType || inst.id || inst.type) === selectedInstrumentId.value,
    ),
  );

  const groupedInstruments = computed<Record<string, Instrument[]>>(() => {
    const groups: Record<string, Instrument[]> = {};
    instruments.value.forEach((inst) => {
      const assetClass = inst.assetClassName || inst.assetClass || 'Other';
      if (!groups[assetClass]) groups[assetClass] = [];
      groups[assetClass].push(inst);
    });
    return groups;
  });

  const hasEdits = computed(() => Object.keys(editedCashflows.value).length > 0);

  const summaryStats = computed<SummaryStat[]>(() => {
    const pvValue = pricingResult.value
      ? formatCurrency(pricingResult.value.totalPv ?? 0)
      : '-';
    const dv01Value = pricingResult.value?.greeks?.delta != null
      ? formatCurrency(pricingResult.value.greeks.delta)
      : greeksResult.value ? formatCurrency(greeksResult.value.delta) : '-';

    return [
      { label: 'Valuation Date', value: valuationDate.value, icon: 'fa-calendar', color: '#10b981' },
      {
        label: 'Instrument',
        value: selectedInstrument.value?.displayName || selectedInstrument.value?.name || '-',
        icon: 'fa-file-contract',
        color: '#3b82f6',
      },
      { label: 'PV', value: pvValue, icon: 'fa-dollar-sign', color: '#8b5cf6' },
      { label: 'DV01', value: dv01Value, icon: 'fa-chart-line', color: '#f59e0b' },
    ];
  });

  const selectedModelConfig = computed(
    () => STOCHASTIC_MODELS.find((m) => m.type === modelType.value) || STOCHASTIC_MODELS[0],
  );

  const currencyAggregation = computed<CurrencyAgg[]>(() => {
    if (!pricingResult.value?.legs) return [];
    const byCcy: Record<string, number> = {};
    pricingResult.value.legs.forEach((leg) => {
      const ccy = leg.currency || reportingCcy.value;
      byCcy[ccy] = (byCcy[ccy] || 0) + leg.pv;
    });
    return Object.entries(byCcy).map(([ccy, pv]) => ({ ccy, pv }));
  });

  // ---------------------------------------------------------------------------
  // Return
  // ---------------------------------------------------------------------------

  return {
    // State
    instruments,
    selectedInstrumentId,
    instrumentParams,
    expandedTrade,
    editedCashflows,
    pricingResult,
    greeksResult,
    // CalcSetting
    pricingMethod,
    computeGreeks,
    reportingCcy,
    mcNumPaths,
    mcNumSteps,
    mcSeed,
    treeNumSteps,
    treeType,
    // Valuation
    valuationDate,
    useDefaults,
    numPaths,
    numSteps,
    seed,
    rateBump,
    fxBump,
    volBump,
    selectedCurveIndex,
    selectedVolSurfaceId,
    modelType,
    modelParams,
    isExpanding,
    isCalculating,
    apiAvailable,
    validationErrors,
    computationMetrics,
    // Getters
    selectedInstrument,
    groupedInstruments,
    hasEdits,
    summaryStats,
    selectedModelConfig,
    currencyAggregation,
  };
});
