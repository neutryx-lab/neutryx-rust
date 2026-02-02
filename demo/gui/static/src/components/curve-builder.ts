/**
 * Curve Builder Module
 * Handles yield curve construction from market instruments
 */

import { createScopedLogger } from '@/utils/logger';
import { getElementById, showToast } from '@/utils/dom';
import { escapeHtml } from '@/utils/format';

const log = createScopedLogger('CurveBuilder');

// =============================================================================
// Types
// =============================================================================

interface CurveInstrument {
  id: string;
  type: string;
  tenor: string;
  tenorYears: number;
  rate: number;
  enabled: boolean;
}

interface BuildResult {
  curveId: string;
  discountFactors: Array<{ date: string; yearFraction: number; value: number }>;
  zeroRates: Array<{ date: string; yearFraction: number; rate: number }>;
  forwardRates: Array<{ startDate: string; endDate: string; rate: number }>;
  metadata: {
    instrumentCount: number;
    interpolation: string;
    processingTimeMs: number;
  };
}

interface CurveBuilderState {
  indices: string[];
  selectedIndex: string | null;
  instruments: CurveInstrument[];
  originalInstruments: CurveInstrument[];
  buildResult: BuildResult | null;
  hasChanges: boolean;
  enableJumps: boolean;
  cbEvents: Array<{ date: string; expectedJumpBps: number; centralBank?: string }>;
}

// =============================================================================
// State
// =============================================================================

const state: CurveBuilderState = {
  indices: [],
  selectedIndex: null,
  instruments: [],
  originalInstruments: [],
  buildResult: null,
  hasChanges: false,
  enableJumps: false,
  cbEvents: [],
};

let elements: Record<string, HTMLElement | null> = {};
let initialised = false;

// =============================================================================
// Element Caching
// =============================================================================

function cacheElements(): void {
  elements = {
    indexContainer: getElementById('index-selector-container'),
    instrumentTable: getElementById('instrument-table-container'),
    settingsContainer: getElementById('builder-settings-container'),
    buildBtn: getElementById('build-curve-btn'),
    exportRatesBtn: getElementById('export-rates-btn'),
    importRatesBtn: getElementById('import-rates-btn'),
    resetRatesBtn: getElementById('reset-rates-btn'),
    changesIndicator: getElementById('changes-indicator'),
    rebuildNotification: getElementById('rebuild-notification'),
    buildStatus: getElementById('build-status'),
    buildSummary: getElementById('build-summary'),
    parameterChartShort: getElementById('parameter-chart-short'),
    parameterChartLong: getElementById('parameter-chart-long'),
    chartPlaceholder: getElementById('chart-placeholder'),
    parameterTabsContainer: getElementById('parameter-tabs-container'),
    parameterTableContainer: getElementById('parameter-table-container'),
    errorContainer: getElementById('error-container'),
    errorMessage: getElementById('error-message'),
    loadingOverlay: getElementById('loading-overlay'),
  };
}

// =============================================================================
// Event Listeners
// =============================================================================

function attachEventListeners(): void {
  elements.buildBtn?.addEventListener('click', () => void buildCurve());
  elements.resetRatesBtn?.addEventListener('click', resetRates);
  elements.exportRatesBtn?.addEventListener('click', exportRates);
}

// =============================================================================
// Data Loading
// =============================================================================

async function loadIndices(): Promise<void> {
  try {
    const response = await fetch('/api/curves/indices');
    if (!response.ok) throw new Error('Failed to load indices');
    const data = await response.json();
    state.indices = data.indices || [];
    renderIndexSelector();
  } catch (error) {
    log.error('Failed to load indices', error);
  }
}

async function loadInstruments(index: string): Promise<void> {
  try {
    const response = await fetch(`/api/curves/instruments/${index}`);
    if (!response.ok) throw new Error('Failed to load instruments');
    const data = await response.json();
    state.instruments = data.instruments || [];
    state.originalInstruments = JSON.parse(JSON.stringify(state.instruments));
    state.hasChanges = false;
    renderInstrumentsTable();
    updateChangesIndicator();
  } catch (error) {
    log.error('Failed to load instruments', error);
  }
}

// =============================================================================
// Rendering
// =============================================================================

