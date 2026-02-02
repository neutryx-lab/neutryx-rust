/**
 * Trade Expansion View Module
 * Handles trade expansion and cashflow generation
 */

import { createScopedLogger } from '@/utils/logger';
import { expandTrade } from '@/services/api';
import type { ExpandedTrade, Cashflow } from '@/types/api';
import { formatNumber, formatDate } from '@/utils/format';

const log = createScopedLogger('TradeExpansion');

// =============================================================================
// Types
// =============================================================================

interface TradeExpansionState {
  expandedTrade: ExpandedTrade | null;
  isLoading: boolean;
}

interface Elements {
  expandBtn: HTMLButtonElement | null;
  tradeType: HTMLSelectElement | null;
  notional: HTMLInputElement | null;
  currency: HTMLSelectElement | null;
  tenor: HTMLSelectElement | null;
  cashflowsContainer: HTMLElement | null;
}

// =============================================================================
// State
// =============================================================================

const state: TradeExpansionState = {
  expandedTrade: null,
  isLoading: false,
};

const elements: Elements = {
  expandBtn: null,
  tradeType: null,
  notional: null,
  currency: null,
  tenor: null,
  cashflowsContainer: null,
};

let initialised = false;

// =============================================================================
// DOM Helpers
// =============================================================================

function getElements(): void {
  elements.expandBtn = document.getElementById('expand-trade-btn') as HTMLButtonElement;
  elements.tradeType = document.getElementById('trade-type') as HTMLSelectElement;
  elements.notional = document.getElementById('trade-notional') as HTMLInputElement;
  elements.currency = document.getElementById('trade-currency') as HTMLSelectElement;
  elements.tenor = document.getElementById('trade-tenor') as HTMLSelectElement;
  elements.cashflowsContainer = document.getElementById('expansion-cashflows');
}

// =============================================================================
// Rendering
// =============================================================================

function renderPlaceholder(): void {
  if (!elements.cashflowsContainer) return;
  elements.cashflowsContainer.innerHTML = `
    <div class="placeholder">
      <i class="fas fa-expand-arrows-alt"></i>
      <p>Click Expand to generate cashflows</p>
    </div>
  `;
}

function renderLoading(): void {
  if (!elements.cashflowsContainer) return;
  elements.cashflowsContainer.innerHTML = `
    <div class="placeholder">
      <i class="fas fa-spinner fa-spin"></i>
      <p>Expanding trade...</p>
    </div>
  `;
}

function renderError(message: string): void {
  if (!elements.cashflowsContainer) return;
  elements.cashflowsContainer.innerHTML = `
    <div class="placeholder error">
      <i class="fas fa-exclamation-triangle"></i>
      <p>${message}</p>
    </div>
  `;
}

function renderCashflows(trade: ExpandedTrade): void {
  if (!elements.cashflowsContainer) return;

  const { legs, metadata } = trade;

  const html = `
    <div class="expanded-trade-summary">
      <div class="summary-stats">
        <span class="stat">
          <i class="fas fa-layer-group"></i>
          ${metadata.totalLegs} legs
        </span>
        <span class="stat">
          <i class="fas fa-coins"></i>
          ${metadata.totalCashflows} cashflows
        </span>
        <span class="stat">
          <i class="fas fa-clock"></i>
          ${metadata.processingTimeMs.toFixed(1)}ms
        </span>
      </div>
    </div>
    <div class="cashflow-legs">
      ${legs.map((leg, idx) => renderLeg(leg, idx)).join('')}
    </div>
  `;

  elements.cashflowsContainer.innerHTML = html;
}

