/**
 * Generic Pricer Module
 * Handles pricing and Greeks calculation for financial instruments
 */

import type {
  Instrument,
  ExpandedTrade,
  PricerState,
} from '@/types';
import { fetchInstruments, expandTrade, priceTrade, calculateGreeks } from '@/services/api';
import { createScopedLogger } from '@/utils/logger';
import { formatCurrency, formatNumberCompact, parseFormattedNumber, escapeHtml } from '@/utils/format';
import { getElementById } from '@/utils/dom';

const log = createScopedLogger('GenericPricer');

// =============================================================================
// Types
// =============================================================================

interface PricerElements {
  instrumentType: HTMLSelectElement | null;
  sampleTrade: HTMLSelectElement | null;
  parameterForm: HTMLDivElement | null;
  curve: HTMLSelectElement | null;
  valuationDate: HTMLInputElement | null;
  reportingCcy: HTMLSelectElement | null;
  useDefaults: HTMLInputElement | null;
  modelConfigFields: HTMLDivElement | null;
  numPaths: HTMLInputElement | null;
  numSteps: HTMLInputElement | null;
  seed: HTMLInputElement | null;
  rateBump: HTMLInputElement | null;
  fxBump: HTMLInputElement | null;
  expandCfBtn: HTMLButtonElement | null;
  resetCfBtn: HTMLButtonElement | null;
  cfTableContainer: HTMLDivElement | null;
  calculateBtn: HTMLButtonElement | null;
  pvResult: HTMLDivElement | null;
  legBreakdown: HTMLDivElement | null;
  greeksResult: HTMLDivElement | null;
}

type InstrumentParamsType = 'rates' | 'swap' | 'fx' | 'equity';

// =============================================================================
// State
// =============================================================================

const state: PricerState = {
  instruments: [],
  selectedInstrument: null,
  instrumentParams: {},
  expandedTrade: null,
  editedCashflows: {},
  cashflows: [],
  pricingResult: null,
  greeksResult: null,
};

const elements: PricerElements = {
  instrumentType: null,
  sampleTrade: null,
  parameterForm: null,
  curve: null,
  valuationDate: null,
  reportingCcy: null,
  useDefaults: null,
  modelConfigFields: null,
  numPaths: null,
  numSteps: null,
  seed: null,
  rateBump: null,
  fxBump: null,
  expandCfBtn: null,
  resetCfBtn: null,
  cfTableContainer: null,
  calculateBtn: null,
  pvResult: null,
  legBreakdown: null,
  greeksResult: null,
};

// =============================================================================
// Initialisation
// =============================================================================

function cacheElements(): void {
  elements.instrumentType = getElementById<HTMLSelectElement>('pricer-instrument-type');
  elements.sampleTrade = getElementById<HTMLSelectElement>('pricer-sample-trade');
  elements.parameterForm = getElementById<HTMLDivElement>('pricer-parameter-form');
  elements.curve = getElementById<HTMLSelectElement>('pricer-curve');
  elements.valuationDate = getElementById<HTMLInputElement>('pricer-valuation-date');
  elements.reportingCcy = getElementById<HTMLSelectElement>('pricer-reporting-ccy');
  elements.useDefaults = getElementById<HTMLInputElement>('pricer-use-defaults');
  elements.modelConfigFields = getElementById<HTMLDivElement>('pricer-model-config-fields');
  elements.numPaths = getElementById<HTMLInputElement>('pricer-num-paths');
  elements.numSteps = getElementById<HTMLInputElement>('pricer-num-steps');
  elements.seed = getElementById<HTMLInputElement>('pricer-seed');
  elements.rateBump = getElementById<HTMLInputElement>('pricer-rate-bump');
  elements.fxBump = getElementById<HTMLInputElement>('pricer-fx-bump');
  elements.expandCfBtn = getElementById<HTMLButtonElement>('pricer-expand-cf-btn');
  elements.resetCfBtn = getElementById<HTMLButtonElement>('pricer-reset-cf-btn');
  elements.cfTableContainer = getElementById<HTMLDivElement>('pricer-cf-table-container');
  elements.calculateBtn = getElementById<HTMLButtonElement>('pricer-calculate-btn');
  elements.pvResult = getElementById<HTMLDivElement>('pricer-pv-result');
  elements.legBreakdown = getElementById<HTMLDivElement>('pricer-leg-breakdown');
  elements.greeksResult = getElementById<HTMLDivElement>('pricer-greeks-result');
}

