<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue';
import { Chart, type ChartDataset, registerables } from 'chart.js';
import { getChartColors } from '@/composables/useChartTheme';
import AssetTabBar from '@/components/common/AssetTabBar.vue';
import type {
  SwaptionInstrument,
  FxVolQuote,
  VolcubeCalibrateResponse,
  CapFloorQuote,
  CapFloorCalibrateResponse,
  CapFloorCalibrationMethod,
} from '@/types';
import {
  fetchVolcubeIndices,
  fetchVolcubeModels,
  fetchVolcubeInstruments,
  fetchFxVolPairs,
  fetchFxVolQuotes,
  calibrateVolcube,
  calibrateFxVol,
  computeModelSmile,
  fetchCapFloorInstruments,
  calibrateCapFloor,
} from '@/services/api';
import { useMarketEnvStore } from '@/stores/marketEnv';
import { useJyInflationStore } from '@/stores/jyInflation';
import { useJYInflation } from '@/composables/useJYInflation';
import JyModelParamsPanel from '@/components/jy/JyModelParamsPanel.vue';
import JySimulationPanel from '@/components/jy/JySimulationPanel.vue';

Chart.register(...registerables);

// ── Constants (F-2) ─────────────────────────────────────────────────────────
const EXPIRY_ORDER = ['1M', '3M', '6M', '1Y', '2Y', '5Y', '10Y', '15Y', '20Y', '30Y'];
const TENOR_ORDER = ['1Y', '2Y', '5Y', '10Y', '15Y', '20Y', '30Y'];
const UNKNOWN_SORT_ORDER = 999;
const DEFAULT_FORWARD_RATE = 0.03;
const SMILE_N_POINTS = 101;
const SMILE_RANGE_BP = 200;
const POPOVER_WIDTH = 256; // matches w-64
const ERROR_AUTO_DISMISS_MS = 8000;

type AssetTab = 'rates' | 'fx' | 'inflation';
type SabrParam = 'alpha' | 'beta' | 'rho' | 'nu';

// ── Model parameter definitions ──────────────────────────────────────────────
interface ModelParamDef {
  key: string;
  symbol: string;
  default: number;
  step: number;
}

const MODEL_PARAMS: Record<string, ModelParamDef[]> = {
  'SABR': [
    { key: 'alpha', symbol: '\u03B1', default: 0.03, step: 0.01 },
    { key: 'beta', symbol: '\u03B2', default: 0, step: 0.1 },
    { key: 'rho', symbol: '\u03C1', default: -0.3, step: 0.01 },
    { key: 'nu', symbol: '\u03BD', default: 0.4, step: 0.01 },
  ],
  'SVI': [
    { key: 'a', symbol: 'a', default: 0.04, step: 0.01 },
    { key: 'b', symbol: 'b', default: 0.1, step: 0.01 },
    { key: 'rho', symbol: '\u03C1', default: -0.3, step: 0.01 },
    { key: 'm', symbol: 'm', default: 0.0, step: 0.01 },
    { key: 'sigma', symbol: '\u03C3', default: 0.1, step: 0.01 },
  ],
  'SSVI': [
    { key: 'rho', symbol: '\u03C1', default: -0.3, step: 0.01 },
    { key: 'eta', symbol: '\u03B7', default: 0.5, step: 0.01 },
    { key: 'gamma', symbol: '\u03B3', default: 0.5, step: 0.01 },
    { key: 'atmVol', symbol: '\u03C3\u2080', default: 0.2, step: 0.01 },
  ],
  'Vanna-Volga': [
    { key: 'sigmaAtm', symbol: '\u03C3_ATM', default: 0.10, step: 0.01 },
    { key: 'sigma25dPut', symbol: '\u03C3_P25', default: 0.12, step: 0.01 },
    { key: 'sigma25dCall', symbol: '\u03C3_C25', default: 0.09, step: 0.01 },
  ],
  'ZABR': [
    { key: 'alpha', symbol: '\u03B1', default: 0.2, step: 0.01 },
    { key: 'beta', symbol: '\u03B2', default: 0.5, step: 0.1 },
    { key: 'nu', symbol: '\u03BD', default: 0.4, step: 0.01 },
    { key: 'rho', symbol: '\u03C1', default: -0.3, step: 0.01 },
    { key: 'gammaMix', symbol: '\u03B3', default: 0.0, step: 0.01 },
  ],
  'Mixture Lognormal': [
    { key: 'weight1', symbol: 'w\u2081', default: 0.6, step: 0.05 },
    { key: 'sigma1', symbol: '\u03C3\u2081', default: 0.15, step: 0.01 },
    { key: 'sigma2', symbol: '\u03C3\u2082', default: 0.30, step: 0.01 },
  ],
  'Polynomial': [
    { key: 'c0', symbol: 'c\u2080', default: 0.04, step: 0.01 },
    { key: 'c1', symbol: 'c\u2081', default: 0.0, step: 0.01 },
    { key: 'c2', symbol: 'c\u2082', default: 0.01, step: 0.001 },
  ],
  'Variance Gamma': [
    { key: 'sigma', symbol: '\u03C3', default: 0.2, step: 0.01 },
    { key: 'nu', symbol: '\u03BD', default: 0.5, step: 0.01 },
    { key: 'theta', symbol: '\u03B8', default: -0.1, step: 0.01 },
  ],
  'Black-Scholes': [
    { key: 'vol', symbol: '\u03C3', default: 0.2, step: 0.01 },
  ],
};

/** Map display name → backend API model string. */
function modelApiName(displayName: string): string {
  const map: Record<string, string> = {
    'SABR': 'sabr', 'SVI': 'svi', 'SSVI': 'ssvi',
    'Vanna-Volga': 'vanna_volga', 'ZABR': 'zabr',
    'Mixture Lognormal': 'mixture_lognormal',
    'Polynomial': 'polynomial', 'Variance Gamma': 'variance_gamma',
    'Local Volatility': 'local_volatility', 'Black-Scholes': 'black_scholes',
  };
  return map[displayName] || displayName.toLowerCase().replace(/[- ]/g, '_');
}

/** Build model-specific params object for the smile API. */
function buildSmileParams(model: string, values: Record<string, number>, forward: number): Record<string, unknown> {
  switch (model) {
    case 'Polynomial':
      return { coefficients: [values.c0 ?? 0.04, values.c1 ?? 0, values.c2 ?? 0.01] };
    case 'Vanna-Volga':
      return {
        ...values,
        strikeAtm: forward,
        strike25dPut: forward * 0.95,
        strike25dCall: forward * 1.05,
      };
    case 'Mixture Lognormal': {
      const w1 = values.weight1 ?? 0.6;
      return {
        weights: [w1, 1 - w1],
        forwards: [forward, forward],
        volatilities: [values.sigma1 ?? 0.15, values.sigma2 ?? 0.30],
      };
    }
    default:
      return { ...values };
  }
}

// ── Market Environment ───────────────────────────────────────────────────────
const marketEnv = useMarketEnvStore();

// ── JY Inflation ─────────────────────────────────────────────────────────────
const jyStore = useJyInflationStore();
const { runSimulation: jyRunSimulation } = useJYInflation();
const volPublishFeedback = ref(false);

function publishVolToEnvironment() {
  if (!calibrationResult.value) return;
  const indexOrPair = activeTab.value === 'rates' ? selectedSwaptionIndex.value : selectedFxPair.value;
  const assetType = activeTab.value === 'rates' ? 'swaption' as const : 'fx' as const;
  marketEnv.publishVolSurface(indexOrPair, assetType, calibrationResult.value, selectedModel.value);
  volPublishFeedback.value = true;
  setTimeout(() => { volPublishFeedback.value = false; }, 2000);
}

// ── State ────────────────────────────────────────────────────────────────────
const activeTab = ref<AssetTab>('rates');
const swaptionIndices = ref<string[]>([]);
const selectedSwaptionIndex = ref('');
const swaptionInstruments = ref<SwaptionInstrument[]>([]);
const swaptionModels = ref<string[]>([]);
const selectedModel = ref('');
const referenceDate = ref('');

// Forward swap rate matrix state
const fwdSwapRates = ref<Map<string, number>>(new Map());
const isBuildingCurve = ref(false);
const curveError = ref<string | null>(null); // D-4
const matrixTab = ref<'vol' | 'capfloor' | 'fwd'>('vol');
const paramTab = ref<SabrParam>('alpha');

// Cap/Floor state
const capFloorQuotes = ref<CapFloorQuote[]>([]);
const capFloorCalibMethod = ref<CapFloorCalibrationMethod>('bootstrap');
const capFloorCalibResult = ref<CapFloorCalibrateResponse | null>(null);
const isCapFloorCalibrating = ref(false);

const fxPairs = ref<string[]>([]);
const selectedFxPair = ref('');
const fxQuotes = ref<FxVolQuote[]>([]);
const fxSpot = ref('');
const fxDomesticRate = ref('0');
const fxForeignRate = ref('0');

// SABR parameter settings (initial values + fixed flags) — used for calibration
const sabrInitial = ref<Record<SabrParam, number>>({ alpha: 0.03, beta: 0, rho: -0.3, nu: 0.4 });
const sabrFixed = ref<Record<SabrParam, boolean>>({ alpha: false, beta: true, rho: false, nu: false });

// Generic model parameters (for non-SABR smile exploration)
const modelParams = ref<Record<string, number>>({});

const isSabrModel = computed(() => selectedModel.value === 'SABR');
const activeModelParamDefs = computed(() => MODEL_PARAMS[selectedModel.value] ?? []);

/** Initialise modelParams with defaults for the given model. */
function resetModelParams(model: string) {
  const defs = MODEL_PARAMS[model];
  if (!defs) { modelParams.value = {}; return; }
  const vals: Record<string, number> = {};
  for (const d of defs) vals[d.key] = d.default;
  modelParams.value = vals;
}

const calibrationResult = ref<VolcubeCalibrateResponse | null>(null);
const isCalibrating = ref(false);
const isLoadingData = ref(false); // D-2

// Error state (D-1)
const errorMessage = ref<string | null>(null);
let errorTimer: ReturnType<typeof setTimeout> | null = null;

function showError(msg: string) {
  errorMessage.value = msg;
  if (errorTimer) clearTimeout(errorTimer);
  errorTimer = setTimeout(() => { errorMessage.value = null; }, ERROR_AUTO_DISMISS_MS);
}

// Popover state (transient — cleared on outside click)
const popoverCell = ref<{ expiry: string; tenor: string } | null>(null);
const popoverPosition = ref<{ top: number; left: number }>({ top: 0, left: 0 });

// Selected cell state (persistent — survives outside click, cleared on next selection or close)
const selectedCell = ref<{ expiry: string; tenor: string } | null>(null);

// Detail card chart state (Swaption)
const smileChartCanvas = ref<HTMLCanvasElement | null>(null);
const pdfChartCanvas = ref<HTMLCanvasElement | null>(null);
let smileChartInstance: Chart | null = null;
let pdfChartInstance: Chart | null = null;

// FX selected row + chart state
const selectedFxTenor = ref<string | null>(null);
const fxSmileChartCanvas = ref<HTMLCanvasElement | null>(null);
const fxPdfChartCanvas = ref<HTMLCanvasElement | null>(null);
let fxSmileChartInstance: Chart | null = null;
let fxPdfChartInstance: Chart | null = null;

// AbortController for in-flight requests (E-2)
let activeAbortController: AbortController | null = null;

function newAbortSignal(): AbortSignal {
  if (activeAbortController) activeAbortController.abort();
  activeAbortController = new AbortController();
  return activeAbortController.signal;
}

// ── Matrix computed properties ───────────────────────────────────────────────
const instrumentMap = computed(() => {
  const map = new Map<string, SwaptionInstrument>();
  for (const inst of swaptionInstruments.value) {
    map.set(`${inst.expiry}|${inst.tenor}`, inst);
  }
  return map;
});

