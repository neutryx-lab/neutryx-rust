<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue';
import { Chart, type ChartDataset, registerables } from 'chart.js';
import type {
  SwaptionInstrument,
  FxVolQuote,
  VolcubeCalibrateResponse,
} from '@/types';
import {
  fetchVolcubeIndices,
  fetchVolcubeModels,
  fetchVolcubeInstruments,
  fetchFxVolPairs,
  fetchFxVolQuotes,
  calibrateVolcube,
  calibrateFxVol,
  computeSabrSmile,
} from '@/services/api';

Chart.register(...registerables);

// ── Constants (F-2) ─────────────────────────────────────────────────────────
const EXPIRY_ORDER = ['1M', '3M', '6M', '1Y', '2Y', '5Y', '10Y', '15Y', '20Y', '30Y'];
const TENOR_ORDER = ['1Y', '2Y', '5Y', '10Y', '15Y', '20Y', '30Y'];
const UNKNOWN_SORT_ORDER = 999;
const DEFAULT_FORWARD_RATE = 0.03;
const SABR_SMILE_N_POINTS = 101;
const SABR_SMILE_RANGE_BP = 200;
const POPOVER_WIDTH = 256; // matches w-64
const ERROR_AUTO_DISMISS_MS = 8000;

type AssetTab = 'swaption' | 'fx';
type SabrParam = 'alpha' | 'beta' | 'rho' | 'nu';

// ── State ────────────────────────────────────────────────────────────────────
const activeTab = ref<AssetTab>('swaption');
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
const matrixTab = ref<'vol' | 'fwd'>('vol');
const paramTab = ref<SabrParam>('alpha');

const fxPairs = ref<string[]>([]);
const selectedFxPair = ref('');
const fxQuotes = ref<FxVolQuote[]>([]);
const fxSpot = ref('');
const fxDomesticRate = ref('0');
const fxForeignRate = ref('0');

// SABR parameter settings (initial values + fixed flags)
const sabrInitial = ref<Record<SabrParam, number>>({ alpha: 0.03, beta: 0, rho: -0.3, nu: 0.4 });
const sabrFixed = ref<Record<SabrParam, boolean>>({ alpha: false, beta: true, rho: false, nu: false });

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

// Detail card chart state
const smileChartCanvas = ref<HTMLCanvasElement | null>(null);
const pdfChartCanvas = ref<HTMLCanvasElement | null>(null);
let smileChartInstance: Chart | null = null;
let pdfChartInstance: Chart | null = null;

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
  if (!cellParams) return; // No calibrated params — nothing to render

  const inst = selectedInstrument.value;
  const cell = selectedCell.value;

  const axisStyle = {
    ticks: { color: 'rgba(255,255,255,0.6)', font: { size: 10 } },
    grid: { color: 'rgba(255,255,255,0.08)' },
  };

  // Determine forward rate for this cell (C-2: use ?? instead of ||)
  const fwdKey = `${cell.expiry}|${cell.tenor}`;
  const forward = fwdSwapRates.value.get(fwdKey) ?? DEFAULT_FORWARD_RATE;

  try {
    const result = await computeSabrSmile({
      alpha: cellParams.alpha,
      beta: cellParams.beta,
      rho: cellParams.rho,
      nu: cellParams.nu,
      forward,
      expiry_years: expiryToYears(cell.expiry),
      n_points: SABR_SMILE_N_POINTS,
      range_bp: SABR_SMILE_RANGE_BP,
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
          label: 'SABR Fitted',
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
              legend: { display: datasets.length > 1, labels: { color: 'rgba(255,255,255,0.7)', font: { size: 10 } } },
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
                title: { display: true, text: 'Strike Offset (bp)', color: 'rgba(255,255,255,0.5)', font: { size: 10 } },
                ticks: { ...axisStyle.ticks, maxTicksLimit: 10 },
              },
              y: {
                ...axisStyle,
                title: { display: true, text: 'Normal Vol (bp)', color: 'rgba(255,255,255,0.5)', font: { size: 10 } },
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
                title: { display: true, text: 'Strike Offset (bp)', color: 'rgba(255,255,255,0.5)', font: { size: 10 } },
                ticks: { ...axisStyle.ticks, maxTicksLimit: 10 },
              },
              y: {
                ...axisStyle,
                title: { display: true, text: 'Density', color: 'rgba(255,255,255,0.5)', font: { size: 10 } },
              },
            },
          },
        });
      }
    }
  } catch (error) {
    const msg = error instanceof Error ? error.message : 'SABR smile computation failed';
    showError(msg);
    console.error('Failed to compute SABR smile:', error);
  }
}