function attachEventListeners(): void {
  elements.calculateBtn?.addEventListener('click', () => void calculateAll());
  elements.instrumentType?.addEventListener('change', (e) => {
    state.selectedInstrument = (e.target as HTMLSelectElement).value;
    onInstrumentSelected();
  });
  elements.expandCfBtn?.addEventListener('click', () => void expandCashflows());
  elements.resetCfBtn?.addEventListener('click', resetCashflows);
  elements.useDefaults?.addEventListener('change', (e) => {
    if (elements.modelConfigFields) {
      elements.modelConfigFields.style.display = (e.target as HTMLInputElement).checked ? 'none' : 'block';
    }
  });
}

function setDefaultDate(): void {
  if (elements.valuationDate) {
    elements.valuationDate.value = new Date().toISOString().split('T')[0];
  }
}

async function loadInstruments(): Promise<void> {
  try {
    const data = await fetchInstruments();
    state.instruments = data.instruments || [];
    renderInstrumentSelector();
    await setDefaultInstrument();
  } catch (error) {
    log.error('Failed to load instruments', error);
    showApiNotAvailable();
  }
}

async function setDefaultInstrument(): Promise<void> {
  const ois = state.instruments.find(
    (inst) => (inst.instrumentType || inst.id || inst.type) === 'ois'
  );

  if (ois && elements.instrumentType) {
    elements.instrumentType.value = 'ois';
    state.selectedInstrument = 'ois';
    onInstrumentSelected();
    setTimeout(() => void expandCashflows(), 100);
  }
}

function showApiNotAvailable(): void {
  if (elements.instrumentType) {
    elements.instrumentType.innerHTML = '<option value="">Pricer API not available</option>';
    elements.instrumentType.disabled = true;
  }
  if (elements.calculateBtn) {
    elements.calculateBtn.disabled = true;
  }
  if (elements.pvResult) {
    elements.pvResult.innerHTML = `
      <div class="pricer-api-notice">
        <i class="fas fa-info-circle"></i>
        <p>Pricer API is not available in this build configuration.</p>
      </div>
    `;
  }
}

// =============================================================================
// Instrument Selection
// =============================================================================

function renderInstrumentSelector(): void {
  if (!elements.instrumentType) return;

  const groups: Record<string, Instrument[]> = {};
  state.instruments.forEach((inst) => {
    const assetClass = inst.assetClassName || inst.assetClass || 'Other';
    if (!groups[assetClass]) {
      groups[assetClass] = [];
    }
    groups[assetClass].push(inst);
  });

  let optionsHtml = '<option value="">Select instrument...</option>';
  for (const [assetClass, instruments] of Object.entries(groups)) {
    optionsHtml += `<optgroup label="${escapeHtml(assetClass)}">`;
    instruments.forEach((inst) => {
      const value = inst.instrumentType || inst.id || inst.type;
      const label = inst.displayName || inst.name || value;
      optionsHtml += `<option value="${escapeHtml(value)}">${escapeHtml(label)}</option>`;
    });
    optionsHtml += '</optgroup>';
  }

  elements.instrumentType.innerHTML = optionsHtml;

  if (elements.sampleTrade) {
    elements.sampleTrade.closest('.pricer-form-group')?.setAttribute('style', 'display: none');
  }
}

function onInstrumentSelected(): void {
  const instrumentType = state.selectedInstrument;

  if (!instrumentType) {
    hideParameterForm();
    return;
  }

  const instrument = state.instruments.find(
    (inst) => (inst.instrumentType || inst.id || inst.type) === instrumentType
  );

  if (instrument) {
    renderParameterForm(instrument);
  }
}