function sortByOrder(labels: string[], order: string[]): string[] {
  return [...labels].sort((a, b) => {
    const idxA = order.indexOf(a);
    const idxB = order.indexOf(b);
    return (idxA === -1 ? UNKNOWN_SORT_ORDER : idxA) - (idxB === -1 ? UNKNOWN_SORT_ORDER : idxB);
  });
}

const matrixExpiries = computed(() => {
  const expiries = [...new Set(swaptionInstruments.value.map(i => i.expiry))];
  return sortByOrder(expiries, EXPIRY_ORDER);
});

const matrixTenors = computed(() => {
  const tenors = [...new Set(swaptionInstruments.value.map(i => i.tenor))];
  return sortByOrder(tenors, TENOR_ORDER);
});

const volRange = computed(() => {
  const vols = swaptionInstruments.value.map(i => i.atmVol);
  if (vols.length === 0) return { min: 0, max: 1 };
  return { min: Math.min(...vols), max: Math.max(...vols) };
});

function getCell(expiry: string, tenor: string): SwaptionInstrument | undefined {
  return instrumentMap.value.get(`${expiry}|${tenor}`);
}

const popoverInstrument = computed(() => {
  if (!popoverCell.value) return null;
  return getCell(popoverCell.value.expiry, popoverCell.value.tenor) ?? null;
});

const selectedInstrument = computed(() => {
  if (!selectedCell.value) return null;
  return getCell(selectedCell.value.expiry, selectedCell.value.tenor) ?? null;
});

const selectedCellParams = computed(() => {
  if (!selectedCell.value || !calibrationResult.value?.cellParameters) return null;
  const key = `${selectedCell.value.expiry}|${selectedCell.value.tenor}`;
  return calibrationResult.value.cellParameters[key] ?? null;
});

const selectedCellJacobian = computed(() => {
  if (!selectedCell.value || !calibrationResult.value?.cellJacobians) return null;
  const key = `${selectedCell.value.expiry}|${selectedCell.value.tenor}`;
  return calibrationResult.value.cellJacobians[key] ?? null;
});

const selectedFxJacobian = computed(() => {
  if (!selectedFxTenor.value || !calibrationResult.value?.cellJacobians) return null;
  return calibrationResult.value.cellJacobians[selectedFxTenor.value] ?? null;
});

// Jacobian heatmap helpers (shared by swaption and FX cell detail)
function cellJacAbsMax(jac: { matrix: number[][] } | null): number {
  if (!jac) return 1;
  const vals = jac.matrix.flat().filter(v => v !== 0);
  return vals.length > 0 ? Math.max(...vals.map(Math.abs)) : 1;
}

function cellJacBg(value: number, absMax: number): string {
  if (absMax === 0 || value === 0) return 'transparent';
  const t = Math.min(Math.abs(value) / absMax, 1);
  if (value < 0) return `rgba(239, 68, 68, ${0.08 + t * 0.35})`;
  return `rgba(59, 130, 246, ${0.08 + t * 0.35})`;
}

function cellJacText(value: number, absMax: number): string {
  if (absMax === 0 || value === 0) return 'var(--text-muted)';
  const t = Math.min(Math.abs(value) / absMax, 1);
  if (t > 0.4) return value < 0 ? '#f87171' : '#60a5fa';
  return 'var(--text-secondary)';
}

// ── FX delta-vol computed ───────────────────────────────────────────────────
const fxDeltaVols = computed(() =>
  fxQuotes.value.map(q => ({
    tenor: q.expiryLabel,
    expiry: q.expiry,
    forward: q.forward,
    put10: q.rr10d != null && q.bf10d != null ? q.atmVol + q.bf10d - q.rr10d / 2 : null,
    put25: q.atmVol + q.bf25d - q.rr25d / 2,
    atm: q.atmVol,
    call25: q.atmVol + q.bf25d + q.rr25d / 2,
    call10: q.rr10d != null && q.bf10d != null ? q.atmVol + q.bf10d + q.rr10d / 2 : null,
  }))
);

const selectedFxParams = computed(() => {
  if (!selectedFxTenor.value || !calibrationResult.value?.cellParameters) return null;
  return calibrationResult.value.cellParameters[selectedFxTenor.value] ?? null;
});

const selectedFxQuote = computed(() => {
  if (!selectedFxTenor.value) return null;
  return fxQuotes.value.find(q => q.expiryLabel === selectedFxTenor.value) ?? null;
});

// ── Unified heatmap colour functions (F-1) ──────────────────────────────────
function rangedHeatmapBg(val: number, range: { min: number; max: number }): string {
  const { min, max } = range;
  if (max === min) return 'rgba(99, 102, 241, 0.15)';
  const t = Math.max(0, Math.min(1, (val - min) / (max - min)));
  const hue = 220 - t * 205;
  const saturation = 60 + t * 20;
  const lightness = 45 + (1 - Math.abs(t - 0.5) * 2) * 10;
  return `hsla(${hue}, ${saturation}%, ${lightness}%, 0.25)`;
}

function rangedHeatmapText(val: number, range: { min: number; max: number }): string {
  const { min, max } = range;
  if (max === min) return 'var(--text-primary)';
  const t = Math.max(0, Math.min(1, (val - min) / (max - min)));
  if (t > 0.75) return '#f97316';
  if (t > 0.5) return '#22c55e';
  if (t > 0.25) return '#3b82f6';
  return 'var(--text-secondary)';
}

// Calibration param heatmap
const paramRange = computed(() => {
  const cp = calibrationResult.value?.cellParameters;
  if (!cp) return { min: 0, max: 1 };
  const key = paramTab.value;
  const vals = Object.values(cp).map(p => p[key]);
  if (vals.length === 0) return { min: 0, max: 1 };
  return { min: Math.min(...vals), max: Math.max(...vals) };
});

// Forward swap rate heatmap helpers
const fwdRateRange = computed(() => {
  const vals = [...fwdSwapRates.value.values()].filter(v => v > 0);
  if (vals.length === 0) return { min: 0, max: 1 };
  return { min: Math.min(...vals), max: Math.max(...vals) };
});

// Cap/Floor vol range for heatmap
const capFloorVolRange = computed(() => {
  const vols = capFloorQuotes.value.map(q => q.marketVol).filter(v => v > 0);
  if (vols.length === 0) return { min: 0, max: 1 };
  return { min: Math.min(...vols), max: Math.max(...vols) };
});

// ── Detail card: chart helpers ───────────────────────────────────────────────
function expiryToYears(expiry: string): number {
  const m = expiry.match(/^(\d+)(M|Y)$/);
  if (!m) return 1;
  const n = parseInt(m[1]);
  return m[2] === 'M' ? n / 12 : n;
}

function destroyCharts() {
  if (smileChartInstance) { smileChartInstance.destroy(); smileChartInstance = null; }
  if (pdfChartInstance) { pdfChartInstance.destroy(); pdfChartInstance = null; }
}

// Forward swap rate computation (delegated to backend)
async function buildCurveForFwdRates() {
  const rateFile = selectedSwaptionIndex.value;
  if (!rateFile) return;

  isBuildingCurve.value = true;
  curveError.value = null;
  const signal = newAbortSignal();
  try {
    // Load rate data
    const rateResp = await fetch(`/data/input/rates/${rateFile}.json`, { signal });
    if (!rateResp.ok) throw new Error(`Failed to load rate data for ${rateFile}`);
    const rateData = await rateResp.json();

    // Use deposits + OIS only (skip events, FRAs, futures to avoid duplicate maturities)
    const allowedTypes = new Set(['deposit', 'ois']);
    const instruments = (rateData.instruments || [])
      .filter((i: { type: string }) => allowedTypes.has(i.type))
      .map((i: { type: string; tenor: string; rate: number }) => ({
        instrument_type: i.type,
        tenor: i.tenor,
        rate: i.rate,
      }));

    // Build curve
    const buildResp = await fetch('/api/curves/build', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        index: rateData.index,
        currency: rateData.currency,
        reference_date: rateData.reference_date,
        instruments,
        interpolation: 'log_linear',
      }),
      signal,
    });
    if (!buildResp.ok) throw new Error('Curve build failed');
    const buildResult = await buildResp.json();

    // Compute forward swap rates via backend
    const fwdResp = await fetch('/api/curves/forward-swap-rates', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        curve_id: buildResult.curve_id,
        expiries: matrixExpiries.value,
        tenors: matrixTenors.value,
      }),
      signal,
    });
    if (!fwdResp.ok) throw new Error('Forward swap rate computation failed');
    const fwdResult = await fwdResp.json();

    fwdSwapRates.value = new Map(Object.entries(fwdResult.rates));
  } catch (error) {
    if ((error as Error).name === 'AbortError') return;
    const msg = error instanceof Error ? error.message : 'Curve build failed';
    curveError.value = msg;
    console.error('Failed to build curve for forward swap rates:', error);
  } finally {
    isBuildingCurve.value = false;
  }
}

async function renderDetailCharts() {
  if (!selectedCell.value) return;

  const cellParams = selectedCellParams.value;
  // For SABR: require calibrated params; for other models: use manual params
  if (isSabrModel.value && !cellParams) return;

  const inst = selectedInstrument.value;
  const cell = selectedCell.value;

  const cc = getChartColors();
  const axisStyle = {
    ticks: { color: cc.tick, font: { size: 10 } },
    grid: { color: cc.grid },
  };

  // Determine forward rate for this cell (C-2: use ?? instead of ||)
  const fwdKey = `${cell.expiry}|${cell.tenor}`;
  const forward = fwdSwapRates.value.get(fwdKey) ?? DEFAULT_FORWARD_RATE;

  try {
    // Build params: use calibrated SABR params when available, else manual model params
    const smileModel = selectedModel.value || 'SABR';
    const smileParamsRaw = isSabrModel.value && cellParams
      ? { alpha: cellParams.alpha, beta: cellParams.beta, rho: cellParams.rho, nu: cellParams.nu }
      : modelParams.value;
    const smileParamsObj = buildSmileParams(smileModel, smileParamsRaw, forward);

    const result = await computeModelSmile({
      model: modelApiName(smileModel),
      forward,
      expiryYears: expiryToYears(cell.expiry),
      nPoints: SMILE_N_POINTS,
      rangeBp: SMILE_RANGE_BP,
      params: smileParamsObj,
    });

    const smileLabels = result.offsets.map((o: number) => (o > 0 ? '+' : '') + Math.round(o));
    const smileVols = result.vols.map((v: number) => v * 100);

    // E-1: destroy existing chart before creating new one
    if (smileChartInstance) { smileChartInstance.destroy(); smileChartInstance = null; }

    // Smile chart
    if (smileChartCanvas.value) {
      const ctx = smileChartCanvas.value.getContext('2d');
      if (ctx) {
        const datasets: ChartDataset<'line'>[] = [{
          label: `${smileModel} Fitted`,
          data: smileVols,
          borderColor: '#6366f1',
          backgroundColor: 'rgba(99, 102, 241, 0.10)',
          borderWidth: 2,
          fill: true,
          tension: 0.3,
          pointRadius: 0,
        }];

        // Overlay market data points if available
        if (inst && inst.smile && inst.smile.length > 0) {
          const marketPts = [
            ...inst.smile.map(s => ({ k: s.strikeOffsetBp, v: s.vol * 100 })),
            { k: 0, v: inst.atmVol * 100 },
          ].sort((a, b) => a.k - b.k);
          const marketData = new Array(result.offsets.length).fill(null);
          for (const pt of marketPts) {
            let bestIdx = 0;
            let bestDist = Math.abs(result.offsets[0] - pt.k);
            for (let j = 1; j < result.offsets.length; j++) {
              const dist = Math.abs(result.offsets[j] - pt.k);
              if (dist < bestDist) { bestDist = dist; bestIdx = j; }
            }
            marketData[bestIdx] = pt.v;
          }
          datasets.push({
            label: 'Market',
            data: marketData,
            borderColor: '#f59e0b',
            borderWidth: 0,
            pointRadius: 5,
            pointBackgroundColor: '#f59e0b',
            pointBorderColor: '#f59e0b',
            showLine: false,
            fill: false,
          });
        }

        smileChartInstance = new Chart(ctx, {
          type: 'line',
          data: { labels: smileLabels, datasets },
          options: {
            responsive: true,
            maintainAspectRatio: false,
            plugins: {
              legend: { display: datasets.length > 1, labels: { color: cc.legend, font: { size: 10 } } },
              tooltip: {
                callbacks: {
                  title: (items: { label: string }[]) => `Strike: ${items[0].label} bp`,
                  label: (item: { raw: unknown }) => item.raw != null ? `Vol: ${(item.raw as number).toFixed(1)} bp` : '',
                },
              },
            },
            scales: {
              x: {
                ...axisStyle,
                title: { display: true, text: 'Strike Offset (bp)', color: cc.tick, font: { size: 10 } },
                ticks: { ...axisStyle.ticks, maxTicksLimit: 10 },
              },
              y: {
                ...axisStyle,
                title: { display: true, text: 'Normal Vol (bp)', color: cc.tick, font: { size: 10 } },
              },
            },
          },
        });
      }
    }

    // E-1: destroy existing PDF chart before creating new one
    if (pdfChartInstance) { pdfChartInstance.destroy(); pdfChartInstance = null; }

    // Density chart
    if (pdfChartCanvas.value) {
      const pdfLabels = result.offsets.map((o: number) => (o > 0 ? '+' : '') + Math.round(o));
      const ctx = pdfChartCanvas.value.getContext('2d');
      if (ctx) {
        pdfChartInstance = new Chart(ctx, {
          type: 'line',
          data: {
            labels: pdfLabels,
            datasets: [{
              data: result.density,
              borderColor: '#10b981',
              backgroundColor: 'rgba(16, 185, 129, 0.15)',
              borderWidth: 2,
              fill: true,
              tension: 0.3,
              pointRadius: 0,
            }],
          },
          options: {
            responsive: true,
            maintainAspectRatio: false,
            plugins: {
              legend: { display: false },
              tooltip: {
                callbacks: {
                  title: (items: { label: string }[]) => `Offset: ${items[0].label} bp`,
                  label: (item: { raw: unknown }) => item.raw != null ? `Density: ${(item.raw as number).toExponential(2)}` : '',
                },
              },
            },
            scales: {
              x: {
                ...axisStyle,
                title: { display: true, text: 'Strike Offset (bp)', color: cc.tick, font: { size: 10 } },
                ticks: { ...axisStyle.ticks, maxTicksLimit: 10 },
              },
              y: {
                ...axisStyle,
                title: { display: true, text: 'Density', color: cc.tick, font: { size: 10 } },
              },
            },
          },
        });
      }
    }
  } catch (error) {
    const msg = error instanceof Error ? error.message : 'Smile computation failed';
    showError(msg);
    console.error('Failed to compute smile:', error);
  }
}

