/**
 * VolCube Builder Module
 * Handles volatility surface calibration for swaptions and FX options
 */

import { createScopedLogger } from '@/utils/logger';
import { getElementById, showToast } from '@/utils/dom';
import { escapeHtml, formatVol } from '@/utils/format';

const log = createScopedLogger('VolCubeBuilder');

// =============================================================================
// Types
// =============================================================================

interface SwaptionInstrument {
  expiry: string;
  tenor: string;
  atmVol: number;
  smile?: Array<{ strikeOffsetBp: number; vol: number }>;
}

interface FxQuote {
  expiry: number;
  atmVol: number;
  rr25d: number;
  bf25d: number;
  rr10d?: number;
  bf10d?: number;
}

interface CalibrationResult {
  surfaceId: string;
  model: string;
  parameters: Record<string, number>;
  errors: Array<{ expiry: string; tenor?: string; error: number }>;
  metadata: {
    instrumentCount: number;
    processingTimeMs: number;
  };
}

interface VolcubeState {
  activeTab: 'swaption' | 'fx';
  swaptionIndices: string[];
  selectedSwaptionIndex: string | null;
  swaptionInstruments: SwaptionInstrument[];
  swaptionModels: string[];
  fxPairs: string[];
  selectedFxPair: string | null;
  fxQuotes: FxQuote[];
  calibrationResult: CalibrationResult | null;
  expiries: number[];
  tenors: number[];
  selectedExpiry: number | null;
  selectedTenor: number | null;
}

// =============================================================================
// State
// =============================================================================

const state: VolcubeState = {
  activeTab: 'swaption',
  swaptionIndices: [],
  selectedSwaptionIndex: null,
  swaptionInstruments: [],
  swaptionModels: [],
  fxPairs: [],
  selectedFxPair: null,
  fxQuotes: [],
  calibrationResult: null,
  expiries: [],
  tenors: [],
  selectedExpiry: null,
  selectedTenor: null,
};

let elements: Record<string, HTMLElement | null> = {};

// =============================================================================
// Element Caching
// =============================================================================

function cacheElements(): void {
  elements = {
    assetTabs: getElementById('volcube-asset-tabs'),
    swaptionPanel: getElementById('volcube-swaption-panel'),
    fxPanel: getElementById('volcube-fx-panel'),
    indexSelector: getElementById('volcube-index-selector'),
    referenceDate: getElementById('volcube-reference-date'),
    calibSettings: getElementById('volcube-calib-settings'),
    instrumentsTable: getElementById('volcube-instruments-table'),
    fxPairSelector: getElementById('fxvol-pair-selector'),
    fxModelSelector: getElementById('fxvol-model-selector'),
    fxSpot: getElementById('fxvol-spot'),
    fxDomesticRate: getElementById('fxvol-domestic-rate'),
    fxForeignRate: getElementById('fxvol-foreign-rate'),
    fxQuotesTable: getElementById('fxvol-quotes-table'),
    expirySelector: getElementById('volcube-expiry-selector'),
    tenorSelector: getElementById('volcube-tenor-selector'),
    calibrateBtn: getElementById('volcube-calibrate-btn'),
    exportCsvBtn: getElementById('volcube-export-csv'),
    exportJsonBtn: getElementById('volcube-export-json'),
    resultsContainer: getElementById('volcube-results-container'),
  };

  // Set default reference date
  if (elements.referenceDate) {
    (elements.referenceDate as HTMLInputElement).value = new Date().toISOString().split('T')[0];
  }
}

// =============================================================================
// Event Listeners
// =============================================================================

function attachEventListeners(): void {
  elements.calibrateBtn?.addEventListener('click', () => void calibrate());

  (elements.indexSelector as HTMLSelectElement | null)?.addEventListener('change', (e) => {
    state.selectedSwaptionIndex = (e.target as HTMLSelectElement).value;
    if (state.selectedSwaptionIndex) {
      void loadSwaptionInstruments(state.selectedSwaptionIndex);
    }
  });

  (elements.fxPairSelector as HTMLSelectElement | null)?.addEventListener('change', (e) => {
    state.selectedFxPair = (e.target as HTMLSelectElement).value;
    if (state.selectedFxPair) {
      void loadFxQuotes(state.selectedFxPair);
    }
  });

  (elements.expirySelector as HTMLSelectElement | null)?.addEventListener('change', (e) => {
    state.selectedExpiry = parseFloat((e.target as HTMLSelectElement).value);
    updateVisualization();
  });

  (elements.tenorSelector as HTMLSelectElement | null)?.addEventListener('change', (e) => {
    state.selectedTenor = parseFloat((e.target as HTMLSelectElement).value);
    updateVisualization();
  });

  elements.exportCsvBtn?.addEventListener('click', exportCsv);
  elements.exportJsonBtn?.addEventListener('click', exportJson);
}