function renderIndexSelector(): void {
  if (!elements.indexContainer) return;

  const html = `
    <select id="curve-index-selector" class="fancy-select">
      <option value="">Select index...</option>
      ${state.indices.map((idx) => `<option value="${escapeHtml(idx)}">${escapeHtml(idx)}</option>`).join('')}
    </select>
  `;
  elements.indexContainer.innerHTML = html;

  const selector = getElementById<HTMLSelectElement>('curve-index-selector');
  selector?.addEventListener('change', (e) => {
    state.selectedIndex = (e.target as HTMLSelectElement).value;
    if (state.selectedIndex) {
      void loadInstruments(state.selectedIndex);
    }
  });
}

function renderInstrumentsTable(): void {
  if (!elements.instrumentTable) return;

  if (state.instruments.length === 0) {
    elements.instrumentTable.innerHTML = `
      <div class="empty-state">
        <i class="fas fa-chart-line"></i>
        <p>Select an index to load instruments</p>
      </div>
    `;
    return;
  }

  const html = `
    <table class="instruments-table">
      <thead>
        <tr>
          <th>Type</th>
          <th>Tenor</th>
          <th>Rate (%)</th>
          <th>Enabled</th>
        </tr>
      </thead>
      <tbody>
        ${state.instruments
          .map(
            (inst, idx) => `
          <tr data-idx="${idx}">
            <td>${escapeHtml(inst.type)}</td>
            <td>${escapeHtml(inst.tenor)}</td>
            <td>
              <input type="number" class="rate-input" value="${(inst.rate * 100).toFixed(4)}"
                     step="0.0001" data-idx="${idx}">
            </td>
            <td>
              <input type="checkbox" ${inst.enabled ? 'checked' : ''} data-idx="${idx}">
            </td>
          </tr>
        `
          )
          .join('')}
      </tbody>
    </table>
  `;
  elements.instrumentTable.innerHTML = html;

  // Attach change handlers
  elements.instrumentTable.querySelectorAll<HTMLInputElement>('.rate-input').forEach((input) => {
    input.addEventListener('change', (e) => {
      const idx = parseInt((e.target as HTMLElement).dataset.idx || '0');
      const value = parseFloat((e.target as HTMLInputElement).value) / 100;
      state.instruments[idx].rate = value;
      checkForChanges();
    });
  });

  elements.instrumentTable.querySelectorAll<HTMLInputElement>('input[type="checkbox"]').forEach((input) => {
    input.addEventListener('change', (e) => {
      const idx = parseInt((e.target as HTMLElement).dataset.idx || '0');
      state.instruments[idx].enabled = (e.target as HTMLInputElement).checked;
      checkForChanges();
    });
  });
}

function checkForChanges(): void {
  state.hasChanges = JSON.stringify(state.instruments) !== JSON.stringify(state.originalInstruments);
  updateChangesIndicator();
}

function updateChangesIndicator(): void {
  if (elements.changesIndicator) {
    elements.changesIndicator.style.display = state.hasChanges ? 'block' : 'none';
  }
  if (elements.rebuildNotification) {
    elements.rebuildNotification.style.display = state.hasChanges ? 'block' : 'none';
  }
}

// =============================================================================
// Actions
// =============================================================================

async function buildCurve(): Promise<void> {
  if (!state.selectedIndex) {
    showToast('Please select an index first', 'warning');
    return;
  }

  const enabledInstruments = state.instruments.filter((inst) => inst.enabled);
  if (enabledInstruments.length === 0) {
    showToast('No instruments enabled', 'warning');
    return;
  }

  if (elements.buildBtn) {
    (elements.buildBtn as HTMLButtonElement).disabled = true;
    elements.buildBtn.innerHTML = '<i class="fas fa-spinner fa-spin"></i> Building...';
  }

  try {
    const response = await fetch('/api/curves/build', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        index: state.selectedIndex,
        instruments: enabledInstruments.map((inst) => ({
          type: inst.type,
          tenor: inst.tenor,
          rate: inst.rate,
        })),
        enableJumps: state.enableJumps,
        cbEvents: state.cbEvents,
      }),
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.message || 'Build failed');
    }

    state.buildResult = await response.json();
    state.originalInstruments = JSON.parse(JSON.stringify(state.instruments));
    state.hasChanges = false;
    updateChangesIndicator();
    renderBuildResult();
    showToast('Curve built successfully', 'success');
    log.info('Curve built successfully', { index: state.selectedIndex });
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    log.error('Build failed', message);
    showToast(`Build failed: ${message}`, 'error');
  } finally {
    if (elements.buildBtn) {
      (elements.buildBtn as HTMLButtonElement).disabled = false;
      elements.buildBtn.innerHTML = '<i class="fas fa-hammer"></i> Build Curve';
    }
  }
}