// ── FX detail charts ─────────────────────────────────────────────────────────
function destroyFxCharts() {
  if (fxSmileChartInstance) { fxSmileChartInstance.destroy(); fxSmileChartInstance = null; }
  if (fxPdfChartInstance) { fxPdfChartInstance.destroy(); fxPdfChartInstance = null; }
}

async function renderFxDetailCharts() {
  if (!selectedFxTenor.value) return;
  const params = selectedFxParams.value;
  const quote = selectedFxQuote.value;
  // For SABR: require calibrated params; for other models: use manual params
  if (isSabrModel.value && !params) return;
  if (!quote) return;

  const forward = quote.forward ?? parseFloat(fxSpot.value);
  if (!forward || forward <= 0) return;

  const cc = getChartColors();
  const axisStyle = {
    ticks: { color: cc.tick, font: { size: 10 } },
    grid: { color: cc.grid },
  };

  try {
    const smileModel = selectedModel.value || 'SABR';
    const smileParamsRaw = isSabrModel.value && params
      ? { alpha: params.alpha, beta: params.beta, rho: params.rho, nu: params.nu }
      : modelParams.value;
    const smileParamsObj = buildSmileParams(smileModel, smileParamsRaw, forward);

    const result = await computeModelSmile({
      model: modelApiName(smileModel),
      forward,
      expiryYears: quote.expiry,
      nPoints: SMILE_N_POINTS,
      rangeBp: SMILE_RANGE_BP,
      params: smileParamsObj,
    });

    const smileLabels = result.offsets.map((o: number) => (o > 0 ? '+' : '') + Math.round(o));
    // Backend already sends vols in % — use directly for FX Black vol
    const smileVols = result.vols as number[];

    destroyFxCharts();

    // Smile chart
    if (fxSmileChartCanvas.value) {
      const ctx = fxSmileChartCanvas.value.getContext('2d');
      if (ctx) {
        fxSmileChartInstance = new Chart(ctx, {
          type: 'line',
          data: {
            labels: smileLabels,
            datasets: [{
              label: `${smileModel} Fitted`,
              data: smileVols,
              borderColor: '#6366f1',
              backgroundColor: 'rgba(99, 102, 241, 0.10)',
              borderWidth: 2,
              fill: true,
              tension: 0.3,
              pointRadius: 0,
            }],
          },
          options: {
            responsive: true,
            maintainAspectRatio: false,
            plugins: {
              legend: { display: false },
              tooltip: {
                callbacks: {
                  title: (items: { label: string }[]) => `Strike: ${items[0].label} bp`,
                  label: (item: { raw: unknown }) => item.raw != null ? `Vol: ${(item.raw as number).toFixed(2)}%` : '',
                },
              },
            },
            scales: {
              x: { ...axisStyle, title: { display: true, text: 'Strike Offset (bp)', color: cc.tick, font: { size: 10 } }, ticks: { ...axisStyle.ticks, maxTicksLimit: 10 } },
              y: { ...axisStyle, title: { display: true, text: 'Black Vol (%)', color: cc.tick, font: { size: 10 } } },
            },
          },
        });
      }
    }

    // Density chart
    if (fxPdfChartCanvas.value) {
      const pdfLabels = result.offsets.map((o: number) => (o > 0 ? '+' : '') + Math.round(o));
      const ctx = fxPdfChartCanvas.value.getContext('2d');
      if (ctx) {
        fxPdfChartInstance = new Chart(ctx, {
          type: 'line',
          data: {
            labels: pdfLabels,
            datasets: [{
              data: result.density,
              borderColor: '#10b981',
              backgroundColor: 'rgba(16, 185, 129, 0.15)',
              borderWidth: 2,
              fill: true,
              tension: 0.3,
              pointRadius: 0,
            }],
          },
          options: {
            responsive: true,
            maintainAspectRatio: false,
            plugins: {
              legend: { display: false },
              tooltip: {
                callbacks: {
                  title: (items: { label: string }[]) => `Offset: ${items[0].label} bp`,
                  label: (item: { raw: unknown }) => item.raw != null ? `Density: ${(item.raw as number).toExponential(2)}` : '',
                },
              },
            },
            scales: {
              x: { ...axisStyle, title: { display: true, text: 'Strike Offset (bp)', color: cc.tick, font: { size: 10 } }, ticks: { ...axisStyle.ticks, maxTicksLimit: 10 } },
              y: { ...axisStyle, title: { display: true, text: 'Density', color: cc.tick, font: { size: 10 } } },
            },
          },
        });
      }
    }
  } catch (error) {
    const msg = error instanceof Error ? error.message : 'Smile computation failed';
    showError(msg);
    console.error('Failed to compute FX smile:', error);
  }
}

// ── Summary stats ────────────────────────────────────────────────────────────
const summaryStats = computed(() => {
  if (activeTab.value === 'rates') {
    const instruments = swaptionInstruments.value;
    return [
      { label: 'Valuation Date', value: referenceDate.value || '-', icon: 'fa-calendar', color: '#8b5cf6' },
      { label: 'Instruments', value: instruments.length, icon: 'fa-th', color: '#3b82f6' },
      { label: 'Matrix', value: instruments.length > 0 ? `${matrixExpiries.value.length} x ${matrixTenors.value.length}` : '-', icon: 'fa-border-all', color: '#10b981' },
      { label: 'Status', value: calibrationResult.value ? 'Calibrated' : 'Pending', icon: 'fa-info-circle', color: calibrationResult.value ? '#10b981' : '#f59e0b' },
    ];
  }
  return [
    { label: 'Valuation Date', value: referenceDate.value || '-', icon: 'fa-calendar', color: '#8b5cf6' },
    { label: 'Selected Pair', value: selectedFxPair.value || '-', icon: 'fa-exchange-alt', color: '#10b981' },
    { label: 'Spot Rate', value: fxSpot.value || '-', icon: 'fa-dollar-sign', color: '#8b5cf6' },
    { label: 'Status', value: calibrationResult.value ? 'Calibrated' : 'Pending', icon: 'fa-info-circle', color: calibrationResult.value ? '#10b981' : '#f59e0b' },
  ];
});

// ── Utility functions ────────────────────────────────────────────────────────
function formatVol(vol: number): string {
  return `${(vol * 100).toFixed(1)} bp`;
}

function expiryToLabel(expiry: number): string {
  if (expiry < 0.05) return '1W';
  if (expiry < 0.125) return '1M';
  if (expiry < 0.33) return '3M';
  if (expiry < 0.54) return '6M';
  if (expiry < 1.5) return '1Y';
  if (expiry < 2.5) return '2Y';
  return `${Math.round(expiry)}Y`;
}

// ── Popover functions ────────────────────────────────────────────────────────
function togglePopover(event: MouseEvent | KeyboardEvent, expiry: string, tenor: string) {
  const cell = getCell(expiry, tenor);
  if (!cell || !cell.smile || cell.smile.length === 0) return;

  if (popoverCell.value?.expiry === expiry && popoverCell.value?.tenor === tenor) {
    popoverCell.value = null;
    return;
  }

  const target = (event.currentTarget ?? event.target) as HTMLElement;
  const container = target.closest('.matrix-container') as HTMLElement;
  if (!container) return;

  const targetRect = target.getBoundingClientRect();
  const containerRect = container.getBoundingClientRect();

  // G-1: Popover bounds checking
  const rawLeft = targetRect.left - containerRect.left + targetRect.width / 2;
  const halfPopover = POPOVER_WIDTH / 2;
  const clampedLeft = Math.max(halfPopover, Math.min(containerRect.width - halfPopover, rawLeft));

  popoverPosition.value = {
    top: targetRect.bottom - containerRect.top + 4,
    left: clampedLeft,
  };

  popoverCell.value = { expiry, tenor };
  selectedCell.value = { expiry, tenor };
}

function closePopover() {
  popoverCell.value = null;
}

function closeDetailCard() {
  selectedCell.value = null;
}

function selectCalibrationCell(expiry: string, tenor: string) {
  selectedCell.value = { expiry, tenor };
}

function onDocumentClick(event: MouseEvent) {
  const target = event.target as HTMLElement;
  if (!target.closest('.popover-trigger') && !target.closest('.smile-popover')) {
    popoverCell.value = null;
    // selectedCell intentionally NOT cleared — detail card persists
  }
}

// ── API calls (B-1: use service layer) ───────────────────────────────────────
async function loadSwaptionIndices() {
  try {
    const data = await fetchVolcubeIndices();
    swaptionIndices.value = data.indices || [];
    const usdIndex = swaptionIndices.value.find(idx => idx.startsWith('usd'));
    if (usdIndex && !selectedSwaptionIndex.value) {
      selectedSwaptionIndex.value = usdIndex;
    }
  } catch (error) {
    const msg = error instanceof Error ? error.message : 'Failed to load swaption indices';
    showError(msg);
    console.error('Failed to load swaption indices:', error);
  }
}

