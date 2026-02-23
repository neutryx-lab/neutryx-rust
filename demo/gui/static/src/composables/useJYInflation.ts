/**
 * Composable for JY Inflation Model business logic.
 *
 * Wraps API calls, error handling, loading states, and toast notifications
 * for the 6-step JY inflation workflow.
 */

import { useToast } from '@/composables/useToast';
import { useJyInflationStore } from '@/stores/jyInflation';
import {
  jyBuildCurves,
  jyInstrumentCashflows,
  jySimulate,
  jyPrice,
  runIncrementalXva,
} from '@/services/api';

export function useJYInflation() {
  const store = useJyInflationStore();
  const toast = useToast();

  async function buildCurves() {
    store.loading = true;
    try {
      const request = {
        nominalCurveRef: store.nominalCurveRef,
        realRates: store.realRates,
        valuationDate: store.valuationDate,
        modelParams: store.modelParams,
        correlation: store.correlation,
      };
      store.curveResult = await jyBuildCurves(request);
      toast.success('Curves built successfully');
    } catch (e) {
      toast.error(`Curve building failed: ${e instanceof Error ? e.message : 'Unknown error'}`);
    } finally {
      store.loading = false;
    }
  }

  async function generateCashflows() {
    store.loading = true;
    try {
      const request = {
        instrumentType: store.instrumentType,
        notional: store.notional,
        fixedRate: store.fixedRate,
        startDate: store.startDate,
        maturityDate: store.maturityDate,
        paymentFrequency: store.paymentFrequency,
        nominalCurveRate: store.initialNominalRate,
        realCurveRate: store.initialRealRate,
      };
      store.instrumentResult = await jyInstrumentCashflows(request);
      toast.success(`${store.instrumentResult.cashflows.length} cashflows generated`);
    } catch (e) {
      toast.error(`Cashflow generation failed: ${e instanceof Error ? e.message : 'Unknown error'}`);
    } finally {
      store.loading = false;
    }
  }

  async function runSimulation() {
    store.loading = true;
    try {
      const request = {
        modelParams: store.modelParams,
        correlation: store.correlation,
        numPaths: store.numPaths,
        numSteps: store.numSteps,
        horizon: store.horizon,
        initialNominalRate: store.initialNominalRate,
        initialRealRate: store.initialRealRate,
        initialIndex: store.initialIndex,
        numSamplePaths: store.numSamplePaths,
      };
      store.simulationResult = await jySimulate(request);
      toast.success(`Simulation complete: ${store.numPaths} paths`);
    } catch (e) {
      toast.error(`Simulation failed: ${e instanceof Error ? e.message : 'Unknown error'}`);
    } finally {
      store.loading = false;
    }
  }

  async function runPricing() {
    store.loading = true;
    try {
      const request = {
        modelParams: store.modelParams,
        correlation: store.correlation,
        initialNominalRate: store.initialNominalRate,
        initialRealRate: store.initialRealRate,
        initialIndex: store.initialIndex,
        notional: store.notional,
        fixedRate: store.fixedRate,
        maturity: store.maturityYears,
        nominalCurveRate: store.initialNominalRate,
        realCurveRate: store.initialRealRate,
      };
      store.pricingResult = await jyPrice(request);
      toast.success(`MtM: ${store.formatCcy(store.pricingResult.mtm)}`);
    } catch (e) {
      toast.error(`Pricing failed: ${e instanceof Error ? e.message : 'Unknown error'}`);
    } finally {
      store.loading = false;
    }
  }

  async function runXva() {
    store.loading = true;
    try {
      // Convert PD → hazard rate: h ≈ -ln(1 - pd)
      const cptyHazard = -Math.log(1 - store.counterpartyPd);
      const ownHazard = -Math.log(1 - store.ownPd);

      const request = {
        nPaths: store.xvaNumPaths,
        horizonYears: store.maturityYears,
        timeStep: 'quarterly' as const,
        antithetic: true,
        bilateral: true,
        computeFva: true,
        // HW1F = JY nominal rate parameters
        hwMeanReversion: store.modelParams.aN,
        hwVolatility: store.modelParams.sigmaN,
        hwInitialRate: store.initialNominalRate,
        couplingMethod: 'swap_rate',
        // Credit: PD → hazard, recovery → LGD
        hazardRate: cptyHazard,
        lgd: 1 - store.counterpartyRecovery,
        ownHazardRate: ownHazard,
        ownLgd: 1 - store.ownRecovery,
        fundingSpread: store.fundingSpread,
        // JY 3-factor parameters
        jyRealMeanReversion: store.modelParams.aR,
        jyRealVolatility: store.modelParams.sigmaR,
        jyInitialRealRate: store.initialRealRate,
        jyInflationVolatility: store.modelParams.sigmaI,
        jyInitialIndex: store.initialIndex,
        jyRhoNominalReal: store.correlation.rhoNr,
        jyRhoNominalInflation: store.correlation.rhoNi,
        jyRhoRealInflation: store.correlation.rhoRi,
        // Portfolio: single inflation swap as incremental
        baseSwaps: [],
        baseExotics: [],
        baseInflationSwaps: [],
        incrementalTrade: {
          type: 'inflationSwap' as const,
          tradeId: 'ZCIS_JY',
          notional: store.notional,
          fixedRate: store.fixedRate,
          maturityYears: store.maturityYears,
          baseIndex: store.initialIndex,
        },
      };

      const result = await runIncrementalXva(request);

      // Map incremental XVA response → JyXvaResponse shape
      const xva = result.incrementalXva;
      store.xvaResult = {
        cva: xva.bcva,
        dva: xva.bdva,
        fva: xva.fva,
        totalXva: xva.total,
        cleanMtm: 0,
        adjustedMtm: xva.total,
        exposureProfile: {
          timeGrid: result.timeGrid,
          expectedExposure: result.fullEpe,
          negativeExpectedExposure: result.fullEne,
          pfe95: [],
          pfe99: [],
        },
      };
      toast.success(`XVA computed: CVA=${store.formatCcy(xva.bcva)}`);
    } catch (e) {
      toast.error(`XVA computation failed: ${e instanceof Error ? e.message : 'Unknown error'}`);
    } finally {
      store.loading = false;
    }
  }

  return {
    store,
    buildCurves,
    generateCashflows,
    runSimulation,
    runPricing,
    runXva,
  };
}
