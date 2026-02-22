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
  jyXva,
} from '@/services/api';

export function useJYInflation() {
  const store = useJyInflationStore();
  const toast = useToast();

  async function buildCurves() {
    store.loading = true;
    try {
      const request = {
        nominalRates: store.nominalRates,
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
        counterpartyPd: store.counterpartyPd,
        counterpartyRecovery: store.counterpartyRecovery,
        ownPd: store.ownPd,
        ownRecovery: store.ownRecovery,
        fundingSpread: store.fundingSpread,
        numPaths: store.xvaNumPaths,
        numSteps: store.xvaNumSteps,
      };
      store.xvaResult = await jyXva(request);
      toast.success(`XVA computed: CVA=${store.formatCcy(store.xvaResult.cva)}`);
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