function renderParameterForm(instrument: Instrument): void {
  let formContainer = elements.parameterForm;
  if (!formContainer) {
    const tradeSection = elements.instrumentType?.closest('.pricer-section');
    if (tradeSection) {
      formContainer = document.createElement('div');
      formContainer.id = 'pricer-parameter-form';
      formContainer.className = 'pricer-parameter-form';
      tradeSection.appendChild(formContainer);
      elements.parameterForm = formContainer;
    } else {
      return;
    }
  }

  const requiredParams = instrument.requiredParams || [];
  const optionalParams = instrument.optionalParams || [];

  if (requiredParams.length === 0 && optionalParams.length === 0) {
    formContainer.innerHTML = `
      <div class="param-notice">
        <i class="fas fa-info-circle"></i>
        <span>This instrument has no configurable parameters.</span>
      </div>
    `;
    return;
  }

  state.instrumentParams = {};

  let html = '<div class="param-form-grid">';
  requiredParams.forEach((param) => {
    html += renderParameterField(param, true);
  });

  if (optionalParams.length > 0) {
    html += `<div class="param-section-divider"><span>Optional Parameters</span></div>`;
    optionalParams.forEach((param) => {
      html += renderParameterField(param, false);
    });
  }

  html += '</div>';
  formContainer.innerHTML = html;
  formContainer.style.display = 'block';

  formContainer.querySelectorAll<HTMLInputElement | HTMLSelectElement>('input, select').forEach((el) => {
    el.addEventListener('change', (e) => {
      const target = e.target as HTMLInputElement | HTMLSelectElement;
      const name = target.name;
      let value: string | number = target.value;

      if (target.type === 'number') {
        value = parseFloat(value) || 0;
      }

      state.instrumentParams[name] = value;
    });

    el.dispatchEvent(new Event('change'));
  });
}

function renderParameterField(param: Instrument['requiredParams'][0], isRequired: boolean): string {
  const name = param.name;
  const label = param.label || name;
  const fieldType = param.fieldType || 'string';
  const defaultValue = param.defaultValue !== null ? param.defaultValue : '';
  const options = param.options || [];
  const validation = param.validation || {};
  const requiredAttr = isRequired ? 'required' : '';
  const requiredMark = isRequired ? '<span class="required-mark">*</span>' : '';

  let inputHtml = '';

  switch (fieldType) {
    case 'select':
      inputHtml = `
        <select name="${escapeHtml(name)}" class="fancy-select" ${requiredAttr}>
          ${options.map((opt) => `
            <option value="${escapeHtml(opt.value)}" ${opt.value === defaultValue ? 'selected' : ''}>
              ${escapeHtml(opt.label)}
            </option>
          `).join('')}
        </select>
      `;
      break;

    case 'date': {
      const dateValue = defaultValue || new Date().toISOString().split('T')[0];
      inputHtml = `
        <input type="date" name="${escapeHtml(name)}" class="fancy-input"
               value="${dateValue}" ${requiredAttr}>
      `;
      break;
    }

    case 'number': {
      const min = validation.min !== undefined ? `min="${validation.min}"` : '';
      const max = validation.max !== undefined ? `max="${validation.max}"` : '';
      inputHtml = `
        <input type="number" name="${escapeHtml(name)}" class="fancy-input"
               value="${defaultValue}" step="any" ${min} ${max} ${requiredAttr}>
      `;
      break;
    }

    default:
      inputHtml = `
        <input type="text" name="${escapeHtml(name)}" class="fancy-input"
               value="${defaultValue}" ${requiredAttr}>
      `;
  }

  return `
    <div class="param-field">
      <label for="${escapeHtml(name)}">${escapeHtml(label)}${requiredMark}</label>
      ${inputHtml}
    </div>
  `;
}

function hideParameterForm(): void {
  if (elements.parameterForm) {
    elements.parameterForm.style.display = 'none';
  }
  state.instrumentParams = {};
}

function getInstrumentParamsType(instrumentType: string): InstrumentParamsType {
  const ratesTypes = ['deposit', 'fra', 'futures', 'ois'];
  const swapTypes = ['basis_swap', 'irs'];
  const fxTypes = ['fx_forward', 'fx_option', 'cross_currency_swap'];
  const equityTypes = ['equity_vanilla_option', 'equity_forward'];

  if (ratesTypes.includes(instrumentType)) return 'rates';
  if (swapTypes.includes(instrumentType)) return 'swap';
  if (fxTypes.includes(instrumentType)) return 'fx';
  if (equityTypes.includes(instrumentType)) return 'equity';
  return 'rates';
}