async function loadSwaptionModels() {
  try {
    const data = await fetchVolcubeModels();
    swaptionModels.value = data.models || [];
    if (swaptionModels.value.length > 0) {
      selectedModel.value = swaptionModels.value[0];
    }
  } catch (error) {
    const msg = error instanceof Error ? error.message : 'Failed to load calibration models';
    showError(msg);
    console.error('Failed to load calibration models:', error);
  }
}

async function loadSwaptionInstruments(index: string) {
  try {
    const currency = index.split('-')[0];
    const data = await fetchVolcubeInstruments(currency);
    swaptionInstruments.value = data.instruments || [];
    referenceDate.value = data.referenceDate || '';
    calibrationResult.value = null;
    capFloorCalibResult.value = null;
    popoverCell.value = null;
    selectedCell.value = null;
    // C-1: await the curve build + load cap/floor in parallel
    await Promise.all([
      buildCurveForFwdRates(),
      loadCapFloorInstruments(index),
    ]);
  } catch (error) {
    const msg = error instanceof Error ? error.message : 'Failed to load instruments';
    showError(msg);
    console.error('Failed to load instruments:', error);
  }
}

async function loadFxPairs() {
  try {
    const data = await fetchFxVolPairs();
    fxPairs.value = (data.pairs || []).map((p: { pair: string }) => p.pair);
    const eurUsd = fxPairs.value.find(p => p === 'EURUSD');
    if (eurUsd && !selectedFxPair.value) {
      selectedFxPair.value = eurUsd;
    }
  } catch (error) {
    const msg = error instanceof Error ? error.message : 'Failed to load FX pairs';
    showError(msg);
    console.error('Failed to load FX pairs:', error);
  }
}

async function loadFxQuotes(pair: string) {
  try {
    const data = await fetchFxVolQuotes(pair);
    fxQuotes.value = data.quotes || [];
    if (data.spot != null) {
      fxSpot.value = data.spot.toFixed(4);
    }
    if (data.domesticRate != null) {
      fxDomesticRate.value = (data.domesticRate * 100).toFixed(2);
    }
    if (data.foreignRate != null) {
      fxForeignRate.value = (data.foreignRate * 100).toFixed(2);
    }
    calibrationResult.value = null;
  } catch (error) {
    const msg = error instanceof Error ? error.message : 'Failed to load FX quotes';
    showError(msg);
    console.error('Failed to load FX quotes:', error);
  }
}

async function loadCapFloorInstruments(index: string) {
  try {
    const currency = index.split('-')[0];
    const data = await fetchCapFloorInstruments(currency);
    capFloorQuotes.value = data.instruments || [];
    capFloorCalibResult.value = null;
  } catch {
    // Cap/Floor data may not be available — silently ignore
    capFloorQuotes.value = [];
  }
}

async function calibrateCapFloorVol() {
  if (isCapFloorCalibrating.value || !selectedSwaptionIndex.value) return;
  if (capFloorQuotes.value.length === 0) return;

  isCapFloorCalibrating.value = true;
  try {
    capFloorCalibResult.value = await calibrateCapFloor({
      index: selectedSwaptionIndex.value.split('-')[0],
      referenceDate: referenceDate.value,
      method: capFloorCalibMethod.value,
      model: selectedModel.value,
      initialParams: {
        alpha: sabrInitial.value.alpha,
        beta: sabrInitial.value.beta,
        rho: sabrInitial.value.rho,
        nu: sabrInitial.value.nu,
      },
      fixedParams: {
        alpha: sabrFixed.value.alpha,
        beta: sabrFixed.value.beta,
        rho: sabrFixed.value.rho,
        nu: sabrFixed.value.nu,
      },
    });

    // Merge stripped caplet vols back into quotes for display
    if (capFloorCalibResult.value?.capletVols) {
      capFloorQuotes.value = capFloorQuotes.value.map(q => ({
        ...q,
        capletVol: capFloorCalibResult.value!.capletVols[q.maturity] ?? q.capletVol,
      }));
    }
  } catch (error) {
    const msg = error instanceof Error ? error.message : 'Cap/Floor calibration failed';
    showError(msg);
    console.error('Cap/Floor calibration failed:', error);
  } finally {
    isCapFloorCalibrating.value = false;
  }
}

async function calibrate() {
  // C-5: prevent multiple simultaneous calibrations
  if (isCalibrating.value) return;
  if (activeTab.value === 'rates' && !selectedSwaptionIndex.value) return;
  if (activeTab.value === 'fx' && !selectedFxPair.value) return;

  // C-4: validate FX inputs before sending
  if (activeTab.value === 'fx') {
    const spot = parseFloat(fxSpot.value);
    if (isNaN(spot) || spot <= 0) {
      showError('Invalid spot rate');
      return;
    }
    const domRate = parseFloat(fxDomesticRate.value);
    const forRate = parseFloat(fxForeignRate.value);
    if (isNaN(domRate) || isNaN(forRate)) {
      showError('Invalid interest rate');
      return;
    }
  }

  isCalibrating.value = true;
  try {
    if (activeTab.value === 'rates') {
      calibrationResult.value = await calibrateVolcube({
        index: selectedSwaptionIndex.value.split('-')[0],
        referenceDate: referenceDate.value,
        model: selectedModel.value,
        forwardRates: Object.fromEntries(fwdSwapRates.value),
        initialParams: {
          alpha: sabrInitial.value.alpha,
          beta: sabrInitial.value.beta,
          rho: sabrInitial.value.rho,
          nu: sabrInitial.value.nu,
        },
        fixedParams: {
          alpha: sabrFixed.value.alpha,
          beta: sabrFixed.value.beta,
          rho: sabrFixed.value.rho,
          nu: sabrFixed.value.nu,
        },
      });
    } else {
      selectedFxTenor.value = null;
      calibrationResult.value = await calibrateFxVol({
        pair: selectedFxPair.value,
        spot: parseFloat(fxSpot.value),
        domesticRate: parseFloat(fxDomesticRate.value) / 100,
        foreignRate: parseFloat(fxForeignRate.value) / 100,
        forwardRates: Object.fromEntries(
          fxQuotes.value
            .filter(q => q.forward != null)
            .map(q => [q.expiryLabel, q.forward!])
        ),
        initialParams: {
          alpha: sabrInitial.value.alpha,
          beta: sabrInitial.value.beta,
          rho: sabrInitial.value.rho,
          nu: sabrInitial.value.nu,
        },
        fixedParams: {
          alpha: sabrFixed.value.alpha,
          beta: sabrFixed.value.beta,
          rho: sabrFixed.value.rho,
          nu: sabrFixed.value.nu,
        },
      });
    }
  } catch (error) {
    const msg = error instanceof Error ? error.message : 'Calibration failed';
    showError(msg);
    console.error('Calibration failed:', error);
  } finally {
    isCalibrating.value = false;
  }
}

// F-5: Enhanced CSV export with cell parameters
function exportCsv() {
  if (!calibrationResult.value) return;

  const lines: string[] = [];

  // Global parameters
  lines.push('Section,Key,Alpha,Beta,Rho,Nu');
  const gp = calibrationResult.value.parameters;
  lines.push(`Global,--,${gp.alpha},${gp.beta},${gp.rho},${gp.nu}`);

  // Cell parameters
  if (calibrationResult.value.cellParameters) {
    for (const [key, cp] of Object.entries(calibrationResult.value.cellParameters)) {
      const [expiry, tenor] = key.split('|');
      lines.push(`Cell,${expiry}x${tenor},${cp.alpha},${cp.beta},${cp.rho},${cp.nu}`);
    }
  }

  downloadFile(lines.join('\n'), 'volcube_calibration.csv', 'text/csv');
}

function exportJson() {
  if (!calibrationResult.value) return;

  const json = JSON.stringify(calibrationResult.value, null, 2);
  downloadFile(json, 'volcube_calibration.json', 'application/json');
}

// E-3: downloadFile with try/finally for URL cleanup
function downloadFile(content: string, filename: string, mimeType: string) {
  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);
  try {
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
  } finally {
    URL.revokeObjectURL(url);
  }
}

// ── Watch for selection changes ──────────────────────────────────────────────
watch(activeTab, (tab) => {
  calibrationResult.value = null;
  capFloorCalibResult.value = null;
  selectedCell.value = null;
  popoverCell.value = null;
  selectedFxTenor.value = null;
  destroyFxCharts();

  // Reset SABR β default per asset class:
  //   Rates    → β=0 (Normal / Bachelier)
  //   FX       → β=1 (Lognormal / Black-Scholes)
  if (tab === 'fx') {
    sabrInitial.value.beta = 1;
  } else {
    sabrInitial.value.beta = 0;
  }
  sabrFixed.value.beta = true;
});

watch(selectedModel, (model) => {
  if (model) resetModelParams(model);
});

watch(selectedSwaptionIndex, (index) => {
  if (index) loadSwaptionInstruments(index);
});

watch(selectedFxPair, (pair) => {
  if (pair) loadFxQuotes(pair);
});

watch(selectedCell, () => {
  destroyCharts();
  nextTick(() => renderDetailCharts());
});

watch(selectedFxTenor, () => {
  destroyFxCharts();
  nextTick(() => renderFxDetailCharts());
});

// ── Lifecycle ────────────────────────────────────────────────────────────────
onMounted(() => {
  document.addEventListener('click', onDocumentClick);
});

onUnmounted(() => {
  document.removeEventListener('click', onDocumentClick);
  destroyCharts();
  destroyFxCharts();
  if (activeAbortController) activeAbortController.abort();
  if (errorTimer) clearTimeout(errorTimer);
});

// ── Initialize ───────────────────────────────────────────────────────────────
isLoadingData.value = true;
Promise.all([loadSwaptionIndices(), loadSwaptionModels(), loadFxPairs()])
  .finally(() => { isLoadingData.value = false; });
</script>