// =============================================================================
// Data Loading
// =============================================================================

async function loadSwaptionIndices(): Promise<void> {
  try {
    const response = await fetch('/api/volcube/indices');
    if (!response.ok) throw new Error('Failed to load indices');
    const data = await response.json();
    state.swaptionIndices = data.indices || [];
    renderIndexSelector();
    renderAssetTabs();
  } catch (error) {
    log.error('Failed to load swaption indices', error);
  }
}

async function loadSwaptionModels(): Promise<void> {
  try {
    const response = await fetch('/api/volcube/models');
    if (!response.ok) throw new Error('Failed to load models');
    const data = await response.json();
    state.swaptionModels = data.models || [];
    renderCalibSettings();
  } catch (error) {
    log.error('Failed to load calibration models', error);
  }
}

async function loadSwaptionInstruments(index: string): Promise<void> {
  try {
    const response = await fetch(`/api/volcube/instruments/${index}`);
    if (!response.ok) throw new Error('Failed to load instruments');
    const data = await response.json();
    state.swaptionInstruments = data.instruments || [];
    extractExpiriesAndTenors();
    renderInstrumentsTable();
  } catch (error) {
    log.error('Failed to load instruments', error);
  }
}

async function loadFxPairs(): Promise<void> {
  try {
    const response = await fetch('/api/fxvol/pairs');
    if (!response.ok) throw new Error('Failed to load FX pairs');
    const data = await response.json();
    state.fxPairs = (data.pairs || []).map((p: { pair: string }) => p.pair);
    renderFxPairSelector();
  } catch (error) {
    log.error('Failed to load FX pairs', error);
  }
}

async function loadFxQuotes(pair: string): Promise<void> {
  try {
    const response = await fetch(`/api/fxvol/quotes/${pair}`);
    if (!response.ok) throw new Error('Failed to load FX quotes');
    const data = await response.json();
    state.fxQuotes = data.quotes || [];
    if (data.spot && elements.fxSpot) {
      (elements.fxSpot as HTMLInputElement).value = data.spot.toFixed(4);
    }
    renderFxQuotesTable();
  } catch (error) {
    log.error('Failed to load FX quotes', error);
  }
}

// =============================================================================
// Rendering
// =============================================================================

function renderAssetTabs(): void {
  if (!elements.assetTabs) return;

  elements.assetTabs.innerHTML = `
    <button class="volcube-tab ${state.activeTab === 'swaption' ? 'active' : ''}" data-tab="swaption">
      <i class="fas fa-percentage"></i> Swaption
    </button>
    <button class="volcube-tab ${state.activeTab === 'fx' ? 'active' : ''}" data-tab="fx">
      <i class="fas fa-exchange-alt"></i> FX
    </button>
  `;

  elements.assetTabs.querySelectorAll('.volcube-tab').forEach((tab) => {
    tab.addEventListener('click', () => {
      state.activeTab = (tab as HTMLElement).dataset.tab as 'swaption' | 'fx';
      renderAssetTabs();
      togglePanels();
    });
  });
}

function togglePanels(): void {
  if (elements.swaptionPanel) {
    elements.swaptionPanel.style.display = state.activeTab === 'swaption' ? 'block' : 'none';
  }
  if (elements.fxPanel) {
    elements.fxPanel.style.display = state.activeTab === 'fx' ? 'block' : 'none';
  }
}

function renderIndexSelector(): void {
  const selector = elements.indexSelector as HTMLSelectElement | null;
  if (!selector) return;

  selector.innerHTML = `
    <option value="">Select index...</option>
    ${state.swaptionIndices.map((idx) => `<option value="${escapeHtml(idx)}">${escapeHtml(idx)}</option>`).join('')}
  `;
}

