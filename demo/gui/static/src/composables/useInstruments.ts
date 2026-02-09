/**
 * Composable for instrument loading, selection, and IRS auto-configuration.
 *
 * Manages the instrument list, grouped display, selection,
 * and resets dependent state when the instrument changes.
 */

import { watch } from 'vue';
import { usePricerStore } from '@/stores/pricer';
import { useToast } from '@/composables/useToast';
import { fetchInstruments } from '@/services/api';

export function useInstruments() {
  const store = usePricerStore();
  const toast = useToast();

  // Flag to suppress the instrument-change watcher during auto-selection
  let skipWatcher = false;

  /**
   * Fetch the instrument catalogue from the API, populate the store,
   * and auto-select IRS with USD OIS 5Y defaults when available.
   */
  async function loadInstruments(): Promise<void> {
    try {
      const data = await fetchInstruments();
      store.instruments = data.instruments || [];

      // Auto-select IRS and set USD OIS 5Y defaults
      const irs = store.instruments.find((inst) =>
        ['IRS', 'irs'].includes(inst.instrumentType || inst.id || inst.type || ''),
      );
      if (irs) {
        skipWatcher = true;
        store.selectedInstrumentId = irs.instrumentType || irs.id || irs.type || 'IRS';

        const today = new Date();
        const fiveYears = new Date(today);
        fiveYears.setFullYear(fiveYears.getFullYear() + 5);

        store.instrumentParams = {
          notional: 1_000_000,
          currency: 'USD',
          startDate: today.toISOString().split('T')[0],
          endDate: fiveYears.toISOString().split('T')[0],
          fixedRate: 0.04,
        };
        skipWatcher = false;
      }
    } catch (error) {
      console.error('Failed to load instruments:', error);
      store.apiAvailable = false;
      toast.error('Failed to load instruments. API may be unavailable.');
    }
  }

  /**
   * Select an instrument by id.
   * Dependent state is reset automatically via the watcher.
   */
  function selectInstrument(id: string): void {
    store.selectedInstrumentId = id;
  }

  /**
   * Clear parameters, expanded trade, edits, and pricing results.
   */
  function resetDependentState(): void {
    store.instrumentParams = {};
    store.expandedTrade = null;
    store.editedCashflows = {};
    store.pricingResult = null;
    store.greeksResult = null;
  }

  // Reset dependent state when instrument changes (user-initiated)
  watch(
    () => store.selectedInstrumentId,
    () => {
      if (skipWatcher) return;
      resetDependentState();
    },
  );

  // Clear validation errors when params change
  watch(
    () => store.instrumentParams,
    () => {
      store.validationErrors = [];
    },
    { deep: true },
  );

  return { loadInstruments, selectInstrument, resetDependentState };
}
