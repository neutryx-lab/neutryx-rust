/**
 * Composable for cashflow editing and pricing-leg construction.
 *
 * Tracks per-cashflow overrides (notional / rate) in the store
 * and builds the PricingLeg array with edited values applied.
 */

import { usePricerStore } from '@/stores/pricer';
import type { PricingLeg } from '@/types/api';

export function useCashflowEditor() {
  const store = usePricerStore();

  /**
   * Record an edit for a single cashflow field.
   */
  function updateCashflow(
    legIdx: number,
    cfIdx: number,
    field: 'notional' | 'rate',
    value: number,
  ): void {
    const key = `${legIdx}-${cfIdx}`;
    if (!store.editedCashflows[key]) store.editedCashflows[key] = {};
    store.editedCashflows[key][field] = value;
  }

  /**
   * Discard all cashflow edits.
   */
  function resetEdits(): void {
    store.editedCashflows = {};
  }

  /**
   * Build PricingLeg[] from the expanded trade, applying any edits.
   * Raw values are passed through — the backend computes the amount.
   */
  function buildPricingLegs(): PricingLeg[] {
    const legs: PricingLeg[] = [];

    if (store.expandedTrade?.legs) {
      store.expandedTrade.legs.forEach((leg, legIdx) => {
        const cashflows = leg.cashflows.map((cf, cfIdx) => {
          const key = `${legIdx}-${cfIdx}`;
          const edited = store.editedCashflows[key] || {};
          const notional = edited.notional !== undefined ? edited.notional : cf.notional;
          const rate = edited.rate !== undefined ? edited.rate : cf.rate;
          return {
            paymentDate: cf.paymentDate,
            notional,
            rate,
            yearFraction: cf.yearFraction,
            payoffType: cf.payoffType,
            rateIndex: cf.rateIndex,
            accrualStart: cf.accrualStart,
            accrualEnd: cf.accrualEnd,
          };
        });

        legs.push({
          currency: leg.currency,
          direction: leg.direction.toLowerCase() as 'payer' | 'receiver',
          cashflows,
        });
      });
    }

    return legs;
  }

  return { updateCashflow, resetEdits, buildPricingLegs };
}