// =============================================================================
// Cashflow Expansion
// =============================================================================

async function expandCashflows(): Promise<void> {
  if (!state.selectedInstrument) {
    alert('Please select an instrument type first');
    return;
  }

  try {
    if (elements.expandCfBtn) {
      elements.expandCfBtn.disabled = true;
      elements.expandCfBtn.innerHTML = '<i class="fas fa-spinner fa-spin"></i> Expanding...';
    }

    const paramsType = getInstrumentParamsType(state.selectedInstrument);
    const request = {
      instrumentType: state.selectedInstrument,
      params: {
        type: paramsType,
        ...state.instrumentParams,
      },
    };

    const data = await expandTrade(request);
    state.expandedTrade = data;
    renderExpandedTrade(data);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    log.error('Failed to expand cashflows', message);
    alert('Error: ' + message);
  } finally {
    if (elements.expandCfBtn) {
      elements.expandCfBtn.disabled = false;
      elements.expandCfBtn.innerHTML = '<i class="fas fa-expand"></i> Expand CF';
    }
  }
}

function renderExpandedTrade(trade: ExpandedTrade): void {
  if (!elements.cfTableContainer) return;

  if (!trade || !trade.legs || trade.legs.length === 0) {
    elements.cfTableContainer.innerHTML = '<p class="pricer-placeholder">No cashflows to display</p>';
    return;
  }

  let html = `
    <div class="expanded-trade-header">
      <div class="expanded-trade-info">
        <div class="trade-id-badge">
          <i class="fas fa-hashtag"></i> ${escapeHtml(trade.tradeId)}
        </div>
        <div class="trade-type-badge">
          <i class="fas fa-file-contract"></i> ${escapeHtml(trade.tradeType)}
        </div>
      </div>
      <div class="cf-edit-indicator" id="cf-edit-indicator" style="display: none;">
        <i class="fas fa-edit"></i> Modified
      </div>
    </div>
  `;

  trade.legs.forEach((leg, legIdx) => {
    const directionClass = leg.direction === 'Payer' ? 'payer' : 'receiver';
    html += `
      <div class="leg-section" data-leg-idx="${legIdx}">
        <div class="leg-header ${directionClass}">
          <span class="leg-badge">${legIdx + 1}</span>
          <span class="leg-direction-tag ${directionClass}">${escapeHtml(leg.direction)}</span>
          <span class="leg-currency-tag">${escapeHtml(leg.currency)}</span>
          <span class="leg-type-tag">${escapeHtml(leg.legType)}</span>
          ${leg.rateIndex ? `<span class="leg-index-tag">${escapeHtml(leg.rateIndex)}</span>` : ''}
        </div>
        <div class="cf-table-wrapper">
          <table class="pricer-cf-table editable">
            <thead>
              <tr>
                <th class="col-date">Payment</th>
                <th class="col-period">Accrual Period</th>
                <th class="col-yf">YF</th>
                <th class="col-notional">Notional</th>
                <th class="col-rate">Rate</th>
                <th class="col-type">Type</th>
              </tr>
            </thead>
            <tbody>
              ${leg.cashflows.map((cf, cfIdx) => {
                const key = `${legIdx}-${cfIdx}`;
                const edited = state.editedCashflows[key] || {};
                const notional = edited.notional !== undefined ? edited.notional : cf.notional;
                const rate = edited.rate !== undefined ? edited.rate : cf.rate;
                const isEdited = edited.notional !== undefined || edited.rate !== undefined;

                return `
                  <tr class="${isEdited ? 'edited' : ''}" data-cf-key="${key}">
                    <td class="col-date">${escapeHtml(cf.paymentDate)}</td>
                    <td class="col-period">
                      <span class="period-start">${escapeHtml(cf.accrualStart)}</span>
                      <span class="period-arrow">\u2192</span>
                      <span class="period-end">${escapeHtml(cf.accrualEnd)}</span>
                    </td>
                    <td class="col-yf">${cf.yearFraction.toFixed(4)}</td>
                    <td class="col-notional">
                      <input type="text"
                             class="cf-input notional-input ${isEdited ? 'modified' : ''}"
                             data-leg="${legIdx}"
                             data-cf="${cfIdx}"
                             data-field="notional"
                             data-original="${cf.notional}"
                             value="${formatNumberCompact(notional)}">
                    </td>
                    <td class="col-rate">
                      ${rate !== null ? `
                        <input type="text"
                               class="cf-input rate-input ${isEdited ? 'modified' : ''}"
                               data-leg="${legIdx}"
                               data-cf="${cfIdx}"
                               data-field="rate"
                               data-original="${cf.rate}"
                               value="${((rate ?? 0) * 100).toFixed(4)}">
                        <span class="rate-unit">%</span>
                      ` : '<span class="rate-floating">Floating</span>'}
                    </td>
                    <td class="col-type">
                      <span class="payoff-badge ${cf.payoffType.toLowerCase()}">${escapeHtml(cf.payoffType)}</span>
                      ${cf.rateIndex ? `<span class="index-badge">${escapeHtml(cf.rateIndex)}</span>` : ''}
                    </td>
                  </tr>
                `;
              }).join('')}
            </tbody>
          </table>
        </div>
      </div>
    `;
  });

  html += `
    <div class="trade-metadata">
      <span><i class="fas fa-layer-group"></i> ${trade.metadata.totalLegs} legs</span>
      <span><i class="fas fa-coins"></i> ${trade.metadata.totalCashflows} cashflows</span>
      <span><i class="fas fa-clock"></i> ${trade.metadata.processingTimeMs.toFixed(2)}ms</span>
      <button class="reset-edits-btn" id="reset-edits-btn" style="display: none;">
        <i class="fas fa-undo"></i> Reset Edits
      </button>
    </div>
  `;

  elements.cfTableContainer.innerHTML = html;
  attachCashflowEditListeners();
}

