/**
 * Shared Market Environment store.
 *
 * Holds curves and vol surfaces published from CurveBuilder / VolSurface,
 * making them available to the Pricer as market data overrides.
 */

import { defineStore } from 'pinia';
import { ref } from 'vue';
import type { VolcubeCalibrateResponse, PricerGraphResponse, JyCurveBuildResponse } from '@/types/api';

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

export interface PublishedPricerGraph {
  id: string;
  label: string;
  instrumentType: string;
  instrumentName: string;
  publishedAt: number;
  graphResponse: PricerGraphResponse;
  detailLevel: 'operation' | 'scope';
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

export interface PublishedInflationCurves {
  id: string;
  label: string;
  publishedAt: number;
  curveResult: JyCurveBuildResponse;
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const useMarketEnvStore = defineStore('marketEnv', () => {
  const curves = ref<PublishedCurve[]>([]);
  const volSurfaces = ref<PublishedVolSurface[]>([]);
  const inflationCurves = ref<PublishedInflationCurves[]>([]);
  const pricerGraphs = ref<PublishedPricerGraph[]>([]);

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

  function publishInflationCurves(
    curveResult: JyCurveBuildResponse,
  ) {
    const ts = Date.now();
    const time = new Date(ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    const id = `custom-inflation-${ts}`;
    inflationCurves.value.unshift({
      id,
      label: `JY Inflation Curves (${time})`,
      publishedAt: ts,
      curveResult: { ...curveResult },
    });
    return id;
  }

  function removeInflationCurves(id: string) {
    inflationCurves.value = inflationCurves.value.filter((c) => c.id !== id);
  }

  function getInflationCurves(id: string) {
    return inflationCurves.value.find((c) => c.id === id);
  }

  function publishPricerGraph(
    instrumentType: string,
    instrumentName: string,
    graphResponse: PricerGraphResponse,
    detailLevel: 'operation' | 'scope',
  ) {
    const ts = Date.now();
    const time = new Date(ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    const id = `graph-${instrumentType}-${ts}`;
    pricerGraphs.value.unshift({
      id,
      label: `${instrumentName} (${time})`,
      instrumentType,
      instrumentName,
      publishedAt: ts,
      graphResponse: { ...graphResponse },
      detailLevel,
    });
    return id;
  }

  function removePricerGraph(id: string) {
    pricerGraphs.value = pricerGraphs.value.filter((g) => g.id !== id);
  }

  function getPricerGraph(id: string) {
    return pricerGraphs.value.find((g) => g.id === id);
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
    inflationCurves,
    pricerGraphs,
    publishCurve,
    publishVolSurface,
    publishInflationCurves,
    publishPricerGraph,
    removeCurve,
    removeVolSurface,
    removeInflationCurves,
    removePricerGraph,
    getCurve,
    getVolSurface,
    getInflationCurves,
    getPricerGraph,
  };
});
