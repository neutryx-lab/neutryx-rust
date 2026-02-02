/**
 * Portfolio View Module
 * Handles portfolio data loading and rendering
 */

import { createScopedLogger } from '@/utils/logger';
import { fetchPortfolio, fetchCounterparties } from '@/services/api';

const log = createScopedLogger('Portfolio');

// =============================================================================
// Types
// =============================================================================

interface PortfolioData {
  metadata: {
    description: string;
    last_updated: string;
    version: string;
  };
  portfolio: {
    total_pv: number;
    trade_count: number;
    trades: Trade[];
  };
}

interface Trade {
  id: string;
  type: string;
  notional: number;
  currency: string;
  pv: number;
  counterparty: string;
  maturity: string;
}

interface CounterpartyData {
  metadata: {
    description: string;
    last_updated: string;
    version: string;
  };
  counterparties: Counterparty[];
}

interface Counterparty {
  name: string;
  rating: string;
  exposure: number;
  limit: number;
  cva: number;
  utilization: number;
}

// =============================================================================
// State
// =============================================================================

interface PortfolioState {
  trades: Trade[];
  counterparties: Counterparty[];
  totalPv: number;
  tradeCount: number;
  isLoading: boolean;
}

const state: PortfolioState = {
  trades: [],
  counterparties: [],
  totalPv: 0,
  tradeCount: 0,
  isLoading: false,
};

let initialised = false;

// =============================================================================
// Utility Functions
// =============================================================================

function formatCurrency(value: number, _currency: string = 'USD'): string {
  const absValue = Math.abs(value);
  if (absValue >= 1_000_000) {
    return `$${(value / 1_000_000).toFixed(1)}M`;
  } else if (absValue >= 1_000) {
    return `$${(value / 1_000).toFixed(0)}K`;
  }
  return `$${value.toFixed(0)}`;
}

function formatNotional(value: number): string {
  if (value >= 1_000_000) {
    return `$${(value / 1_000_000).toFixed(0)}M`;
  } else if (value >= 1_000) {
    return `$${(value / 1_000).toFixed(0)}K`;
  }
  return `$${value.toFixed(0)}`;
}

function getRiskLevel(pv: number, notional: number): { level: string; class: string } {
  const ratio = Math.abs(pv) / notional;
  if (ratio > 0.05) {
    return { level: 'High', class: 'high' };
  } else if (ratio > 0.02) {
    return { level: 'Med', class: 'medium' };
  }
  return { level: 'Low', class: 'low' };
}

function getRatingClass(rating: string): string {
  if (rating.startsWith('AAA') || rating.startsWith('AA')) {
    return 'aa';
  } else if (rating.startsWith('A')) {
    return 'a';
  } else if (rating.startsWith('BBB')) {
    return 'bbb';
  }
  return 'other';
}

function getUtilisationStatus(utilization: number): { class: string; status: string } {
  if (utilization >= 80) {
    return { class: 'danger', status: 'Alert' };
  } else if (utilization >= 50) {
    return { class: 'warning', status: 'Watch' };
  }
  return { class: '', status: 'OK' };
}