// ── Summary stats ────────────────────────────────────────────────────────────
const summaryStats = computed(() => {
  if (activeTab.value === 'swaption') {
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
    popoverCell.value = null;
    selectedCell.value = null;
    // C-1: await the curve build
    await buildCurveForFwdRates();
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

async function calibrate() {
  // C-5: prevent multiple simultaneous calibrations
  if (isCalibrating.value) return;
  if (activeTab.value === 'swaption' && !selectedSwaptionIndex.value) return;
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
    if (activeTab.value === 'swaption') {
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
watch(activeTab, () => {
  calibrationResult.value = null;
  selectedCell.value = null;
  popoverCell.value = null;
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

// ── Lifecycle ────────────────────────────────────────────────────────────────
onMounted(() => {
  document.addEventListener('click', onDocumentClick);
});

onUnmounted(() => {
  document.removeEventListener('click', onDocumentClick);
  destroyCharts();
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
      <div class="flex gap-2 mb-6">
        <button
          :class="[
            'px-4 py-2 rounded-lg font-medium transition-all duration-200 flex items-center gap-2',
            activeTab === 'swaption'
              ? 'bg-[var(--primary)] text-white'
              : 'bg-[var(--surface)] text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]'
          ]"
          @click="activeTab = 'swaption'"
        >
          <i class="fas fa-percentage"></i>
          Swaption
        </button>
        <button
          :class="[
            'px-4 py-2 rounded-lg font-medium transition-all duration-200 flex items-center gap-2',
            activeTab === 'fx'
              ? 'bg-[var(--primary)] text-white'
              : 'bg-[var(--surface)] text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]'
          ]"
          @click="activeTab = 'fx'"
        >
          <i class="fas fa-exchange-alt"></i>
          FX
        </button>
      </div>

      <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <!-- Left Panel: Settings -->
        <div class="space-y-4">
          <!-- Swaption Settings -->
          <template v-if="activeTab === 'swaption'">
            <div class="glass-card p-5">
              <h3 class="text-base font-semibold text-[var(--text-primary)] mb-3">Index Selection</h3>
              <select
                v-model="selectedSwaptionIndex"
                class="w-full px-3 py-2 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-[var(--primary)]"
              >
                <option value="">Select index...</option>
                <option v-for="idx in swaptionIndices" :key="idx" :value="idx">{{ idx }}</option>
              </select>
            </div>

            <div class="glass-card p-5">
              <h3 class="text-base font-semibold text-[var(--text-primary)] mb-3">Calibration Settings</h3>
              <div class="space-y-3">
                <div>
                  <label class="block text-xs text-[var(--text-muted)] mb-1">Model</label>
                  <select
                    v-model="selectedModel"
                    class="w-full px-2 py-1.5 rounded bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm"
                  >
                    <option v-for="model in swaptionModels" :key="model" :value="model">{{ model }}</option>
                  </select>
                </div>

                <!-- SABR Parameter Initial Values + Fix Checkboxes -->
                <div class="border-t border-[var(--glass-border)] pt-3 mt-2">
                  <label class="block text-xs text-[var(--text-muted)] mb-2">SABR Parameters</label>
                  <div class="space-y-2">
                    <div v-for="param in (['alpha', 'beta', 'rho', 'nu'] as SabrParam[])" :key="param" class="flex items-center gap-2">
                      <label class="w-10 text-xs font-mono text-[var(--text-secondary)] select-none" :for="'sabr-' + param">{{ param === 'alpha' ? 'α' : param === 'beta' ? 'β' : param === 'rho' ? 'ρ' : 'ν' }}</label>
                      <input
                        :id="'sabr-' + param"
                        v-model.number="sabrInitial[param]"
                        type="number"
                        step="0.01"
                        class="flex-1 px-2 py-1 rounded bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-xs font-mono focus:outline-none focus:ring-1 focus:ring-[var(--primary)] w-0"
                        :class="{ 'opacity-70': sabrFixed[param] }"
                      />
                      <label class="flex items-center gap-1 cursor-pointer select-none" :title="'Fix ' + param + ' during calibration'">
                        <input
                          v-model="sabrFixed[param]"
                          type="checkbox"
                          class="w-3.5 h-3.5 rounded border-[var(--glass-border)] text-[var(--primary)] focus:ring-[var(--primary)] focus:ring-offset-0 cursor-pointer"
                        />
                        <span class="text-[10px] text-[var(--text-muted)]">fix</span>
                      </label>
                    </div>
                  </div>
                  <p v-if="sabrFixed.beta && sabrInitial.beta === 0" class="text-[10px] text-[var(--accent)] mt-1.5 italic">
                    β=0 fixed → Normal SABR (Bachelier)
                  </p>
                </div>
              </div>
            </div>
          </template>

          <!-- FX Settings -->
          <template v-else>
            <div class="glass-card p-5">
              <h3 class="text-base font-semibold text-[var(--text-primary)] mb-3">Currency Pair</h3>
              <select
                v-model="selectedFxPair"
                class="w-full px-3 py-2 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-[var(--primary)]"
              >
                <option value="">Select pair...</option>
                <option v-for="pair in fxPairs" :key="pair" :value="pair">{{ pair }}</option>
              </select>
            </div>

            <div class="glass-card p-5">
              <h3 class="text-base font-semibold text-[var(--text-primary)] mb-3">Calibration Settings</h3>
              <div class="space-y-3">
                <div>
                  <label class="block text-xs text-[var(--text-muted)] mb-1">Model</label>
                  <select
                    v-model="selectedModel"
                    class="w-full px-2 py-1.5 rounded bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm"
                  >
                    <option v-for="model in swaptionModels" :key="model" :value="model">{{ model }}</option>
                  </select>
                </div>

                <!-- SABR Parameter Initial Values + Fix Checkboxes -->
                <div class="border-t border-[var(--glass-border)] pt-3 mt-2">
                  <label class="block text-xs text-[var(--text-muted)] mb-2">SABR Parameters</label>
                  <div class="space-y-2">
                    <div v-for="param in (['alpha', 'beta', 'rho', 'nu'] as SabrParam[])" :key="param" class="flex items-center gap-2">
                      <label class="w-10 text-xs font-mono text-[var(--text-secondary)] select-none" :for="'fx-sabr-' + param">{{ param === 'alpha' ? 'α' : param === 'beta' ? 'β' : param === 'rho' ? 'ρ' : 'ν' }}</label>
                      <input
                        :id="'fx-sabr-' + param"
                        v-model.number="sabrInitial[param]"
                        type="number"
                        step="0.01"
                        class="flex-1 px-2 py-1 rounded bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-xs font-mono focus:outline-none focus:ring-1 focus:ring-[var(--primary)] w-0"
                        :class="{ 'opacity-70': sabrFixed[param] }"
                      />
                      <label class="flex items-center gap-1 cursor-pointer select-none" :title="'Fix ' + param + ' during calibration'">
                        <input
                          v-model="sabrFixed[param]"
                          type="checkbox"
                          class="w-3.5 h-3.5 rounded border-[var(--glass-border)] text-[var(--primary)] focus:ring-[var(--primary)] focus:ring-offset-0 cursor-pointer"
                        />
                        <span class="text-[10px] text-[var(--text-muted)]">fix</span>
                      </label>
                    </div>
                  </div>
                  <p v-if="sabrFixed.beta && sabrInitial.beta === 0" class="text-[10px] text-[var(--accent)] mt-1.5 italic">
                    β=0 fixed → Normal SABR (Bachelier)
                  </p>
                </div>
              </div>
            </div>
          </template>

          <!-- Actions -->
          <div class="glass-card p-5">
            <button
              :disabled="(activeTab === 'swaption' && !selectedSwaptionIndex) || (activeTab === 'fx' && !selectedFxPair) || isCalibrating"
              class="w-full px-4 py-2.5 rounded-lg bg-[var(--primary)] text-white font-medium hover:opacity-90 disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
              @click="calibrate"
            >
              <i :class="['fas', isCalibrating ? 'fa-spinner fa-spin' : 'fa-cogs']"></i>
              {{ isCalibrating ? 'Calibrating...' : 'Calibrate' }}
            </button>
          </div>

        </div>

        <!-- Right Panel: Data Table -->
        <div class="lg:col-span-2">
          <div class="glass-card p-6">
            <div class="flex items-center justify-between mb-4">
              <h3 class="text-lg font-semibold text-[var(--text-primary)]">
                {{ activeTab === 'swaption' ? 'Swaption Instruments' : 'FX Quotes' }}
              </h3>
              <div v-if="activeTab === 'swaption' && swaptionInstruments.length > 0" class="flex gap-1 bg-[var(--surface)] rounded-lg p-0.5">
                <button
                  :class="[
                    'px-3 py-1 text-xs font-medium rounded-md transition-all duration-150',
                    matrixTab === 'vol'
                      ? 'bg-[var(--primary)] text-white shadow-sm'
                      : 'text-[var(--text-secondary)] hover:text-[var(--text-primary)]'
                  ]"
                  @click="matrixTab = 'vol'"
                >Vol</button>
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
            <template v-if="activeTab === 'swaption'">
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

              <!-- Forward Swap Rate Matrix -->
              <div v-else class="overflow-x-auto">
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

          <!-- Calibration Result (only shown after calibration, same column as instruments) -->
          <div v-if="calibrationResult" class="glass-card p-6 mt-6">
            <div class="flex items-center justify-between mb-4">
              <h3 class="text-base font-semibold text-[var(--text-primary)] flex items-center gap-2">
                <i class="fas fa-check-circle text-[var(--success)]"></i>
                Calibration Result
              </h3>
              <div class="flex items-center gap-4">
                <!-- SABR param tabs -->
                <div v-if="calibrationResult.cellParameters && Object.keys(calibrationResult.cellParameters).length > 0" class="flex gap-1 bg-[var(--surface)] rounded-lg p-0.5">
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

            <!-- Parameter Matrix -->
            <div v-if="calibrationResult.cellParameters && Object.keys(calibrationResult.cellParameters).length > 0" class="overflow-x-auto mb-4">
              <table class="w-full border-collapse" aria-label="SABR parameter matrix" role="grid">
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

                <!-- Calibrated SABR parameters -->
                <div v-if="selectedCellParams" class="flex flex-wrap gap-2 mb-4">
                  <span class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-mono bg-[var(--surface)] text-[var(--text-primary)]">
                    <span class="text-[var(--primary)] font-semibold">&alpha;</span> {{ selectedCellParams.alpha.toFixed(4) }}
                  </span>
                  <span class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-mono bg-[var(--surface)] text-[var(--text-primary)]">
                    <span class="text-[var(--primary)] font-semibold">&beta;</span> {{ selectedCellParams.beta.toFixed(4) }}
                  </span>
                  <span class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-mono bg-[var(--surface)] text-[var(--text-primary)]">
                    <span class="text-[var(--primary)] font-semibold">&rho;</span> {{ selectedCellParams.rho.toFixed(4) }}
                  </span>
                  <span class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-mono bg-[var(--surface)] text-[var(--text-primary)]">
                    <span class="text-[var(--primary)] font-semibold">&nu;</span> {{ selectedCellParams.nu.toFixed(4) }}
                  </span>
                </div>

                <!-- Smile & PDF charts (only when SABR params are available) -->
                <div v-if="selectedCellParams" class="grid grid-cols-1 md:grid-cols-2 gap-6">
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
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.glass-card {
  background: var(--glass-bg);
  backdrop-filter: blur(20px);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-lg);
  box-shadow: var(--glass-shadow);
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
</style>
