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
   * Build default parameter values from an instrument's param definitions.
   * Uses the API-provided `defaultValue` for each param, with sensible
   * fallbacks for date fields (today / today + 5Y).
   */
  function buildDefaults(inst: (typeof store.instruments)[number]): Record<string, string | number> {
    const params: Record<string, string | number> = {};
    const today = new Date();

    for (const p of [...inst.requiredParams, ...(inst.optionalParams ?? [])]) {
      if (p.defaultValue !== undefined && p.defaultValue !== null) {
        params[p.name] = p.defaultValue as string | number;
      } else if (p.fieldType === 'date') {
        // Sensible date fallbacks: first date = today, second = today + 5Y
        if (p.name.toLowerCase().includes('end') || p.name.toLowerCase().includes('maturity')) {
          const future = new Date(today);
          future.setFullYear(future.getFullYear() + 5);
          params[p.name] = future.toISOString().split('T')[0];
        } else {
          params[p.name] = today.toISOString().split('T')[0];
        }
      }
    }
    return params;
  }

  /**
   * Fetch the instrument catalogue from the API, populate the store,
   * and auto-select the first instrument with API-provided defaults.
   */
  async function loadInstruments(): Promise<void> {
    try {
      const data = await fetchInstruments();
      store.instruments = data.instruments || [];

      // Set initial asset tab and auto-select first instrument
      if (store.assetClasses.length > 0) {
        store.assetTab = store.assetClasses[0];
      }
      const first = store.filteredInstruments[0] ?? store.instruments[0];
      if (first) {
        skipWatcher = true;
        store.selectedInstrumentId = first.instrumentType || first.id || first.type || '';
        store.instrumentParams = buildDefaults(first);
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
   * If the newly selected instrument has defaultValues, apply them.
   */
  function resetDependentState(): void {
    const inst = store.selectedInstrument;
    store.instrumentParams = inst ? buildDefaults(inst) : {};
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

  // Auto-select first instrument when asset tab changes
  watch(
    () => store.assetTab,
    () => {
      const first = store.filteredInstruments[0];
      if (first) {
        skipWatcher = true;
        store.selectedInstrumentId = first.instrumentType || first.id || first.type || '';
        store.instrumentParams = buildDefaults(first);
        store.expandedTrade = null;
        store.editedCashflows = {};
        store.pricingResult = null;
        store.greeksResult = null;
        skipWatcher = false;
      }
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
