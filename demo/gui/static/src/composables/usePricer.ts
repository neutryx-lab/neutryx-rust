/**
 * Composable orchestrating the full pricing flow:
 * validation → trade expansion → pricing → metrics.
 *
 * Builds a PricingRequest that mirrors Rust CalcSetting.
 */

import { usePricerStore } from '@/stores/pricer';
import { useToast } from '@/composables/useToast';
import { useCashflowEditor } from '@/composables/useCashflowEditor';

import { expandTrade, priceTrade, calculateGreeks } from '@/services/api';
import type { PricingRequest, GreeksRequest, TradeExpandRequest } from '@/types/api';
import type { ValidationError } from '@/constants/pricer';

export function usePricer() {
  const store = usePricerStore();
  const toast = useToast();
  const { buildPricingLegs } = useCashflowEditor();

  /**
   * Validate required instrument parameters.
   */
  function validateParams(): ValidationError[] {
    const errors: ValidationError[] = [];

    if (!store.selectedInstrumentId) {
      errors.push({ field: 'instrumentType', message: 'Please select an instrument' });
    }

    const inst = store.selectedInstrument;
    if (inst?.requiredParams) {
      inst.requiredParams.forEach((param) => {
        const val = store.instrumentParams[param.name];

        if (val === undefined || val === '' || val === null) {
          errors.push({
            field: param.name,
            message: `${param.label || param.name} is required`,
          });
        }

        if (param.validation && val !== undefined && val !== '') {
          const numVal = Number(val);
          if (param.validation.min !== undefined && numVal < param.validation.min) {
            errors.push({
              field: param.name,
              message: `Minimum value is ${param.validation.min}`,
            });
          }
          if (param.validation.max !== undefined && numVal > param.validation.max) {
            errors.push({
              field: param.name,
              message: `Maximum value is ${param.validation.max}`,
            });
          }
        }
      });
    }

    return errors;
  }

  /**
   * Validate parameters, expand the trade via API, and update the store.
   */
  async function expandCashflows(): Promise<void> {
    if (!store.selectedInstrumentId) return;

    store.validationErrors = validateParams();
    if (store.validationErrors.length > 0) {
      toast.warning('Please fix validation errors before expanding.');
      return;
    }

    store.isExpanding = true;
    try {
      const result = await expandTrade({
        instrumentType: store.selectedInstrumentId,
        params: { type: store.selectedInstrumentId, ...store.instrumentParams },
      } as TradeExpandRequest);

      store.expandedTrade = result;
      store.editedCashflows = {};
      store.pricingResult = null;
      store.greeksResult = null;
    } catch (error) {
      console.error('Failed to expand cashflows:', error);
      toast.error(`Failed to expand cashflows: ${(error as Error).message}`);
    } finally {
      store.isExpanding = false;
    }
  }

  /**
   * Build a PricingRequest mirroring CalcSetting.
   */
  function buildPricingRequest(legs: ReturnType<typeof buildPricingLegs>): PricingRequest {
    const method = store.pricingMethod;

    const mcConfig = (method === 'monteCarlo' || method === 'auto')
      ? { numPaths: store.mcNumPaths, numSteps: store.mcNumSteps, seed: store.mcSeed }
      : null;

    const treeConfig = (method === 'tree' || method === 'auto')
      ? { numSteps: store.treeNumSteps, treeType: store.treeType }
      : null;

    return {
      valuationDate: store.valuationDate,
      reportingCurrency: store.reportingCcy,
      legs,
      method,
      computeGreeks: store.computeGreeks,
      mcConfig,
      treeConfig,
    };
  }

  /**
   * Run pricing (with optional inline Greeks), update the store,
   * and record computation metrics.
   */
  async function calculateAll(): Promise<void> {
    if (!store.selectedInstrumentId || !store.expandedTrade) return;

    store.isCalculating = true;
    const startTime = performance.now();

    try {
      const legs = buildPricingLegs();
      const request = buildPricingRequest(legs);

      // Run pricing (Greeks are inline if computeGreeks is true).
      const priceResult = await priceTrade(request);
      const endTime = performance.now();

      store.pricingResult = priceResult;

      // Backward-compat: also run separate Greeks endpoint if computeGreeks is off
      // (for the old GreeksDisplay component).
      if (!store.computeGreeks) {
        try {
          const greeksRequest: GreeksRequest = {
            ...request,
            bumpSizes: {
              rateBumpBp: store.rateBump,
              fxBumpPct: store.fxBump,
              volBumpPct: store.volBump,
            },
          };
          store.greeksResult = await calculateGreeks(greeksRequest);
        } catch {
          // Non-fatal
        }
      } else {
        store.greeksResult = null;
      }

      store.computationMetrics = {
        pricingTimeMs: priceResult.computationTimeMs ?? (endTime - startTime),
        method: priceResult.method ?? store.pricingMethod,
        timestamp: Date.now(),
      };

      // Save to history for Greeks Analyser.
      const inst = store.selectedInstrument;
      store.resultHistory.unshift({
        id: crypto.randomUUID(),
        timestamp: Date.now(),
        instrumentId: store.selectedInstrumentId,
        instrumentName: inst?.displayName || inst?.name || store.selectedInstrumentId,
        valuationDate: store.valuationDate,
        reportingCcy: store.reportingCcy,
        totalPv: priceResult.totalPv,
        legs,
        pricingResult: priceResult,
      });
      if (store.resultHistory.length > 50) store.resultHistory.length = 50;

      toast.success('Pricing complete');
    } catch (error) {
      console.error('Calculation failed:', error);
      toast.error(`Calculation failed: ${(error as Error).message}`);
    } finally {
      store.isCalculating = false;
    }
  }

  /**
   * Clear all expansion and pricing results.
   */
  function resetAll(): void {
    store.expandedTrade = null;
    store.editedCashflows = {};
    store.pricingResult = null;
    store.greeksResult = null;
    store.computationMetrics = null;
  }

  return { expandCashflows, calculateAll, resetAll, validateParams };
}
