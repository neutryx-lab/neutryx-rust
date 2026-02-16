/**
 * Shared Market Environment store.
 *
 * Holds curves and vol surfaces published from CurveBuilder / VolSurface,
 * making them available to the Pricer via computed dropdown items.
 */

import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { CURVE_OPTIONS } from '@/constants/pricer';
import type { VolcubeCalibrateResponse } from '@/types/api';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface BuildResultSnapshot {
  curve_id?: string;
  instrument_count?: number;
  interpolation?: string;
  calculation_time_ms?: number;
  pillars?: unknown[];
  forward_curve?: unknown[];
  short_term_grid?: unknown[];
  long_term_grid?: unknown[];
  converged?: boolean;
  bootstrap_method?: string;
}

export interface PublishedCurve {
  id: string;
  label: string;
  curveName: string;
  currency: string;
  publishedAt: number;
  buildResult: BuildResultSnapshot;
  interpolation: string;
  calibrationMethod: string;
}

export interface PublishedVolSurface {
  id: string;
  label: string;
  indexOrPair: string;
  assetType: 'swaption' | 'fx';
  publishedAt: number;
  calibrationResult: VolcubeCalibrateResponse;
  model: string;
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const useMarketEnvStore = defineStore('marketEnv', () => {
  const curves = ref<PublishedCurve[]>([]);
  const volSurfaces = ref<PublishedVolSurface[]>([]);

  // -- Computed: dropdown items for Pricer -----------------------------------

  const allCurveItems = computed(() => {
    const defaults = CURVE_OPTIONS.map((c) => ({ title: c.label, value: c.index }));
    const custom = curves.value.map((c) => ({ title: c.label, value: c.id }));
    return custom.length > 0
      ? [...defaults, { title: '--- Published ---', value: '__divider__', props: { disabled: true } }, ...custom]
      : defaults;
  });

  const allVolSurfaceItems = computed(() =>
    volSurfaces.value.map((v) => ({ title: v.label, value: v.id })),
  );

  // -- Actions ---------------------------------------------------------------

  function publishCurve(
    curveName: string,
    currency: string,
    buildResult: BuildResultSnapshot,
    interpolation: string,
    calibrationMethod: string,
  ) {
    const ts = Date.now();
    const time = new Date(ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    const id = `custom-${curveName}-${ts}`;
    curves.value.unshift({
      id,
      label: `${curveName} (${time})`,
      curveName,
      currency,
      publishedAt: ts,
      buildResult: { ...buildResult },
      interpolation,
      calibrationMethod,
    });
    return id;
  }

  function publishVolSurface(
    indexOrPair: string,
    assetType: 'swaption' | 'fx',
    calibrationResult: VolcubeCalibrateResponse,
    model: string,
  ) {
    const ts = Date.now();
    const time = new Date(ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    const id = `custom-vol-${indexOrPair}-${ts}`;
    volSurfaces.value.unshift({
      id,
      label: `${indexOrPair} ${model} (${time})`,
      indexOrPair,
      assetType,
      publishedAt: ts,
      calibrationResult: { ...calibrationResult },
      model,
    });
    return id;
  }

  function removeCurve(id: string) {
    curves.value = curves.value.filter((c) => c.id !== id);
  }

  function removeVolSurface(id: string) {
    volSurfaces.value = volSurfaces.value.filter((v) => v.id !== id);
  }

  function getCurve(id: string) {
    return curves.value.find((c) => c.id === id);
  }

  function getVolSurface(id: string) {
    return volSurfaces.value.find((v) => v.id === id);
  }

  return {
    curves,
    volSurfaces,
    allCurveItems,
    allVolSurfaceItems,
    publishCurve,
    publishVolSurface,
    removeCurve,
    removeVolSurface,
    getCurve,
    getVolSurface,
  };
});
