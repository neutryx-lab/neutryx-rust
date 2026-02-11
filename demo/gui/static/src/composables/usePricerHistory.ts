/**
 * Composable for result history management, restoration, and comparison mode.
 *
 * Tracks up to 5 recent pricing results and allows restoring or comparing them.
 */

import { usePricerStore } from '@/stores/pricer';
import { useToast } from '@/composables/useToast';
import type { HistoryEntry } from '@/constants/pricer';

/** Deep-clone plain data, stripping Vue reactive proxies and non-serialisable values. */
function deepClone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value));
}

let historyCounter = 0;

export function usePricerHistory() {
  const store = usePricerStore();
  const toast = useToast();

  /**
   * Capture the current pricing result as a history entry (max 5, FIFO).
   */
  function addToHistory(): void {
    if (!store.pricingResult) return;

    const entry: HistoryEntry = {
      id: ++historyCounter,
      timestamp: Date.now(),
      instrumentId: store.selectedInstrumentId,
      instrumentName:
        store.selectedInstrument?.displayName || store.selectedInstrument?.name || store.selectedInstrumentId,
      params: { ...store.instrumentParams },
      pricingResult: deepClone(store.pricingResult),
      greeksResult: store.greeksResult ? deepClone(store.greeksResult) : null,
      valuationDate: store.valuationDate,
      reportingCcy: store.reportingCcy,
      modelType: store.modelType,
      curveIndex: store.selectedCurveIndex,
    };

    store.resultHistory.unshift(entry);
    if (store.resultHistory.length > 5) {
      store.resultHistory = store.resultHistory.slice(0, 5);
    }
  }

  /**
   * Restore all parameters and results from a history entry.
   */
  function restoreFromHistory(entry: HistoryEntry): void {
    store.selectedInstrumentId = entry.instrumentId;
    store.instrumentParams = { ...entry.params };
    store.valuationDate = entry.valuationDate;
    store.reportingCcy = entry.reportingCcy;
    store.modelType = entry.modelType;
    store.selectedCurveIndex = entry.curveIndex;
    store.pricingResult = deepClone(entry.pricingResult);
    store.greeksResult = entry.greeksResult ? deepClone(entry.greeksResult) : null;
    toast.info('Restored pricing result from history');
  }

  /**
   * Toggle comparison mode on/off.
   */
  function toggleCompareMode(): void {
    store.compareMode = !store.compareMode;
  }

  return { addToHistory, restoreFromHistory, toggleCompareMode };
}