function attachCashflowEditListeners(): void {
  const container = elements.cfTableContainer;
  if (!container) return;

  container.querySelectorAll<HTMLInputElement>('.notional-input').forEach((input) => {
    input.addEventListener('change', (e) => onCashflowEdit(e, 'notional'));
    input.addEventListener('focus', (e) => (e.target as HTMLInputElement).select());
  });

  container.querySelectorAll<HTMLInputElement>('.rate-input').forEach((input) => {
    input.addEventListener('change', (e) => onCashflowEdit(e, 'rate'));
    input.addEventListener('focus', (e) => (e.target as HTMLInputElement).select());
  });

  const resetBtn = getElementById<HTMLButtonElement>('reset-edits-btn');
  resetBtn?.addEventListener('click', resetCashflowEdits);
}

function onCashflowEdit(event: Event, fieldType: 'notional' | 'rate'): void {
  const input = event.target as HTMLInputElement;
  const legIdx = parseInt(input.dataset.leg || '0');
  const cfIdx = parseInt(input.dataset.cf || '0');
  const original = parseFloat(input.dataset.original || '0');
  const key = `${legIdx}-${cfIdx}`;

  let value: number;
  if (fieldType === 'notional') {
    value = parseFormattedNumber(input.value);
    input.value = formatNumberCompact(value);
  } else {
    value = parseFloat(input.value.replace(/[,%]/g, '')) / 100;
    input.value = (value * 100).toFixed(4);
  }

  if (!state.editedCashflows[key]) {
    state.editedCashflows[key] = {};
  }

  const isChanged = Math.abs(value - original) > 1e-10;

  if (isChanged) {
    state.editedCashflows[key][fieldType] = value;
    input.classList.add('modified');
    input.closest('tr')?.classList.add('edited');
  } else {
    delete state.editedCashflows[key][fieldType];
    if (Object.keys(state.editedCashflows[key]).length === 0) {
      delete state.editedCashflows[key];
    }
    input.classList.remove('modified');
    const row = input.closest('tr');
    const otherModified = row?.querySelector('.cf-input.modified');
    if (!otherModified) {
      row?.classList.remove('edited');
    }
  }

  updateEditIndicator();
}