function escapeHtml(text: string): string {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

// =============================================================================
// Data Loading
// =============================================================================

async function loadPortfolioData(): Promise<void> {
  state.isLoading = true;

  try {
    const [portfolioData, counterpartyData] = await Promise.all([
      fetchPortfolio() as Promise<PortfolioData>,
      fetchCounterparties() as Promise<CounterpartyData>,
    ]);

    state.trades = portfolioData.portfolio.trades;
    state.totalPv = portfolioData.portfolio.total_pv;
    state.tradeCount = portfolioData.portfolio.trade_count;
    state.counterparties = counterpartyData.counterparties;

    log.info('Portfolio data loaded', {
      trades: state.trades.length,
      counterparties: state.counterparties.length,
    });
  } catch (error) {
    log.error('Failed to load portfolio data', error);
    throw error;
  } finally {
    state.isLoading = false;
  }
}

// =============================================================================
// Rendering
// =============================================================================

function renderPortfolioTable(): void {
  const tbody = document.getElementById('portfolio-body');
  if (!tbody) {
    log.warn('Portfolio table body not found');
    return;
  }

  if (state.trades.length === 0) {
    tbody.innerHTML = `
      <tr>
        <td colspan="12" class="empty-state">
          <i class="fas fa-inbox"></i>
          <p>No trades in portfolio</p>
        </td>
      </tr>
    `;
    return;
  }

  const rows = state.trades.map((trade) => {
    const risk = getRiskLevel(trade.pv, trade.notional);
    const pvClass = trade.pv >= 0 ? 'positive' : 'negative';
    // Mock values for delta and vega - these would normally come from pricing
    const delta = (0.3 + Math.random() * 0.5).toFixed(2);
    const vega = formatCurrency(Math.abs(trade.pv) * 0.05);

    return `
      <tr data-trade-id="${escapeHtml(trade.id)}">
        <td><input type="checkbox" aria-label="Select trade ${escapeHtml(trade.id)}"></td>
        <td>${escapeHtml(trade.id)}</td>
        <td>${escapeHtml(trade.type)} ${escapeHtml(trade.currency)}</td>
        <td>${escapeHtml(trade.type)}</td>
        <td>${escapeHtml(trade.counterparty)}</td>
        <td>${escapeHtml(trade.maturity)}</td>
        <td>${formatNotional(trade.notional)}</td>
        <td class="${pvClass}">${formatCurrency(trade.pv)}</td>
        <td>${delta}</td>
        <td>${vega}</td>
        <td><span class="risk-badge ${risk.class}">${risk.level}</span></td>
        <td><button class="icon-btn small" aria-label="Trade actions"><i class="fas fa-ellipsis-v"></i></button></td>
      </tr>
    `;
  }).join('');

  tbody.innerHTML = rows;
  log.debug('Portfolio table rendered', { count: state.trades.length });
}

function renderCounterpartyTable(): void {
  const tbody = document.getElementById('counterparty-table-body');
  if (!tbody) {
    log.warn('Counterparty table body not found');
    return;
  }

  if (state.counterparties.length === 0) {
    tbody.innerHTML = `
      <tr>
        <td colspan="7" class="empty-state">
          <i class="fas fa-users"></i>
          <p>No counterparty data</p>
        </td>
      </tr>
    `;
    return;
  }

  const rows = state.counterparties.map((cp) => {
    const ratingClass = getRatingClass(cp.rating);
    const utilStatus = getUtilisationStatus(cp.utilization);
    const statusColorClass = utilStatus.status === 'Alert' ? 'red' :
                              utilStatus.status === 'Watch' ? 'yellow' : 'green';

    return `
      <tr>
        <td><span class="cp-name">${escapeHtml(cp.name)}</span></td>
        <td><span class="rating-badge ${ratingClass}">${escapeHtml(cp.rating)}</span></td>
        <td>${formatCurrency(cp.exposure)}</td>
        <td>${formatCurrency(cp.limit)}</td>
        <td>
          <div class="util-bar ${utilStatus.class}"><div class="util-fill" style="width: ${cp.utilization}%"></div></div>
          <span>${cp.utilization.toFixed(0)}%</span>
        </td>
        <td class="negative">-${formatCurrency(cp.cva)}</td>
        <td><span class="status-badge ${statusColorClass}">${utilStatus.status}</span></td>
      </tr>
    `;
  }).join('');

  tbody.innerHTML = rows;
  log.debug('Counterparty table rendered', { count: state.counterparties.length });
}

function updateSummaryStats(): void {
  // Update portfolio summary
  const totalPvEl = document.getElementById('portfolio-total-pv');
  const countEl = document.getElementById('portfolio-count');
  const totalItemsEl = document.getElementById('total-items');
  const showingEndEl = document.getElementById('showing-end');

  if (totalPvEl) {
    totalPvEl.textContent = formatCurrency(state.totalPv);
    totalPvEl.classList.toggle('positive', state.totalPv >= 0);
    totalPvEl.classList.toggle('negative', state.totalPv < 0);
  }
  if (countEl) countEl.textContent = state.tradeCount.toString();
  if (totalItemsEl) totalItemsEl.textContent = state.tradeCount.toString();
  if (showingEndEl) showingEndEl.textContent = Math.min(state.tradeCount, 50).toString();

  // Calculate average delta (mock)
  const avgDeltaEl = document.getElementById('portfolio-avg-delta');
  if (avgDeltaEl) {
    avgDeltaEl.textContent = (0.3 + Math.random() * 0.3).toFixed(2);
  }

  // Calculate total vega (mock based on total PV)
  const totalVegaEl = document.getElementById('portfolio-total-vega');
  if (totalVegaEl) {
    totalVegaEl.textContent = formatCurrency(state.totalPv * 0.1);
  }
}

function renderAll(): void {
  renderPortfolioTable();
  renderCounterpartyTable();
  updateSummaryStats();
}

// =============================================================================
// Public API
// =============================================================================

export async function init(): Promise<void> {
  if (initialised) {
    // Re-render when navigating back
    renderAll();
    return;
  }

  try {
    await loadPortfolioData();
    renderAll();
    initialised = true;
    log.info('Portfolio module initialised');
  } catch (error) {
    log.error('Failed to initialise portfolio module', error);
    // Show error state in UI
    const tbody = document.getElementById('portfolio-body');
    if (tbody) {
      tbody.innerHTML = `
        <tr>
          <td colspan="12" class="error-state">
            <i class="fas fa-exclamation-triangle"></i>
            <p>Failed to load portfolio data</p>
          </td>
        </tr>
      `;
    }
  }
}

export const portfolio = {
  init,
  state,
  renderAll,
  loadPortfolioData,
};

export default portfolio;