<template>
  <div class="volcube-builder-view">
    <!-- D-1: Error banner -->
    <div
      v-if="errorMessage"
      class="mb-4 px-4 py-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm flex items-center justify-between"
    >
      <div class="flex items-center gap-2">
        <i class="fas fa-exclamation-triangle"></i>
        <span>{{ errorMessage }}</span>
      </div>
      <button
        class="text-red-400 hover:text-red-300 ml-4"
        aria-label="Dismiss error"
        @click="errorMessage = null"
      >
        <i class="fas fa-times"></i>
      </button>
    </div>

    <!-- D-2: Loading indicator for initial data -->
    <div v-if="isLoadingData" class="text-center py-12">
      <i class="fas fa-spinner fa-spin text-2xl text-[var(--primary)] mb-3"></i>
      <p class="text-sm text-[var(--text-muted)]">Loading market data...</p>
    </div>

    <template v-else>
      <!-- Summary Stats -->
      <div class="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
        <div
          v-for="stat in summaryStats"
          :key="stat.label"
          class="glass-card p-4"
        >
          <div class="flex items-start justify-between">
            <div>
              <p class="text-sm text-[var(--text-muted)] mb-1">{{ stat.label }}</p>
              <p class="text-xl font-semibold text-[var(--text-primary)]">{{ stat.value }}</p>
            </div>
            <div
              class="w-9 h-9 rounded-lg flex items-center justify-center"
              :style="{ backgroundColor: `${stat.color}1a` }"
            >
              <i :class="['fas', stat.icon, 'text-sm']" :style="{ color: stat.color }"></i>
            </div>
          </div>
        </div>
      </div>

      <!-- Asset Tabs -->
      <AssetTabBar
        v-model="activeTab"
        class="mb-6"
        :tabs="[
          { key: 'rates', label: 'Rates', icon: 'fa-percentage' },
          { key: 'fx', label: 'FX', icon: 'fa-exchange-alt' },
          { key: 'inflation', label: 'Inflation', icon: 'fa-chart-bar' },
        ]"
      />

      <!-- Inflation Tab Content -->
      <div v-if="activeTab === 'inflation'" class="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <!-- Left Panel: JY Model Parameters -->
        <div class="space-y-4">
          <div class="glass-card p-5">
            <div class="section-header" style="margin-top: 0">JY Model Parameters</div>
            <JyModelParamsPanel />
          </div>

          <!-- Simulation Config -->
          <div class="glass-card p-5">
            <div class="section-header" style="margin-top: 0">Simulation Config</div>
            <div class="config-grid">
              <div class="grid-label">MC Paths</div>
              <div class="grid-input">
                <input v-model.number="jyStore.numPaths" type="number" min="100" max="100000" step="100"
                  class="param-input w-full" />
              </div>
              <div class="grid-label">Time Steps</div>
              <div class="grid-input">
                <input v-model.number="jyStore.numSteps" type="number" min="10" max="5000" step="10"
                  class="param-input w-full" />
              </div>
              <div class="grid-label">Horizon (Y)</div>
              <div class="grid-input">
                <input v-model.number="jyStore.horizon" type="number" min="0.1" max="50" step="0.5"
                  class="param-input w-full" />
              </div>
              <div class="grid-label">Sample Paths</div>
              <div class="grid-input">
                <input v-model.number="jyStore.numSamplePaths" type="number" min="0" max="20" step="1"
                  class="param-input w-full" />
              </div>
            </div>
            <button
              class="w-full mt-4 px-4 py-2 rounded-lg text-sm font-medium bg-[var(--primary)] text-white hover:opacity-90 transition-all disabled:bg-gray-500 disabled:cursor-not-allowed"
              :disabled="jyStore.loading"
              @click="jyRunSimulation"
            >
              <i :class="['fas mr-2', jyStore.loading ? 'fa-spinner fa-spin' : 'fa-play']"></i>
              {{ jyStore.loading ? 'Simulating...' : 'Run Simulation' }}
            </button>
          </div>
        </div>

        <!-- Right Panel: Simulation Results -->
        <div class="lg:col-span-2">
          <JySimulationPanel :result="jyStore.simulationResult" />
        </div>
      </div>

      <!-- Swaption / FX Content -->
      <div v-else class="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <!-- Left Panel: Settings -->
        <div class="space-y-4">
          <!-- Swaption Settings -->
          <template v-if="activeTab === 'rates'">
            <div class="glass-card p-5">
              <div class="section-header" style="margin-top: 0">Index Selection</div>
              <div class="config-grid">
                <div class="grid-label">Index</div>
                <div class="grid-input">
                  <v-select
                    v-model="selectedSwaptionIndex"
                    :items="swaptionIndices.map(i => ({ title: i.toUpperCase(), value: i }))"
                    placeholder="Select index..."
                    density="compact"
                    variant="outlined"
                    hide-details
                  />
                </div>
              </div>
            </div>

            <div class="glass-card p-5">
              <div class="section-header" style="margin-top: 0">Calibration Settings</div>
              <div class="config-grid">
                <div class="grid-label">Model</div>
                <div class="grid-input">
                  <v-select
                    v-model="selectedModel"
                    :items="swaptionModels"
                    density="compact"
                    variant="outlined"
                    hide-details
                  />
                </div>

                <!-- Model Parameters -->
                <div class="section-header">Initial {{ selectedModel }} Parameters</div>

                <!-- SABR: with fix checkboxes (used for calibration) -->
                <template v-if="isSabrModel">
                  <template v-for="param in (['alpha', 'beta', 'rho', 'nu'] as SabrParam[])" :key="param">
                    <div class="grid-label">{{ param === 'alpha' ? 'α' : param === 'beta' ? 'β' : param === 'rho' ? 'ρ' : 'ν' }}</div>
                    <div class="grid-input d-flex align-center" style="gap: 6px">
                      <input
                        :id="'sabr-' + param"
                        v-model.number="sabrInitial[param]"
                        type="number"
                        step="0.01"
                        class="param-input"
                        :class="{ 'opacity-70': sabrFixed[param] }"
                      />
                      <label class="fix-label" :title="'Fix ' + param + ' during calibration'">
                        <input v-model="sabrFixed[param]" type="checkbox" class="fix-checkbox" />
                        <span>fix</span>
                      </label>
                    </div>
                  </template>
                  <div v-if="sabrFixed.beta && sabrInitial.beta === 0" class="grid-span text-[10px] text-[var(--accent)] italic">
                    β=0 fixed → Normal SABR (Bachelier)
                  </div>
                  <div v-else-if="sabrFixed.beta && sabrInitial.beta === 1" class="grid-span text-[10px] text-[var(--accent)] italic">
                    β=1 fixed → Lognormal SABR (Black-Scholes)
                  </div>
                </template>

                <!-- Non-SABR: generic param inputs -->
                <template v-else>
                  <template v-for="def in activeModelParamDefs" :key="def.key">
                    <div class="grid-label font-mono">{{ def.symbol }}</div>
                    <div class="grid-input">
                      <input
                        :id="'mp-' + def.key"
                        v-model.number="modelParams[def.key]"
                        type="number"
                        :step="def.step"
                        class="param-input"
                        :title="def.key"
                      />
                    </div>
                  </template>
                  <div class="grid-span text-[10px] text-[var(--text-muted)] italic">
                    Smile exploration — calibration available for SABR
                  </div>
                </template>
              </div>
            </div>
          </template>

          <!-- FX Settings -->
          <template v-else>
            <div class="glass-card p-5">
              <div class="section-header" style="margin-top: 0">Currency Pair</div>
              <div class="config-grid">
                <div class="grid-label">Pair</div>
                <div class="grid-input">
                  <v-select
                    v-model="selectedFxPair"
                    :items="fxPairs.map(p => ({ title: p, value: p }))"
                    placeholder="Select pair..."
                    density="compact"
                    variant="outlined"
                    hide-details
                  />
                </div>
              </div>
            </div>

            <div class="glass-card p-5">
              <div class="section-header" style="margin-top: 0">Calibration Settings</div>
              <div class="config-grid">
                <div class="grid-label">Model</div>
                <div class="grid-input">
                  <v-select
                    v-model="selectedModel"
                    :items="swaptionModels"
                    density="compact"
                    variant="outlined"
                    hide-details
                  />
                </div>

                <!-- Model Parameters -->
                <div class="section-header">Initial {{ selectedModel }} Parameters</div>

                <template v-if="isSabrModel">
                  <template v-for="param in (['alpha', 'beta', 'rho', 'nu'] as SabrParam[])" :key="param">
                    <div class="grid-label">{{ param === 'alpha' ? 'α' : param === 'beta' ? 'β' : param === 'rho' ? 'ρ' : 'ν' }}</div>
                    <div class="grid-input d-flex align-center" style="gap: 6px">
                      <input
                        :id="'fx-sabr-' + param"
                        v-model.number="sabrInitial[param]"
                        type="number"
                        step="0.01"
                        class="param-input"
                        :class="{ 'opacity-70': sabrFixed[param] }"
                      />
                      <label class="fix-label" :title="'Fix ' + param + ' during calibration'">
                        <input v-model="sabrFixed[param]" type="checkbox" class="fix-checkbox" />
                        <span>fix</span>
                      </label>
                    </div>
                  </template>
                  <div v-if="sabrFixed.beta && sabrInitial.beta === 0" class="grid-span text-[10px] text-[var(--accent)] italic">
                    β=0 fixed → Normal SABR (Bachelier)
                  </div>
                  <div v-else-if="sabrFixed.beta && sabrInitial.beta === 1" class="grid-span text-[10px] text-[var(--accent)] italic">
                    β=1 fixed → Lognormal SABR (Black-Scholes)
                  </div>
                </template>

                <template v-else>
                  <template v-for="def in activeModelParamDefs" :key="def.key">
                    <div class="grid-label font-mono">{{ def.symbol }}</div>
                    <div class="grid-input">
                      <input
                        :id="'fx-mp-' + def.key"
                        v-model.number="modelParams[def.key]"
                        type="number"
                        :step="def.step"
                        class="param-input"
                        :title="def.key"
                      />
                    </div>
                  </template>
                  <div class="grid-span text-[10px] text-[var(--text-muted)] italic">
                    Smile exploration — calibration available for SABR
                  </div>
                </template>
              </div>
            </div>
          </template>

          <!-- Actions -->
          <div class="glass-card p-5">
            <button
              :disabled="(activeTab === 'rates' && !selectedSwaptionIndex) || (activeTab === 'fx' && !selectedFxPair) || isCalibrating"
              class="w-full px-4 py-2.5 rounded-lg bg-[var(--primary)] text-white font-medium hover:opacity-90 disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
              @click="calibrate"
            >
              <i :class="['fas', isCalibrating ? 'fa-spinner fa-spin' : 'fa-cogs']"></i>
              {{ isCalibrating ? 'Calibrating...' : 'Calibrate' }}
            </button>
            <button
              :disabled="!calibrationResult || volPublishFeedback"
              class="w-full mt-2 px-4 py-2 rounded-lg bg-emerald-600 text-white text-sm font-medium hover:opacity-90 disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
              @click="publishVolToEnvironment"
            >
              <i :class="['fas', volPublishFeedback ? 'fa-check' : 'fa-cloud-upload-alt']"></i>
              {{ volPublishFeedback ? 'Published!' : 'Publish to Environment' }}
            </button>
          </div>

        </div>

        <!-- Right Panel: Data Table -->
        <div class="lg:col-span-2">
          <div class="glass-card p-6">
            <div class="flex items-center justify-between mb-4">
              <h3 class="text-lg font-semibold text-[var(--text-primary)]">
                {{ activeTab === 'rates' ? 'Rates Instruments' : 'FX Quotes' }}
              </h3>
              <div v-if="activeTab === 'rates' && swaptionInstruments.length > 0" class="flex gap-1 bg-[var(--surface)] rounded-lg p-0.5">
                <button
                  :class="[
                    'px-3 py-1 text-xs font-medium rounded-md transition-all duration-150',
                    matrixTab === 'vol'
                      ? 'bg-[var(--primary)] text-white shadow-sm'
                      : 'text-[var(--text-secondary)] hover:text-[var(--text-primary)]'
                  ]"
                  @click="matrixTab = 'vol'"
                >Swaption</button>
                <button
                  :class="[
                    'px-3 py-1 text-xs font-medium rounded-md transition-all duration-150',
                    matrixTab === 'capfloor'
                      ? 'bg-[var(--primary)] text-white shadow-sm'
                      : 'text-[var(--text-secondary)] hover:text-[var(--text-primary)]',
                    capFloorQuotes.length === 0 ? 'opacity-50 cursor-not-allowed' : ''
                  ]"
                  :disabled="capFloorQuotes.length === 0"
                  :title="capFloorQuotes.length === 0 ? 'No cap/floor data available' : ''"
                  @click="matrixTab = 'capfloor'"
                >Cap/Floor</button>
                <button
                  :class="[
                    'px-3 py-1 text-xs font-medium rounded-md transition-all duration-150',
                    matrixTab === 'fwd'
                      ? 'bg-[var(--primary)] text-white shadow-sm'
                      : 'text-[var(--text-secondary)] hover:text-[var(--text-primary)]',
                    fwdSwapRates.size === 0 && !isBuildingCurve ? 'opacity-50 cursor-not-allowed' : ''
                  ]"
                  :disabled="fwdSwapRates.size === 0 && !isBuildingCurve"
                  :title="fwdSwapRates.size === 0 ? 'Build curve to view forward rates' : ''"
                  @click="matrixTab = 'fwd'"
                >
                  Fwd
                  <i v-if="isBuildingCurve" class="fas fa-spinner fa-spin ml-1"></i>
                </button>
              </div>
            </div>

            <!-- Swaption Matrix -->
            <template v-if="activeTab === 'rates'">
              <!-- Empty State -->
              <div v-if="swaptionInstruments.length === 0" class="text-center py-12">
                <i class="fas fa-cube text-4xl text-[var(--text-muted)] mb-4"></i>
                <p class="text-[var(--text-muted)]">Select an index to load instruments</p>
              </div>

              <!-- Vol Matrix / Heatmap -->
              <div v-else-if="matrixTab === 'vol'" class="matrix-container relative overflow-x-auto">
                <table class="w-full border-collapse" aria-label="Swaption volatility matrix" role="grid">
                  <thead>
                    <tr>
                      <th class="sticky left-0 z-10 py-2 px-3 text-xs font-medium text-[var(--text-muted)] bg-[var(--glass-bg)] border-b border-r border-[var(--glass-border)]">
                        Expiry \ Tenor
                      </th>
                      <th
                        v-for="tenor in matrixTenors"
                        :key="tenor"
                        class="py-2 px-3 text-xs font-medium text-[var(--text-muted)] text-center border-b border-[var(--glass-border)] min-w-[80px]"
                      >
                        {{ tenor }}
                      </th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="expiry in matrixExpiries" :key="expiry">
                      <td class="sticky left-0 z-10 py-2 px-3 text-xs font-medium text-[var(--text-muted)] bg-[var(--glass-bg)] border-r border-b border-[var(--glass-border)]">
                        {{ expiry }}
                      </td>
                      <!-- F-3: cache getCell result to avoid multiple lookups -->
                      <td
                        v-for="tenor in matrixTenors"
                        :key="tenor"
                        class="py-2 px-2 text-center border-b border-[var(--glass-border)] transition-all duration-150 popover-trigger"
                        :class="[
                          instrumentMap.get(`${expiry}|${tenor}`) ? 'cursor-pointer hover-cell' : 'cursor-default',
                          selectedCell?.expiry === expiry && selectedCell?.tenor === tenor ? 'ring-2 ring-[var(--primary)] ring-inset' : ''
                        ]"
                        :style="instrumentMap.get(`${expiry}|${tenor}`)
                          ? { backgroundColor: rangedHeatmapBg(instrumentMap.get(`${expiry}|${tenor}`)!.atmVol, volRange) }
                          : {}"
                        role="gridcell"
                        :tabindex="instrumentMap.get(`${expiry}|${tenor}`) ? 0 : -1"
                        @click="instrumentMap.get(`${expiry}|${tenor}`) ? togglePopover($event, expiry, tenor) : undefined"
                        @keydown.enter="instrumentMap.get(`${expiry}|${tenor}`) ? togglePopover($event, expiry, tenor) : undefined"
                        @keydown.space.prevent="instrumentMap.get(`${expiry}|${tenor}`) ? togglePopover($event, expiry, tenor) : undefined"
                      >
                        <template v-if="instrumentMap.get(`${expiry}|${tenor}`)">
                          <span
                            class="text-xs font-mono font-medium"
                            :style="{ color: rangedHeatmapText(instrumentMap.get(`${expiry}|${tenor}`)!.atmVol, volRange) }"
                          >
                            {{ formatVol(instrumentMap.get(`${expiry}|${tenor}`)!.atmVol) }}
                          </span>
                        </template>
                        <span v-else class="text-xs text-[var(--text-muted)]">--</span>
                      </td>
                    </tr>
                  </tbody>
                </table>

                <!-- Smile Popover -->
                <div
                  v-if="popoverInstrument"
                  class="smile-popover absolute z-50 w-64 glass-card p-4 shadow-lg"
                  :style="{
                    top: `${popoverPosition.top}px`,
                    left: `${popoverPosition.left}px`,
                    transform: 'translateX(-50%)',
                  }"
                >
                  <div class="flex items-center justify-between mb-3">
                    <h4 class="text-sm font-semibold text-[var(--text-primary)]">
                      {{ popoverInstrument.expiry }} x {{ popoverInstrument.tenor }}
                    </h4>
                    <button
                      class="text-[var(--text-muted)] hover:text-[var(--text-primary)] text-xs"
                      aria-label="Close popover"
                      @click="closePopover"
                    >
                      <i class="fas fa-times"></i>
                    </button>
                  </div>

                  <div class="text-xs space-y-1 mb-3">
                    <div class="flex justify-between">
                      <span class="text-[var(--text-muted)]">ATM Vol:</span>
                      <span class="text-[var(--text-primary)] font-mono">{{ formatVol(popoverInstrument.atmVol) }}</span>
                    </div>
                  </div>

                  <h5 class="text-xs font-medium text-[var(--text-muted)] mb-2">Smile</h5>
                  <table class="w-full text-xs">
                    <thead>
                      <tr class="border-b border-[var(--glass-border)]">
                        <th class="text-left py-1 text-[var(--text-muted)]">Offset (bp)</th>
                        <th class="text-right py-1 text-[var(--text-muted)]">Vol</th>
                      </tr>
                    </thead>
                    <tbody>
                      <tr
                        v-for="pt in popoverInstrument.smile"
                        :key="pt.strikeOffsetBp"
                        class="border-b border-[var(--glass-border)]"
                      >
                        <td class="py-1 text-[var(--text-secondary)]">
                          {{ pt.strikeOffsetBp > 0 ? '+' : '' }}{{ pt.strikeOffsetBp }}
                        </td>
                        <td class="py-1 text-right font-mono text-[var(--text-primary)]">
                          {{ formatVol(pt.vol) }}
                        </td>
                      </tr>
                    </tbody>
                  </table>
                </div>
              </div>

              <!-- Cap/Floor Strip -->
              <div v-else-if="matrixTab === 'capfloor'" class="overflow-x-auto">
                <div v-if="capFloorQuotes.length === 0" class="text-center py-12">
                  <i class="fas fa-layer-group text-4xl text-[var(--text-muted)] mb-4"></i>
                  <p class="text-[var(--text-muted)]">No cap/floor data available for this index</p>
                </div>
                <template v-else>
                  <!-- Calibration method selector + button -->
                  <div class="flex items-center gap-3 mb-4">
                    <div class="flex gap-1 bg-[var(--surface)] rounded-lg p-0.5">
                      <button
                        :class="[
                          'px-3 py-1 text-xs font-medium rounded-md transition-all duration-150',
                          capFloorCalibMethod === 'bootstrap'
                            ? 'bg-[var(--primary)] text-white shadow-sm'
                            : 'text-[var(--text-secondary)] hover:text-[var(--text-primary)]'
                        ]"
                        @click="capFloorCalibMethod = 'bootstrap'"
                      >Bootstrap</button>
                      <button
                        :class="[
                          'px-3 py-1 text-xs font-medium rounded-md transition-all duration-150',
                          capFloorCalibMethod === 'global'
                            ? 'bg-[var(--primary)] text-white shadow-sm'
                            : 'text-[var(--text-secondary)] hover:text-[var(--text-primary)]'
                        ]"
                        @click="capFloorCalibMethod = 'global'"
                      >Global</button>
                    </div>
                    <button
                      :disabled="isCapFloorCalibrating"
                      class="px-4 py-1.5 rounded-lg bg-[var(--primary)] text-white text-xs font-medium hover:opacity-90 disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-1.5"
                      @click="calibrateCapFloorVol"
                    >
                      <i :class="['fas', isCapFloorCalibrating ? 'fa-spinner fa-spin' : 'fa-cogs']"></i>
                      {{ isCapFloorCalibrating ? 'Stripping...' : 'Strip Caplet Vols' }}
                    </button>
                    <span v-if="capFloorCalibResult" class="text-[10px] text-[var(--text-muted)] italic">
                      {{ capFloorCalibResult.method }} — {{ capFloorCalibResult.metadata.processingTimeMs.toFixed(1) }} ms
                    </span>
                  </div>

                  <!-- Cap/Floor table -->
                  <table class="w-full border-collapse" aria-label="Cap/Floor volatility strip">
                    <thead>
                      <tr>
                        <th class="py-2 px-3 text-xs font-medium text-[var(--text-muted)] text-left border-b border-[var(--glass-border)]">Maturity</th>
                        <th class="py-2 px-3 text-xs font-medium text-[var(--text-muted)] text-right border-b border-[var(--glass-border)]">Cap Flat Vol</th>
                        <th class="py-2 px-3 text-xs font-medium text-[var(--text-muted)] text-right border-b border-[var(--glass-border)]">Caplet Vol</th>
                        <th class="py-2 px-3 text-xs font-medium text-[var(--text-muted)] text-right border-b border-[var(--glass-border)]">Strike</th>
                      </tr>
                    </thead>
                    <tbody>
                      <tr
                        v-for="q in capFloorQuotes"
                        :key="q.maturity"
                        class="border-b border-[var(--glass-border)] hover:bg-[var(--surface-hover)] transition-colors"
                      >
                        <td class="py-2 px-3 text-xs font-medium text-[var(--text-primary)]">{{ q.maturity }}</td>
                        <td
                          class="py-2 px-3 text-xs text-right font-mono font-medium"
                          :style="{
                            backgroundColor: rangedHeatmapBg(q.marketVol, capFloorVolRange),
                            color: rangedHeatmapText(q.marketVol, capFloorVolRange),
                          }"
                        >
                          {{ formatVol(q.marketVol) }}
                        </td>
                        <td class="py-2 px-3 text-xs text-right font-mono font-medium">
                          <span v-if="q.capletVol != null" class="text-emerald-400">{{ formatVol(q.capletVol) }}</span>
                          <span v-else class="text-[var(--text-muted)]">--</span>
                        </td>
                        <td class="py-2 px-3 text-xs text-right font-mono text-[var(--text-secondary)]">
                          {{ q.strike != null ? (q.strike * 100).toFixed(2) + '%' : 'ATM' }}
                        </td>
                      </tr>
                    </tbody>
                  </table>
                </template>
              </div>

              <!-- Forward Swap Rate Matrix -->
              <div v-else-if="matrixTab === 'fwd'" class="overflow-x-auto">
                <!-- D-4: Curve error with retry -->
                <div v-if="curveError" class="text-center py-12">
                  <i class="fas fa-exclamation-triangle text-4xl text-red-400 mb-4"></i>
                  <p class="text-[var(--text-muted)] mb-3">{{ curveError }}</p>
                  <button
                    class="px-4 py-2 rounded-lg bg-[var(--surface)] text-[var(--text-primary)] text-sm hover:bg-[var(--surface-hover)]"
                    @click="buildCurveForFwdRates()"
                  >
                    <i class="fas fa-redo mr-1"></i> Retry
                  </button>
                </div>
                <div v-else-if="fwdSwapRates.size === 0" class="text-center py-12">
                  <i v-if="isBuildingCurve" class="fas fa-spinner fa-spin text-4xl text-[var(--primary)] mb-4"></i>
                  <i v-else class="fas fa-chart-line text-4xl text-[var(--text-muted)] mb-4"></i>
                  <p class="text-[var(--text-muted)]">{{ isBuildingCurve ? 'Building curve...' : 'Build curve to view forward swap rates' }}</p>
                </div>
                <table v-else class="w-full border-collapse" aria-label="Forward swap rate matrix">
                  <thead>
                    <tr>
                      <th class="sticky left-0 z-10 py-2 px-3 text-xs font-medium text-[var(--text-muted)] bg-[var(--glass-bg)] border-b border-r border-[var(--glass-border)]">
                        Expiry \ Tenor
                      </th>
                      <th
                        v-for="tenor in matrixTenors"
                        :key="tenor"
                        class="py-2 px-3 text-xs font-medium text-[var(--text-muted)] text-center border-b border-[var(--glass-border)] min-w-[80px]"
                      >
                        {{ tenor }}
                      </th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="expiry in matrixExpiries" :key="expiry">
                      <td class="sticky left-0 z-10 py-2 px-3 text-xs font-medium text-[var(--text-muted)] bg-[var(--glass-bg)] border-r border-b border-[var(--glass-border)]">
                        {{ expiry }}
                      </td>
                      <td
                        v-for="tenor in matrixTenors"
                        :key="tenor"
                        class="py-2 px-2 text-center border-b border-[var(--glass-border)]"
                        :style="fwdSwapRates.get(`${expiry}|${tenor}`) != null
                          ? { backgroundColor: rangedHeatmapBg(fwdSwapRates.get(`${expiry}|${tenor}`)!, fwdRateRange) }
                          : {}"
                      >
                        <span
                          v-if="fwdSwapRates.get(`${expiry}|${tenor}`) != null"
                          class="text-xs font-mono font-medium"
                          :style="{ color: rangedHeatmapText(fwdSwapRates.get(`${expiry}|${tenor}`)!, fwdRateRange) }"
                        >
                          {{ (fwdSwapRates.get(`${expiry}|${tenor}`)! * 100).toFixed(2) }}%
                        </span>
                        <span v-else class="text-xs text-[var(--text-muted)]">--</span>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </template>

            <!-- FX Table -->
            <template v-else>
              <div v-if="fxQuotes.length === 0" class="text-center py-12">
                <i class="fas fa-exchange-alt text-4xl text-[var(--text-muted)] mb-4"></i>
                <p class="text-[var(--text-muted)]">Select a pair to load quotes</p>
              </div>
              <div v-else class="overflow-x-auto">
                <table class="w-full" aria-label="FX volatility quotes">
                  <thead>
                    <tr class="border-b border-[var(--glass-border)]">
                      <th class="text-left py-3 px-3 text-sm font-medium text-[var(--text-muted)]">Tenor</th>
                      <th class="text-right py-3 px-3 text-sm font-medium text-[var(--text-muted)]">Forward</th>
                      <th class="text-right py-3 px-3 text-sm font-medium text-[var(--text-muted)]">ATM Vol</th>
                      <th class="text-right py-3 px-3 text-sm font-medium text-[var(--text-muted)]">25D RR</th>
                      <th class="text-right py-3 px-3 text-sm font-medium text-[var(--text-muted)]">25D BF</th>
                      <th class="text-right py-3 px-3 text-sm font-medium text-[var(--text-muted)]">10D RR</th>
                      <th class="text-right py-3 px-3 text-sm font-medium text-[var(--text-muted)]">10D BF</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr
                      v-for="(quote, idx) in fxQuotes"
                      :key="idx"
                      class="border-b border-[var(--glass-border)] hover:bg-[var(--surface-hover)] transition-colors"
                    >
                      <td class="py-3 px-3 text-sm text-[var(--text-primary)]">{{ quote.expiryLabel || expiryToLabel(quote.expiry) }}</td>
                      <td class="py-3 px-3 text-sm text-right font-mono text-[var(--text-primary)]">{{ quote.forward != null ? quote.forward.toFixed(4) : '--' }}</td>
                      <td class="py-3 px-3 text-sm text-right text-[var(--text-primary)] font-mono">{{ (quote.atmVol * 100).toFixed(2) }}%</td>
                      <td class="py-3 px-3 text-sm text-right font-mono" :class="quote.rr25d < 0 ? 'text-red-400' : 'text-green-400'">{{ (quote.rr25d * 100).toFixed(2) }}%</td>
                      <td class="py-3 px-3 text-sm text-right text-[var(--text-secondary)] font-mono">{{ (quote.bf25d * 100).toFixed(2) }}%</td>
                      <td class="py-3 px-3 text-sm text-right font-mono" :class="(quote.rr10d ?? 0) < 0 ? 'text-red-400' : 'text-green-400'">{{ quote.rr10d != null ? (quote.rr10d * 100).toFixed(2) + '%' : '--' }}</td>
                      <td class="py-3 px-3 text-sm text-right text-[var(--text-secondary)] font-mono">{{ quote.bf10d != null ? (quote.bf10d * 100).toFixed(2) + '%' : '--' }}</td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </template>
          </div>

          <!-- Pre-calibration placeholder for FX tab -->
          <div v-if="!calibrationResult && activeTab === 'fx' && fxQuotes.length > 0" class="glass-card p-6 mt-6 text-center py-8">
            <i class="fas fa-chart-line text-3xl text-[var(--text-muted)] mb-3"></i>
            <p class="text-sm text-[var(--text-muted)]">
              Click <strong>Calibrate</strong> to view delta-strike volatilities &amp; smile charts
            </p>
          </div>

          <!-- Calibration Result (only shown after calibration, same column as instruments) -->
          <div v-if="calibrationResult" class="glass-card p-6 mt-6">
            <div class="flex items-center justify-between mb-4">
              <h3 class="text-base font-semibold text-[var(--text-primary)] flex items-center gap-2">
                <i class="fas fa-check-circle text-[var(--success)]"></i>
                Calibration Result
              </h3>
              <div class="flex items-center gap-4">
                <!-- Calibration param tabs -->
                <div v-if="activeTab === 'rates' && calibrationResult.cellParameters && Object.keys(calibrationResult.cellParameters).length > 0" class="flex gap-1 bg-[var(--surface)] rounded-lg p-0.5">
                  <button
                    v-for="p in (['alpha', 'beta', 'rho', 'nu'] as SabrParam[])"
                    :key="p"
                    :class="[
                      'px-3 py-1 text-xs font-medium rounded-md transition-all duration-150',
                      paramTab === p
                        ? 'bg-[var(--primary)] text-white shadow-sm'
                        : 'text-[var(--text-secondary)] hover:text-[var(--text-primary)]'
                    ]"
                    @click="paramTab = p"
                  >{{ p === 'alpha' ? '\u03B1' : p === 'beta' ? '\u03B2' : p === 'rho' ? '\u03C1' : '\u03BD' }}</button>
                </div>
                <!-- Export buttons -->
                <div class="flex items-center gap-2">
                  <button
                    class="px-3 py-1.5 rounded bg-[var(--surface)] text-[var(--text-secondary)] text-sm hover:bg-[var(--surface-hover)]"
                    @click="exportCsv"
                  >
                    <i class="fas fa-file-csv mr-1"></i>CSV
                  </button>
                  <button
                    class="px-3 py-1.5 rounded bg-[var(--surface)] text-[var(--text-secondary)] text-sm hover:bg-[var(--surface-hover)]"
                    @click="exportJson"
                  >
                    <i class="fas fa-file-code mr-1"></i>JSON
                  </button>
                </div>
              </div>
            </div>

            <!-- Calibration summary badges -->
            <div class="flex flex-wrap items-center gap-3 mb-4">
              <span class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium bg-[var(--surface)] text-[var(--text-primary)]">
                <i class="fas fa-cogs text-[var(--primary)]"></i>
                {{ calibrationResult.model }}
              </span>
              <span class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium bg-[var(--surface)] text-[var(--text-secondary)]">
                <i class="fas fa-th"></i>
                {{ calibrationResult.metadata.instrumentCount }} instruments
              </span>
              <span class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium bg-[var(--surface)] text-[var(--text-secondary)]">
                <i class="fas fa-clock"></i>
                {{ calibrationResult.metadata.processingTimeMs.toFixed(2) }} ms
              </span>
              <span
                v-for="(value, key) in calibrationResult.parameters"
                :key="key"
                class="inline-flex items-center gap-1 px-2.5 py-1 rounded-full text-xs font-mono bg-[var(--surface)] text-[var(--text-secondary)]"
              >
                {{ key }}: {{ Number(value).toFixed(4) }}
              </span>
            </div>

            <!-- ═══ Swaption: Parameter Matrix + Cell Detail ═══ -->
            <template v-if="activeTab === 'rates'">
              <!-- Parameter Matrix -->
              <div v-if="calibrationResult.cellParameters && Object.keys(calibrationResult.cellParameters).length > 0" class="overflow-x-auto mb-4">
                <table class="w-full border-collapse" aria-label="Calibration parameter matrix" role="grid">
                  <thead>
                    <tr>
                      <th class="sticky left-0 z-10 py-2 px-3 text-xs font-medium text-[var(--text-muted)] bg-[var(--glass-bg)] border-b border-r border-[var(--glass-border)]">
                        Expiry \ Tenor
                      </th>
                      <th
                        v-for="tenor in matrixTenors"
                        :key="tenor"
                        class="py-2 px-3 text-xs font-medium text-[var(--text-muted)] text-center border-b border-[var(--glass-border)] min-w-[80px]"
                      >
                        {{ tenor }}
                      </th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="expiry in matrixExpiries" :key="expiry">
                      <td class="sticky left-0 z-10 py-2 px-3 text-xs font-medium text-[var(--text-muted)] bg-[var(--glass-bg)] border-r border-b border-[var(--glass-border)]">
                        {{ expiry }}
                      </td>
                      <td
                        v-for="tenor in matrixTenors"
                        :key="tenor"
                        class="py-2 px-2 text-center border-b border-[var(--glass-border)] transition-all duration-150"
                        :class="[
                          calibrationResult.cellParameters[`${expiry}|${tenor}`] ? 'cursor-pointer hover-cell' : '',
                          selectedCell?.expiry === expiry && selectedCell?.tenor === tenor ? 'ring-2 ring-[var(--primary)] ring-inset' : ''
                        ]"
                        :style="calibrationResult.cellParameters[`${expiry}|${tenor}`]
                          ? { backgroundColor: rangedHeatmapBg(calibrationResult.cellParameters[`${expiry}|${tenor}`][paramTab], paramRange) }
                          : {}"
                        role="gridcell"
                        :tabindex="calibrationResult.cellParameters[`${expiry}|${tenor}`] ? 0 : -1"
                        @click="calibrationResult.cellParameters[`${expiry}|${tenor}`] ? selectCalibrationCell(expiry, tenor) : undefined"
                        @keydown.enter="calibrationResult.cellParameters[`${expiry}|${tenor}`] ? selectCalibrationCell(expiry, tenor) : undefined"
                        @keydown.space.prevent="calibrationResult.cellParameters[`${expiry}|${tenor}`] ? selectCalibrationCell(expiry, tenor) : undefined"
                      >
                        <span
                          v-if="calibrationResult.cellParameters[`${expiry}|${tenor}`]"
                          class="text-xs font-mono font-medium"
                          :style="{ color: rangedHeatmapText(calibrationResult.cellParameters[`${expiry}|${tenor}`][paramTab], paramRange) }"
                        >
                          {{ calibrationResult.cellParameters[`${expiry}|${tenor}`][paramTab].toFixed(4) }}
                        </span>
                        <span v-else class="text-xs text-[var(--text-muted)]">--</span>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>

              <!-- Cell detail (only when a cell is selected in the parameter matrix) -->
              <template v-if="selectedCell">
                <div class="border-t border-[var(--glass-border)] pt-4">
                  <div class="flex items-center justify-between mb-3">
                    <h4 class="text-sm font-semibold text-[var(--text-primary)] flex items-center gap-2">
                      <i class="fas fa-chart-area text-[var(--primary)]"></i>
                      Cell Detail
                      <span class="text-xs text-[var(--text-muted)] font-normal ml-2">
                        {{ selectedCell.expiry }} x {{ selectedCell.tenor }}
                        <template v-if="selectedInstrument"> — ATM {{ formatVol(selectedInstrument.atmVol) }}</template>
                      </span>
                    </h4>
                    <button
                      class="text-[var(--text-muted)] hover:text-[var(--text-primary)] text-sm p-1"
                      aria-label="Close detail card"
                      @click="closeDetailCard"
                    >
                      <i class="fas fa-times"></i>
                    </button>
                  </div>

                  <!-- Calibrated parameters -->
                  <div v-if="selectedCellParams" class="flex flex-wrap gap-2 mb-4">
                    <span
                      v-for="(value, key) in selectedCellParams"
                      :key="key"
                      class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-mono bg-[var(--surface)] text-[var(--text-primary)]"
                    >
                      <span class="text-[var(--primary)] font-semibold">{{ key }}</span> {{ Number(value).toFixed(4) }}
                    </span>
                  </div>

                  <!-- Smile & PDF charts -->
                  <div v-if="selectedCellParams || !isSabrModel" class="grid grid-cols-1 md:grid-cols-2 gap-6">
                    <div>
                      <h5 class="text-xs font-medium text-[var(--text-muted)] mb-2">Smile</h5>
                      <div class="chart-wrapper">
                        <canvas ref="smileChartCanvas"></canvas>
                      </div>
                    </div>
                    <div>
                      <h5 class="text-xs font-medium text-[var(--text-muted)] mb-2">Implied Density (PDF)</h5>
                      <div class="chart-wrapper">
                        <canvas ref="pdfChartCanvas"></canvas>
                      </div>
                    </div>
                  </div>

                  <!-- Cell Jacobian ∂σ/∂θ -->
                  <div v-if="selectedCellJacobian" class="mt-4 pt-4 border-t border-[var(--glass-border)]">
                    <h5 class="text-xs font-medium text-[var(--text-muted)] mb-2">
                      <i class="fas fa-th text-[10px] mr-1"></i>
                      Jacobian &part;&sigma; / &part;&theta;
                    </h5>
                    <div class="overflow-x-auto">
                      <table class="w-full border-collapse">
                        <thead>
                          <tr>
                            <th class="py-1.5 px-2 text-left text-xs font-medium text-[var(--text-muted)] border-b border-[var(--glass-border)]">Strike</th>
                            <th
                              v-for="col in selectedCellJacobian.colLabels"
                              :key="col"
                              class="py-1.5 px-3 text-center text-xs font-medium text-[var(--text-muted)] border-b border-[var(--glass-border)]"
                            >{{ col }}</th>
                          </tr>
                        </thead>
                        <tbody>
                          <tr
                            v-for="(row, i) in selectedCellJacobian.matrix"
                            :key="i"
                            class="hover:bg-[var(--surface-hover)] transition-colors"
                          >
                            <td class="py-1 px-2 text-xs font-medium text-[var(--text-muted)] border-b border-[var(--glass-border)]">
                              {{ selectedCellJacobian.rowLabels[i] }}
                            </td>
                            <td
                              v-for="(val, j) in row"
                              :key="j"
                              class="py-1 px-3 text-center text-xs font-mono border-b border-[var(--glass-border)]"
                              :style="{ backgroundColor: cellJacBg(val, cellJacAbsMax(selectedCellJacobian)), color: cellJacText(val, cellJacAbsMax(selectedCellJacobian)) }"
                            >
                              {{ val === 0 ? '--' : val.toPrecision(3) }}
                            </td>
                          </tr>
                        </tbody>
                      </table>
                    </div>
                    <p class="mt-2 text-[10px] text-[var(--text-muted)]">
                      <i class="fas fa-info-circle mr-1"></i>
                      Sensitivity of model vol (% units) to each calibrated parameter.
                    </p>
                  </div>
                </div>
              </template>
              <template v-else>
                <div class="border-t border-[var(--glass-border)] pt-4 text-center py-6">
                  <p class="text-sm text-[var(--text-muted)]">
                    <i class="fas fa-mouse-pointer mr-1"></i>
                    Select a cell in the parameter matrix to view calibrated parameters &amp; charts
                  </p>
                </div>
              </template>
            </template>

            <!-- ═══ FX: Delta-Vol Table + Row Detail ═══ -->
            <template v-else>
              <!-- Delta-Vol Table -->
              <div class="overflow-x-auto mb-4">
                <table class="w-full" aria-label="FX calibrated delta volatilities">
                  <thead>
                    <tr class="border-b border-[var(--glass-border)]">
                      <th class="text-left py-2.5 px-3 text-xs font-medium text-[var(--text-muted)]">Tenor</th>
                      <th class="text-right py-2.5 px-3 text-xs font-medium text-[var(--text-muted)]">Forward</th>
                      <th class="text-right py-2.5 px-3 text-xs font-medium text-[var(--text-muted)]">10D Put</th>
                      <th class="text-right py-2.5 px-3 text-xs font-medium text-[var(--text-muted)]">25D Put</th>
                      <th class="text-right py-2.5 px-3 text-xs font-medium text-[var(--text-muted)]">ATM</th>
                      <th class="text-right py-2.5 px-3 text-xs font-medium text-[var(--text-muted)]">25D Call</th>
                      <th class="text-right py-2.5 px-3 text-xs font-medium text-[var(--text-muted)]">10D Call</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr
                      v-for="row in fxDeltaVols"
                      :key="row.tenor"
                      class="border-b border-[var(--glass-border)] transition-colors cursor-pointer"
                      :class="selectedFxTenor === row.tenor ? 'fx-row-selected ring-1 ring-[var(--primary)] ring-inset' : 'hover:bg-[var(--surface-hover)]'"
                      @click="selectedFxTenor = row.tenor"
                    >
                      <td class="py-2.5 px-3 text-sm font-medium text-[var(--text-primary)]">{{ row.tenor }}</td>
                      <td class="py-2.5 px-3 text-sm text-right font-mono text-[var(--text-primary)]">{{ row.forward != null ? row.forward.toFixed(4) : '--' }}</td>
                      <td class="py-2.5 px-3 text-sm text-right font-mono text-[var(--text-secondary)]">{{ row.put10 != null ? (row.put10 * 100).toFixed(2) + '%' : '--' }}</td>
                      <td class="py-2.5 px-3 text-sm text-right font-mono text-[var(--text-primary)]">{{ (row.put25 * 100).toFixed(2) }}%</td>
                      <td class="py-2.5 px-3 text-sm text-right font-mono font-semibold text-[var(--text-primary)]">{{ (row.atm * 100).toFixed(2) }}%</td>
                      <td class="py-2.5 px-3 text-sm text-right font-mono text-[var(--text-primary)]">{{ (row.call25 * 100).toFixed(2) }}%</td>
                      <td class="py-2.5 px-3 text-sm text-right font-mono text-[var(--text-secondary)]">{{ row.call10 != null ? (row.call10 * 100).toFixed(2) + '%' : '--' }}</td>
                    </tr>
                  </tbody>
                </table>
              </div>

              <!-- FX Row Detail -->
              <template v-if="selectedFxTenor && (selectedFxParams || !isSabrModel)">
                <div class="border-t border-[var(--glass-border)] pt-4">
                  <div class="flex items-center justify-between mb-3">
                    <h4 class="text-sm font-semibold text-[var(--text-primary)] flex items-center gap-2">
                      <i class="fas fa-chart-area text-[var(--primary)]"></i>
                      Tenor Detail
                      <span class="text-xs text-[var(--text-muted)] font-normal ml-2">
                        {{ selectedFxTenor }}
                        <template v-if="selectedFxQuote"> — ATM {{ (selectedFxQuote.atmVol * 100).toFixed(2) }}%</template>
                      </span>
                    </h4>
                    <button
                      class="text-[var(--text-muted)] hover:text-[var(--text-primary)] text-sm p-1"
                      aria-label="Close detail"
                      @click="selectedFxTenor = null"
                    >
                      <i class="fas fa-times"></i>
                    </button>
                  </div>

                  <!-- Calibrated parameters -->
                  <div v-if="selectedFxParams" class="flex flex-wrap gap-2 mb-4">
                    <span
                      v-for="(value, key) in selectedFxParams"
                      :key="key"
                      class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-mono bg-[var(--surface)] text-[var(--text-primary)]"
                    >
                      <span class="text-[var(--primary)] font-semibold">{{ key }}</span> {{ Number(value).toFixed(4) }}
                    </span>
                  </div>

                  <!-- Smile & PDF charts -->
                  <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                    <div>
                      <h5 class="text-xs font-medium text-[var(--text-muted)] mb-2">Smile</h5>
                      <div class="chart-wrapper">
                        <canvas ref="fxSmileChartCanvas"></canvas>
                      </div>
                    </div>
                    <div>
                      <h5 class="text-xs font-medium text-[var(--text-muted)] mb-2">Implied Density (PDF)</h5>
                      <div class="chart-wrapper">
                        <canvas ref="fxPdfChartCanvas"></canvas>
                      </div>
                    </div>
                  </div>

                  <!-- FX Cell Jacobian ∂σ/∂θ -->
                  <div v-if="selectedFxJacobian" class="mt-4 pt-4 border-t border-[var(--glass-border)]">
                    <h5 class="text-xs font-medium text-[var(--text-muted)] mb-2">
                      <i class="fas fa-th text-[10px] mr-1"></i>
                      Jacobian &part;&sigma; / &part;&theta;
                    </h5>
                    <div class="overflow-x-auto">
                      <table class="w-full border-collapse">
                        <thead>
                          <tr>
                            <th class="py-1.5 px-2 text-left text-xs font-medium text-[var(--text-muted)] border-b border-[var(--glass-border)]">Strike</th>
                            <th
                              v-for="col in selectedFxJacobian.colLabels"
                              :key="col"
                              class="py-1.5 px-3 text-center text-xs font-medium text-[var(--text-muted)] border-b border-[var(--glass-border)]"
                            >{{ col }}</th>
                          </tr>
                        </thead>
                        <tbody>
                          <tr
                            v-for="(row, i) in selectedFxJacobian.matrix"
                            :key="i"
                            class="hover:bg-[var(--surface-hover)] transition-colors"
                          >
                            <td class="py-1 px-2 text-xs font-medium text-[var(--text-muted)] border-b border-[var(--glass-border)]">
                              {{ selectedFxJacobian.rowLabels[i] }}
                            </td>
                            <td
                              v-for="(val, j) in row"
                              :key="j"
                              class="py-1 px-3 text-center text-xs font-mono border-b border-[var(--glass-border)]"
                              :style="{ backgroundColor: cellJacBg(val, cellJacAbsMax(selectedFxJacobian)), color: cellJacText(val, cellJacAbsMax(selectedFxJacobian)) }"
                            >
                              {{ val === 0 ? '--' : val.toPrecision(3) }}
                            </td>
                          </tr>
                        </tbody>
                      </table>
                    </div>
                    <p class="mt-2 text-[10px] text-[var(--text-muted)]">
                      <i class="fas fa-info-circle mr-1"></i>
                      Sensitivity of model vol (% units) to each calibrated parameter.
                    </p>
                  </div>
                </div>
              </template>
              <template v-else>
                <div class="border-t border-[var(--glass-border)] pt-4 text-center py-6">
                  <p class="text-sm text-[var(--text-muted)]">
                    <i class="fas fa-mouse-pointer mr-1"></i>
                    Select a row to view calibrated parameters &amp; smile chart
                  </p>
                </div>
              </template>
            </template>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
