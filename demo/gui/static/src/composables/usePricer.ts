/**
 * Composable orchestrating the full pricing flow:
 * validation → trade expansion → pricing + Greeks → metrics.
 *
 * Depends on useCashflowEditor for building pricing legs.
 * History integration is added in Phase 3 (task 6.3).
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
   * Run priceTrade and calculateGreeks in parallel, update the store,
   * and record computation metrics. Greeks failure is non-fatal.
   */
  async function calculateAll(): Promise<void> {
    if (!store.selectedInstrumentId || !store.expandedTrade) return;

    store.isCalculating = true;
    const startTime = performance.now();

    try {
      const legs = buildPricingLegs();

      // Build request with typed fields + extra backend fields
      const baseRequest = {
        valuationDate: store.valuationDate,
        reportingCurrency: store.reportingCcy,
        legs,
        modelConfig: store.useDefaults
          ? null
          : { numPaths: store.numPaths, numSteps: store.numSteps, seed: store.seed },
        curveIndex: store.selectedCurveIndex,
        modelType: store.modelType,
        modelParams: { ...store.modelParams },
      };

      const greeksRequest = {
        ...baseRequest,
        bumpSizes: {
          rateBumpBp: store.rateBump,
          fxBumpPct: store.fxBump,
          volBumpPct: store.volBump,
        },
      };

      const [priceResult, greeksResult] = await Promise.allSettled([
        priceTrade(baseRequest as PricingRequest),
        calculateGreeks(greeksRequest as GreeksRequest),
      ]);

      const endTime = performance.now();

      if (priceResult.status === 'fulfilled') {
        store.pricingResult = priceResult.value;
      } else {
        toast.error(`Pricing failed: ${priceResult.reason?.message || 'Unknown error'}`);
      }

      if (greeksResult.status === 'fulfilled') {
        store.greeksResult = greeksResult.value;
      } else {
        toast.warning('Greeks calculation failed. PV result may still be valid.');
      }

      store.computationMetrics = {
        pricingTimeMs: endTime - startTime,
        method: store.modelType,
        timestamp: Date.now(),
      };

      if (store.pricingResult) {
        toast.success('Pricing complete');
      }
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