function renderFxPairSelector(): void {
  const selector = elements.fxPairSelector as HTMLSelectElement | null;
  if (!selector) return;

  selector.innerHTML = `
    <option value="">Select pair...</option>
    ${state.fxPairs.map((pair) => `<option value="${escapeHtml(pair)}">${escapeHtml(pair)}</option>`).join('')}
  `;
}

function renderCalibSettings(): void {
  if (!elements.calibSettings) return;

  elements.calibSettings.innerHTML = `
    <div class="form-group">
      <label>Model</label>
      <select id="calib-model-selector" class="fancy-select">
        ${state.swaptionModels.map((m) => `<option value="${escapeHtml(m)}">${escapeHtml(m)}</option>`).join('')}
      </select>
    </div>
  `;
}

function renderInstrumentsTable(): void {
  if (!elements.instrumentsTable) return;

  if (state.swaptionInstruments.length === 0) {
    elements.instrumentsTable.innerHTML = `
      <div class="placeholder">
        <i class="fas fa-cube"></i>
        <p>Select an index to load instruments</p>
      </div>
    `;
    return;
  }

  const html = `
    <table class="volcube-table">
      <thead>
        <tr>
          <th>Expiry</th>
          <th>Tenor</th>
          <th>ATM Vol</th>
        </tr>
      </thead>
      <tbody>
        ${state.swaptionInstruments
          .map(
            (inst) => `
          <tr>
            <td>${escapeHtml(inst.expiry)}</td>
            <td>${escapeHtml(inst.tenor)}</td>
            <td>${formatVol(inst.atmVol)}</td>
          </tr>
        `
          )
          .join('')}
      </tbody>
    </table>
  `;
  elements.instrumentsTable.innerHTML = html;
}

function renderFxQuotesTable(): void {
  if (!elements.fxQuotesTable) return;

  if (state.fxQuotes.length === 0) {
    elements.fxQuotesTable.innerHTML = `
      <div class="placeholder">
        <i class="fas fa-exchange-alt"></i>
        <p>Select a pair to load quotes</p>
      </div>
    `;
    return;
  }

  const html = `
    <table class="volcube-table">
      <thead>
        <tr>
          <th>Expiry</th>
          <th>ATM Vol</th>
          <th>25D RR</th>
          <th>25D BF</th>
        </tr>
      </thead>
      <tbody>
        ${state.fxQuotes
          .map(
            (q) => `
          <tr>
            <td>${expiryToLabel(q.expiry)}</td>
            <td>${formatVol(q.atmVol)}</td>
            <td>${(q.rr25d * 10000).toFixed(1)} bps</td>
            <td>${(q.bf25d * 10000).toFixed(1)} bps</td>
          </tr>
        `
          )
          .join('')}
      </tbody>
    </table>
  `;
  elements.fxQuotesTable.innerHTML = html;
}

function extractExpiriesAndTenors(): void {
  const expiriesSet = new Set<number>();
  const tenorsSet = new Set<number>();

  state.swaptionInstruments.forEach((inst) => {
    // Parse tenor string to years (simplified)
    const expiryYears = parseTenorToYears(inst.expiry);
    const tenorYears = parseTenorToYears(inst.tenor);
    if (expiryYears) expiriesSet.add(expiryYears);
    if (tenorYears) tenorsSet.add(tenorYears);
  });

  state.expiries = Array.from(expiriesSet).sort((a, b) => a - b);
  state.tenors = Array.from(tenorsSet).sort((a, b) => a - b);
}