function updateEditIndicator(): void {
  const indicator = getElementById('cf-edit-indicator');
  const resetBtn = getElementById('reset-edits-btn');
  const hasEdits = Object.keys(state.editedCashflows || {}).length > 0;

  if (indicator) {
    indicator.style.display = hasEdits ? 'flex' : 'none';
  }
  if (resetBtn) {
    resetBtn.style.display = hasEdits ? 'inline-flex' : 'none';
  }
}

function resetCashflowEdits(): void {
  state.editedCashflows = {};
  if (state.expandedTrade) {
    renderExpandedTrade(state.expandedTrade);
  }
}

function resetCashflows(): void {
  state.expandedTrade = null;
  state.editedCashflows = {};
  state.cashflows = [];
  if (elements.cfTableContainer) {
    elements.cfTableContainer.innerHTML = '<p class="pricer-placeholder">Click "Expand CF" to view cashflows</p>';
  }
}

// =============================================================================
// Pricing
// =============================================================================

function buildPricingRequest() {
  const useDefaults = elements.useDefaults?.checked ?? true;
  const editedCashflows = state.editedCashflows || {};

  const legs: Array<{
    currency: string;
    direction: 'payer' | 'receiver';
    cashflows: Array<{ paymentDate: string; amount: number }>;
  }> = [];

  if (state.expandedTrade?.legs) {
    state.expandedTrade.legs.forEach((leg, legIdx) => {
      const cashflows = leg.cashflows.map((cf, cfIdx) => {
        const key = `${legIdx}-${cfIdx}`;
        const edited = editedCashflows[key] || {};

        const notional = edited.notional !== undefined ? edited.notional : cf.notional;
        const rate = edited.rate !== undefined ? edited.rate : (cf.rate || 0);

        return {
          paymentDate: cf.paymentDate,
          amount: notional * rate * cf.yearFraction,
        };
      });

      legs.push({
        currency: leg.currency,
        direction: leg.direction.toLowerCase() as 'payer' | 'receiver',
        cashflows,
      });
    });
  }

  return {
    valuationDate: elements.valuationDate?.value || new Date().toISOString().split('T')[0],
    reportingCurrency: elements.reportingCcy?.value || 'USD',
    legs,
    modelConfig: useDefaults
      ? null
      : {
          numPaths: parseInt(elements.numPaths?.value || '10000'),
          numSteps: parseInt(elements.numSteps?.value || '100'),
          seed: elements.seed?.value ? parseInt(elements.seed.value) : null,
        },
  };
}

async function calculateAll(): Promise<void> {
  if (!state.selectedInstrument) {
    alert('Please select an instrument type first');
    return;
  }

  if (!state.expandedTrade) {
    alert('Please expand cashflows first');
    return;
  }

  if (elements.calculateBtn) {
    elements.calculateBtn.disabled = true;
    elements.calculateBtn.innerHTML = '<i class="fas fa-spinner fa-spin"></i> Calculating...';
  }

  try {
    await Promise.all([price(), calculateGreeksHandler()]);
    log.info('Price & Risks calculation completed', { instrument: state.selectedInstrument });
  } catch (error) {
    log.error('Calculation failed', error);
  } finally {
    if (elements.calculateBtn) {
      elements.calculateBtn.disabled = false;
      elements.calculateBtn.innerHTML = '<i class="fas fa-play"></i> Price & Risks';
    }
  }
}

async function price(): Promise<void> {
  if (!state.selectedInstrument || !state.expandedTrade) return;

  try {
    const request = buildPricingRequest();
    state.pricingResult = await priceTrade(request);
    renderPricingResult();
    log.info('Pricing completed', { instrument: state.selectedInstrument });
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    log.error('Pricing failed', message);
    showPricingError(message);
  }
}

function showPricingError(message: string): void {
  if (elements.pvResult) {
    elements.pvResult.innerHTML = `
      <div class="pricer-error-card">
        <div class="error-icon"><i class="fas fa-exclamation-triangle"></i></div>
        <div class="error-message">Pricing failed: ${escapeHtml(message)}</div>
      </div>
    `;
  }
}