function renderBuildResult(): void {
  if (!state.buildResult) return;

  if (elements.buildStatus) {
    elements.buildStatus.innerHTML = `
      <div class="build-success">
        <i class="fas fa-check-circle"></i>
        <span>Curve built successfully</span>
      </div>
    `;
  }

  if (elements.buildSummary) {
    // Support both old metadata format and new flat format from backend
    const instrumentCount = state.buildResult.instrument_count ?? state.buildResult.metadata?.instrumentCount ?? 0;
    const interpolation = state.buildResult.interpolation ?? state.buildResult.metadata?.interpolation ?? 'Linear';
    const processingTimeMs = state.buildResult.calculation_time_ms ?? state.buildResult.metadata?.processingTimeMs ?? 0;

    elements.buildSummary.innerHTML = `
      <div class="summary-item">
        <span class="label">Instruments:</span>
        <span class="value">${instrumentCount}</span>
      </div>
      <div class="summary-item">
        <span class="label">Interpolation:</span>
        <span class="value">${escapeHtml(String(interpolation))}</span>
      </div>
      <div class="summary-item">
        <span class="label">Processing Time:</span>
        <span class="value">${Number(processingTimeMs).toFixed(2)} ms</span>
      </div>
    `;
  }
}

function resetRates(): void {
  state.instruments = JSON.parse(JSON.stringify(state.originalInstruments));
  state.hasChanges = false;
  renderInstrumentsTable();
  updateChangesIndicator();
  showToast('Rates reset to original values', 'info');
}

function exportRates(): void {
  if (state.instruments.length === 0) {
    showToast('No instruments to export', 'warning');
    return;
  }

  const csv = [
    'Type,Tenor,Rate,Enabled',
    ...state.instruments.map(
      (inst) => `${inst.type},${inst.tenor},${(inst.rate * 100).toFixed(4)},${inst.enabled}`
    ),
  ].join('\n');

  const blob = new Blob([csv], { type: 'text/csv' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = `curve_instruments_${state.selectedIndex || 'unknown'}.csv`;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
  showToast('Instruments exported', 'success');
}

function formatTenor(years: number): string {
  if (years == null || isNaN(years)) return '-';
  if (Math.abs(years) < 0.001) return '0';
  if (years >= 1 && Math.abs(years - Math.round(years)) < 0.001) {
    return `${Math.round(years)}Y`;
  }
  const months = years * 12;
  const roundedMonths = Math.round(months);
  if (roundedMonths > 0 && Math.abs(months - roundedMonths) < 0.1) {
    if (roundedMonths >= 12 && roundedMonths % 12 === 0) {
      return `${roundedMonths / 12}Y`;
    }
    return `${roundedMonths}M`;
  }
  const days = years * 365;
  if (days < 7 && days > 0) {
    const roundedDays = Math.round(days);
    if (roundedDays === 1) return 'O/N';
    if (roundedDays > 0) return `${roundedDays}D`;
  }
  if (years < 1) {
    return `${(years * 12).toFixed(1)}M`;
  }
  return `${years.toFixed(2)}Y`;
}

// =============================================================================
// Public API
// =============================================================================

export async function init(): Promise<void> {
  if (initialised) return;

  cacheElements();
  attachEventListeners();

  try {
    await loadIndices();
    initialised = true;
    log.info('Curve builder module initialised');
  } catch (error) {
    log.error('Init failed', error);
  }
}

export const curveBuilder = {
  init,
  state,
  buildCurve,
  resetRates,
  exportRates,
  formatTenor,
};

export default curveBuilder;