/* ─── Model parameter inputs ─── */
.param-input {
  flex: 1;
  min-width: 0;
  padding: 4px 8px;
  font-size: 0.8rem;
  font-family: monospace;
  text-align: right;
  border-radius: 4px;
  background: var(--surface);
  border: 1px solid var(--glass-border);
  color: var(--text-primary);
  outline: none;
  transition: border-color 0.15s;
}
.param-input:focus {
  border-color: var(--primary);
}

.fix-label {
  display: flex;
  align-items: center;
  gap: 3px;
  cursor: pointer;
  user-select: none;
  font-size: 0.65rem;
  color: var(--text-muted);
  flex-shrink: 0;
}

.fix-checkbox {
  width: 14px;
  height: 14px;
  border-radius: 3px;
  cursor: pointer;
}

.matrix-container {
  position: relative;
}

.hover-cell:hover {
  filter: brightness(1.3);
}

/* G-3: arrow colour uses glass-bg for theme compatibility */
.smile-popover::before {
  content: '';
  position: absolute;
  top: -6px;
  left: 50%;
  transform: translateX(-50%);
  border-left: 6px solid transparent;
  border-right: 6px solid transparent;
  border-bottom: 6px solid var(--glass-bg);
}

/* G-2: responsive chart height */
.chart-wrapper {
  height: clamp(160px, 20vw, 280px);
  position: relative;
}

/* FX row selected highlight — Tailwind opacity modifier doesn't work with CSS vars */
.fx-row-selected {
  background-color: color-mix(in srgb, var(--primary) 10%, transparent);
}
</style>