function renderPricingResult(): void {
  if (!state.pricingResult) return;

  const result = state.pricingResult;
  const pv = result.totalPv ?? result.pv ?? 0;
  const ccy = result.currency || 'USD';

  if (elements.pvResult) {
    elements.pvResult.innerHTML = `
      <div class="pricer-pv-card ${pv >= 0 ? 'positive' : 'negative'}">
        <div class="pv-label">Present Value</div>
        <div class="pv-value">${formatCurrency(pv, ccy)}</div>
        <div class="pv-meta">
          <span>Reporting CCY: ${escapeHtml(ccy)}</span>
        </div>
      </div>
    `;
  }

  if (elements.legBreakdown && result.legs) {
    elements.legBreakdown.innerHTML = `
      <table class="pricer-leg-table">
        <thead>
          <tr>
            <th>Leg</th>
            <th>Direction</th>
            <th>PV</th>
          </tr>
        </thead>
        <tbody>
          ${result.legs.map((leg, idx) => `
            <tr>
              <td>Leg ${idx + 1}</td>
              <td><span class="leg-direction ${leg.direction}">${escapeHtml(leg.direction)}</span></td>
              <td class="${leg.pv >= 0 ? 'positive' : 'negative'}">${formatCurrency(leg.pv, ccy)}</td>
            </tr>
          `).join('')}
        </tbody>
        <tfoot>
          <tr>
            <td colspan="2"><strong>Total</strong></td>
            <td class="${pv >= 0 ? 'positive' : 'negative'}"><strong>${formatCurrency(pv, ccy)}</strong></td>
          </tr>
        </tfoot>
      </table>
    `;
  }
}

async function calculateGreeksHandler(): Promise<void> {
  if (!state.selectedInstrument || !state.expandedTrade) return;

  try {
    const request = {
      ...buildPricingRequest(),
      bumpSizes: {
        rateBumpBp: parseFloat(elements.rateBump?.value || '1'),
        fxBumpPct: parseFloat(elements.fxBump?.value || '1'),
        volBumpPct: 1.0,
      },
    };

    state.greeksResult = await calculateGreeks(request);
    renderGreeksResult();
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    log.error('Greeks calculation failed', message);
    showGreeksError(message);
  }
}

function showGreeksError(message: string): void {
  const container = elements.greeksResult || getElementById('pricer-greeks-result');
  if (container) {
    container.innerHTML = `
      <div class="pricer-error-card">
        <div class="error-icon"><i class="fas fa-exclamation-triangle"></i></div>
        <div class="error-message">Greeks calculation failed: ${escapeHtml(message)}</div>
      </div>
    `;
  }
}

function renderGreeksResult(): void {
  if (!state.greeksResult) return;

  const result = state.greeksResult;
  const ccy = result.currency || 'USD';

  const container = elements.greeksResult || getElementById('pricer-greeks-result');
  if (container) {
    container.innerHTML = `
      <div class="pricer-greeks-grid">
        <div class="greek-card">
          <div class="greek-label">DV01 (Delta)</div>
          <div class="greek-value ${result.delta >= 0 ? 'positive' : 'negative'}">${formatCurrency(result.delta, ccy)}</div>
        </div>
        ${result.gamma !== null ? `
          <div class="greek-card">
            <div class="greek-label">Gamma</div>
            <div class="greek-value">${formatCurrency(result.gamma, ccy)}</div>
          </div>
        ` : ''}
        ${result.theta !== null ? `
          <div class="greek-card">
            <div class="greek-label">Theta</div>
            <div class="greek-value ${result.theta >= 0 ? 'positive' : 'negative'}">${formatCurrency(result.theta, ccy)}</div>
          </div>
        ` : ''}
        ${result.vega !== null ? `
          <div class="greek-card">
            <div class="greek-label">Vega</div>
            <div class="greek-value">${formatCurrency(result.vega, ccy)}</div>
          </div>
        ` : ''}
      </div>
    `;
  }
}

// =============================================================================
// Public API
// =============================================================================

export async function init(): Promise<void> {
  cacheElements();
  attachEventListeners();
  setDefaultDate();
  await loadInstruments();
  log.info('Generic pricer module initialised');
}

export const genericPricer = {
  init,
  state,
  elements,
  calculateAll,
  expandCashflows,
  resetCashflows,
};

export default genericPricer;