function renderLeg(leg: { direction: string; currency: string; legType: string; rateIndex?: string; cashflows: Cashflow[] }, index: number): string {
  const directionClass = leg.direction === 'Payer' ? 'payer' : 'receiver';
  const directionIcon = leg.direction === 'Payer' ? 'arrow-up' : 'arrow-down';

  return `
    <div class="cashflow-leg ${directionClass}">
      <div class="leg-header">
        <span class="leg-title">
          <i class="fas fa-${directionIcon}"></i>
          Leg ${index + 1}: ${leg.direction} ${leg.legType}
        </span>
        <span class="leg-info">
          ${leg.currency}${leg.rateIndex ? ` | ${leg.rateIndex}` : ''}
        </span>
      </div>
      <div class="cashflow-table-wrapper">
        <table class="cashflow-data-table">
          <thead>
            <tr>
              <th>Payment Date</th>
              <th>Accrual Start</th>
              <th>Accrual End</th>
              <th>Year Fraction</th>
              <th>Notional</th>
              <th>Rate</th>
              <th>Type</th>
            </tr>
          </thead>
          <tbody>
            ${leg.cashflows.map(cf => renderCashflowRow(cf)).join('')}
          </tbody>
        </table>
      </div>
    </div>
  `;
}

function renderCashflowRow(cf: Cashflow): string {
  return `
    <tr>
      <td>${formatDate(cf.paymentDate)}</td>
      <td>${formatDate(cf.accrualStart)}</td>
      <td>${formatDate(cf.accrualEnd)}</td>
      <td>${cf.yearFraction.toFixed(6)}</td>
      <td class="number">${formatNumber(cf.notional)}</td>
      <td class="number">${cf.rate !== null ? (cf.rate * 100).toFixed(4) + '%' : '-'}</td>
      <td><span class="payoff-badge">${cf.payoffType}</span></td>
    </tr>
  `;
}

// =============================================================================
// API Calls
// =============================================================================

// Map frontend trade types to backend instrument types
const INSTRUMENT_TYPE_MAP: Record<string, string> = {
  swap: 'IRS',
  fra: 'IRS',
  cap: 'IRS',
  swaption: 'IRS',
  fxforward: 'FxForward',
};

async function handleExpand(): Promise<void> {
  if (state.isLoading) return;

  const tradeType = elements.tradeType?.value || 'swap';
  const instrumentType = INSTRUMENT_TYPE_MAP[tradeType] || 'IRS';
  const notional = parseFloat(elements.notional?.value || '10000000');
  const currency = elements.currency?.value || 'USD';
  const tenor = elements.tenor?.value || '5Y';

  state.isLoading = true;
  renderLoading();

  if (elements.expandBtn) {
    elements.expandBtn.disabled = true;
    elements.expandBtn.innerHTML = '<i class="fas fa-spinner fa-spin"></i> Expanding...';
  }

  try {
    const request = {
      instrumentType,
      params: {
        type: tradeType,
        notional,
        currency,
        tenor,
        startDate: new Date().toISOString().split('T')[0],
      },
    };

    log.debug('Expanding trade', request);
    const result = await expandTrade(request);
    state.expandedTrade = result;
    renderCashflows(result);
    log.info('Trade expanded successfully', { cashflows: result.metadata.totalCashflows });
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Unknown error';
    log.error('Failed to expand trade', message);
    renderError(`Failed to expand trade: ${message}`);
  } finally {
    state.isLoading = false;
    if (elements.expandBtn) {
      elements.expandBtn.disabled = false;
      elements.expandBtn.innerHTML = '<i class="fas fa-expand-arrows-alt"></i> Expand';
    }
  }
}

// =============================================================================
// Event Handlers
// =============================================================================

function setupEventListeners(): void {
  if (!elements.expandBtn) {
    log.warn('Expand button not found');
    return;
  }

  // Remove existing listener to prevent duplicates
  elements.expandBtn.removeEventListener('click', handleExpandClick);
  elements.expandBtn.addEventListener('click', handleExpandClick);
  log.debug('Event listeners attached');
}

function handleExpandClick(): void {
  void handleExpand();
}

// =============================================================================
// Public API
// =============================================================================

export async function init(): Promise<void> {
  // Always refresh elements when view is activated
  getElements();

  if (!elements.expandBtn) {
    log.error('Trade expansion view elements not found in DOM');
    return;
  }

  if (!initialised) {
    setupEventListeners();
    initialised = true;
    log.info('Trade expansion view initialised');
  } else {
    // Re-attach listeners in case DOM was refreshed
    setupEventListeners();
    log.debug('Trade expansion view re-activated');
  }

  renderPlaceholder();
}

export const tradeExpansion = {
  init,
  state,
};

export default tradeExpansion;