function parseTenorToYears(tenor: string): number | null {
  const match = tenor.match(/^(\d+)([DWMY])$/i);
  if (!match) return null;
  const value = parseInt(match[1]);
  const unit = match[2].toUpperCase();
  switch (unit) {
    case 'D':
      return value / 365;
    case 'W':
      return value / 52;
    case 'M':
      return value / 12;
    case 'Y':
      return value;
    default:
      return null;
  }
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

function updateVisualization(): void {
  // Placeholder for visualization update
  log.debug('Visualization update', {
    expiry: state.selectedExpiry,
    tenor: state.selectedTenor,
  });
}

// =============================================================================
// Actions
// =============================================================================

async function calibrate(): Promise<void> {
  if (state.activeTab === 'swaption' && !state.selectedSwaptionIndex) {
    showToast('Please select an index first', 'warning');
    return;
  }
  if (state.activeTab === 'fx' && !state.selectedFxPair) {
    showToast('Please select a pair first', 'warning');
    return;
  }

  if (elements.calibrateBtn) {
    (elements.calibrateBtn as HTMLButtonElement).disabled = true;
    elements.calibrateBtn.innerHTML = '<i class="fas fa-spinner fa-spin"></i> Calibrating...';
  }

  try {
    const endpoint =
      state.activeTab === 'swaption'
        ? '/api/volcube/calibrate'
        : '/api/fxvol/calibrate';

    const body =
      state.activeTab === 'swaption'
        ? {
            index: state.selectedSwaptionIndex,
            referenceDate: (elements.referenceDate as HTMLInputElement | null)?.value,
            model: (getElementById('calib-model-selector') as HTMLSelectElement | null)?.value,
          }
        : {
            pair: state.selectedFxPair,
            spot: parseFloat((elements.fxSpot as HTMLInputElement | null)?.value || '0'),
            domesticRate: parseFloat((elements.fxDomesticRate as HTMLInputElement | null)?.value || '0') / 100,
            foreignRate: parseFloat((elements.fxForeignRate as HTMLInputElement | null)?.value || '0') / 100,
          };

    const response = await fetch(endpoint, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.message || 'Calibration failed');
    }

    state.calibrationResult = await response.json();
    renderCalibrationResult();
    showToast('Calibration completed', 'success');
    log.info('Calibration completed', {
      tab: state.activeTab,
      index: state.selectedSwaptionIndex,
      pair: state.selectedFxPair,
    });
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    log.error('Calibration failed', message);
    showToast(`Calibration failed: ${message}`, 'error');
  } finally {
    if (elements.calibrateBtn) {
      (elements.calibrateBtn as HTMLButtonElement).disabled = false;
      elements.calibrateBtn.innerHTML = '<i class="fas fa-cogs"></i> Calibrate';
    }
  }
}

function renderCalibrationResult(): void {
  if (!state.calibrationResult || !elements.resultsContainer) return;

  const result = state.calibrationResult;
  elements.resultsContainer.innerHTML = `
    <div class="card compact">
      <h4><i class="fas fa-check-circle"></i> Calibration Result</h4>
      <div class="stat-row">
        <span class="stat-label">Model:</span>
        <span class="stat-value">${escapeHtml(result.model)}</span>
      </div>
      <div class="stat-row">
        <span class="stat-label">Instruments:</span>
        <span class="stat-value">${result.metadata.instrumentCount}</span>
      </div>
      <div class="stat-row">
        <span class="stat-label">Processing Time:</span>
        <span class="stat-value">${result.metadata.processingTimeMs.toFixed(2)} ms</span>
      </div>
      <h5>Parameters</h5>
      ${Object.entries(result.parameters)
        .map(
          ([key, value]) => `
        <div class="stat-row">
          <span class="stat-label">${escapeHtml(key)}:</span>
          <span class="stat-value">${(value as number).toFixed(6)}</span>
        </div>
      `
        )
        .join('')}
    </div>
  `;
}

function exportCsv(): void {
  if (!state.calibrationResult) {
    showToast('No calibration result to export', 'warning');
    return;
  }

  const csv = [
    'Parameter,Value',
    ...Object.entries(state.calibrationResult.parameters).map(
      ([key, value]) => `${key},${value}`
    ),
  ].join('\n');

  downloadFile(csv, 'volcube_calibration.csv', 'text/csv');
  showToast('Exported to CSV', 'success');
}

function exportJson(): void {
  if (!state.calibrationResult) {
    showToast('No calibration result to export', 'warning');
    return;
  }

  const json = JSON.stringify(state.calibrationResult, null, 2);
  downloadFile(json, 'volcube_calibration.json', 'application/json');
  showToast('Exported to JSON', 'success');
}

function downloadFile(content: string, filename: string, mimeType: string): void {
  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

// =============================================================================
// Public API
// =============================================================================

export async function init(): Promise<void> {
  cacheElements();
  attachEventListeners();
  await loadSwaptionIndices();
  await loadSwaptionModels();
  await loadFxPairs();
  log.info('VolCube builder module initialised');
}

export const volcubeBuilder = {
  init,
  state,
  calibrate,
  exportCsv,
  exportJson,
};

export default volcubeBuilder;
