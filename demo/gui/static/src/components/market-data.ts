/**
 * Market Data Viewer Module
 * Handles display of market rates, conventions, IR/FX volatility, and events
 */

import type {
  MarketRate,
  MarketDataState,
  AssetClass,
  IrVolQuoteFlat,
  FxVolQuoteFlat,
} from '@/types';
import {
  fetchMarketRates,
  fetchMarketConfig,
  fetchRateDetail,
  refreshMarketRates,
  fetchConventions,
  fetchIrVolCurrencies,
  fetchIrVolQuotes,
  fetchFxVolPairs,
  fetchFxVolQuotes,
  fetchEvents,
  fetchEventTypes,
} from '@/services/api';
import { createScopedLogger } from '@/utils/logger';
import {
  formatRate,
  formatVol,
  formatVolBps,
  escapeHtml,
} from '@/utils/format';
import { getElementById, showToast } from '@/utils/dom';

const log = createScopedLogger('MarketDataViewer');

// =============================================================================
// State
// =============================================================================

const state: MarketDataState = {
  rates: [],
  filteredRates: [],
  selectedRateId: null,
  sortColumn: 'tenor',
  sortDirection: 'asc',
  lastUpdated: null,
  previousValues: new Map(),
  isInitialised: false,
  assetClass: 'Rates',
  allConventions: [],
  filteredConventions: [],
  selectedConventionId: null,
  irVolCurrencies: [],
  irVolQuotes: [],
  selectedIrVolCurrency: null,
  fxVolPairs: [],
  fxVolQuotes: [],
  selectedFxVolPair: null,
  events: [],
  filteredEvents: [],
  eventTypes: [],
  selectedEventId: null,
};

let elements: Record<string, HTMLElement | null> = {};
let TENOR_ORDER: Record<string, number> = {};

// =============================================================================
// Initialisation
// =============================================================================

function cacheElements(): void {
  elements = {
    assetClassToggle: getElementById('market-asset-class-toggle'),
    currencyFilter: getElementById('market-currency-filter'),
    refreshBtn: getElementById('market-refresh-btn'),
    exportBtn: getElementById('market-export-btn'),
    exportMenu: getElementById('market-export-menu'),
    ratesTable: getElementById('market-rates-table'),
    ratesTbody: getElementById('market-rates-tbody'),
    totalCount: getElementById('market-total-count'),
    staleCount: getElementById('market-stale-count'),
    lastUpdated: getElementById('market-last-updated'),
    placeholder: getElementById('market-placeholder'),
    loading: getElementById('market-loading'),
    detailPanel: getElementById('market-detail-panel'),
    detailContent: getElementById('market-detail-content'),
    closeDetailBtn: getElementById('close-detail-panel'),
    conventionsGrid: getElementById('conventions-grid'),
    statsTotal: getElementById('market-stats-total'),
    statsLive: getElementById('market-stats-live'),
    statsDisplayed: getElementById('market-stats-displayed'),
    conventionDetailPanel: getElementById('convention-detail-panel'),
    conventionDetailContent: getElementById('convention-detail-content'),
    closeConventionDetailBtn: getElementById('close-convention-detail'),
    conventionsCount: getElementById('conventions-count'),
  };
}

function bindEvents(): void {
  elements.assetClassToggle?.querySelectorAll('.asset-toggle-btn').forEach((btn) => {
    btn.addEventListener('click', () => {
      const assetClass = (btn as HTMLElement).dataset.asset as AssetClass;
      setAssetClass(assetClass);
    });
  });

  (elements.currencyFilter as HTMLSelectElement | null)?.addEventListener('change', applyFilters);
  elements.refreshBtn?.addEventListener('click', () => void refreshRates());

  document.querySelectorAll('.export-option').forEach((btn) => {
    btn.addEventListener('click', (e) => {
      const format = ((e.currentTarget as HTMLElement).dataset.format || 'csv') as 'csv' | 'json';
      void exportRates(format);
    });
  });

  elements.ratesTable?.querySelectorAll('th.sortable').forEach((th) => {
    th.addEventListener('click', () => {
      const column = (th as HTMLElement).dataset.sort || '';
      if (state.sortColumn === column) {
        state.sortDirection = state.sortDirection === 'asc' ? 'desc' : 'asc';
      } else {
        state.sortColumn = column;
        state.sortDirection = 'asc';
      }
      updateSortIndicators();
      sortAndRender();
    });
  });

  elements.closeDetailBtn?.addEventListener('click', () => {
    state.selectedRateId = null;
    renderDetailPanel();
    updateTableSelection();
  });
}

async function loadConfig(): Promise<void> {
  try {
    const config = await fetchMarketConfig();
    TENOR_ORDER = {};
    config.tenorOrder.forEach((tenor, index) => {
      TENOR_ORDER[tenor.toUpperCase()] = index;
    });
    log.info(`Market config loaded: ${Object.keys(TENOR_ORDER).length} tenors`);
  } catch (error) {
    log.error('Failed to load market config', error);
  }
}

// =============================================================================
// Data Loading
// =============================================================================

async function loadRates(): Promise<void> {
  showLoading(true);
  try {
    const data = await fetchMarketRates();
    state.previousValues.clear();
    state.rates.forEach((r) => state.previousValues.set(r.id, r.value));
    state.rates = data.rates || [];
    state.lastUpdated = data.lastUpdated;
    applyFilters();
    updateStats();
    updateLastUpdated();
    log.info(`Loaded ${state.rates.length} market rates`);
  } catch (error) {
    log.error('Failed to load market rates', error);
    showToast('Failed to load market data', 'error');
  } finally {
    showLoading(false);
  }
}

async function refreshRates(): Promise<void> {
  elements.refreshBtn?.classList.add('spinning');
  try {
    if (state.assetClass === 'IRVol') {
      state.irVolQuotes = [];
      await loadIrVolData();
      showToast('IR Vol data refreshed', 'success');
    } else if (state.assetClass === 'FXVol') {
      state.fxVolQuotes = [];
      await loadFxVolData();
      showToast('FX Vol data refreshed', 'success');
    } else if (state.assetClass === 'Events') {
      state.events = [];
      await loadEventsData();
      showToast('Events data refreshed', 'success');
    } else {
      await refreshMarketRates();
      await loadRates();
      showToast('Market data refreshed', 'success');
    }
  } catch (error) {
    log.error('Failed to refresh data', error);
    showToast('Failed to refresh data', 'error');
  } finally {
    elements.refreshBtn?.classList.remove('spinning');
  }
}

async function loadConventions(): Promise<void> {
  try {
    const data = await fetchConventions();
    state.allConventions = data.conventions || [];
    filterAndRenderConventions();
  } catch (error) {
    log.error('Failed to load conventions', error);
  }
}

async function loadIrVolData(): Promise<void> {
  showLoading(true);
  try {
    const currenciesData = await fetchIrVolCurrencies();
    state.irVolCurrencies = currenciesData.currencies || [];

    const allQuotes: IrVolQuoteFlat[] = [];
    for (const currency of state.irVolCurrencies) {
      try {
        const quotesData = await fetchIrVolQuotes(currency.currency);
        for (const quote of quotesData.quotes || []) {
          allQuotes.push({
            id: `${currency.currency}-${quote.expiry}-${quote.tenor}`,
            currency: currency.currency,
            expiry: quote.expiry,
            tenor: quote.tenor,
            atmVol: quote.atmVol,
            volType: quotesData.volType || 'Normal',
            smile: quote.smile || [],
            source: quotesData.source || 'Demo',
          });
        }
      } catch (e) {
        log.error(`Failed to load quotes for ${currency.currency}`, e);
      }
    }

    state.irVolQuotes = allQuotes;
    state.lastUpdated = new Date().toISOString();
    renderIrVolTable();
    updateIrVolStats();
    updateLastUpdated();
    log.info(`Loaded ${allQuotes.length} IR vol quotes`);
  } catch (error) {
    log.error('Failed to load IR vol data', error);
    showToast('Failed to load IR volatility data', 'error');
  } finally {
    showLoading(false);
  }
}

async function loadFxVolData(): Promise<void> {
  showLoading(true);
  try {
    const pairsData = await fetchFxVolPairs();
    state.fxVolPairs = pairsData.pairs || [];

    const allQuotes: FxVolQuoteFlat[] = [];
    for (const pairInfo of state.fxVolPairs) {
      try {
        const quotesData = await fetchFxVolQuotes(pairInfo.pair);
        for (const quote of quotesData.quotes || []) {
          allQuotes.push({
            id: `${pairInfo.pair}-${quote.expiry}`,
            pair: pairInfo.pair,
            expiry: quote.expiry,
            expiryLabel: expiryToLabel(quote.expiry),
            atmVol: quote.atmVol,
            rr25d: quote.rr25d,
            bf25d: quote.bf25d,
            rr10d: quote.rr10d,
            bf10d: quote.bf10d,
            spot: quotesData.spot,
            source: 'Demo',
          });
        }
      } catch (e) {
        log.error(`Failed to load quotes for ${pairInfo.pair}`, e);
      }
    }

    state.fxVolQuotes = allQuotes;
    state.lastUpdated = new Date().toISOString();
    renderFxVolTable();
    updateFxVolStats();
    updateLastUpdated();
    log.info(`Loaded ${allQuotes.length} FX vol quotes`);
  } catch (error) {
    log.error('Failed to load FX vol data', error);
    showToast('Failed to load FX volatility data', 'error');
  } finally {
    showLoading(false);
  }
}

async function loadEventsData(): Promise<void> {
  showLoading(true);
  try {
    const typesData = await fetchEventTypes();
    state.eventTypes = typesData.types || [];

    const eventsData = await fetchEvents();
    state.events = eventsData.events || [];
    state.filteredEvents = [...state.events];
    state.lastUpdated = new Date().toISOString();
    renderEventsTable();
    updateEventsStats();
    updateLastUpdated();
    log.info(`Loaded ${state.events.length} events`);
  } catch (error) {
    log.error('Failed to load events data', error);
    showToast('Failed to load events data', 'error');
  } finally {
    showLoading(false);
  }
}

// =============================================================================
// Helper Functions
// =============================================================================

function expiryToLabel(expiry: number): string {
  if (expiry < 0.05) return '1W';
  if (expiry < 0.125) return '1M';
  if (expiry < 0.21) return '2M';
  if (expiry < 0.33) return '3M';
  if (expiry < 0.54) return '6M';
  if (expiry < 0.83) return '9M';
  if (expiry < 1.5) return '1Y';
  if (expiry < 2.5) return '2Y';
  if (expiry < 4.0) return '3Y';
  return `${Math.round(expiry)}Y`;
}

function compareTenor(a: string, b: string): number {
  const aUpper = String(a).toUpperCase();
  const bUpper = String(b).toUpperCase();

  const aOrder = TENOR_ORDER[aUpper];
  const bOrder = TENOR_ORDER[bUpper];

  if (aOrder !== undefined && bOrder !== undefined) {
    return aOrder - bOrder;
  }
  if (aOrder === undefined && bOrder !== undefined) return 1;
  if (aOrder !== undefined && bOrder === undefined) return -1;
  return aUpper.localeCompare(bUpper);
}

// =============================================================================
// Rendering Functions
// =============================================================================

function showLoading(show: boolean): void {
  if (elements.loading) {
    elements.loading.style.display = show ? 'flex' : 'none';
  }
  if (elements.ratesTbody) {
    elements.ratesTbody.style.display = show ? 'none' : '';
  }
}

function updateStats(): void {
  const totalAll = state.rates.length;
  const liveAll = state.rates.filter((r) => !r.isStale).length;
  const displayed = state.filteredRates.length;
  const stale = state.filteredRates.filter((r) => r.isStale).length;

  if (elements.statsTotal) elements.statsTotal.textContent = String(totalAll);
  if (elements.statsLive) elements.statsLive.textContent = String(liveAll);
  if (elements.statsDisplayed) elements.statsDisplayed.textContent = String(displayed);
  if (elements.totalCount) elements.totalCount.textContent = String(displayed);
  if (elements.staleCount) elements.staleCount.textContent = String(stale);
}

function updateLastUpdated(): void {
  if (!elements.lastUpdated || !state.lastUpdated) return;
  const date = new Date(state.lastUpdated);
  elements.lastUpdated.textContent = date.toLocaleTimeString();
}

function updateSortIndicators(): void {
  elements.ratesTable?.querySelectorAll('th.sortable').forEach((th) => {
    th.classList.remove('sorted-asc', 'sorted-desc');
    if ((th as HTMLElement).dataset.sort === state.sortColumn) {
      th.classList.add(`sorted-${state.sortDirection}`);
    }
  });
}

function updateTableSelection(): void {
  elements.ratesTbody?.querySelectorAll('tr').forEach((row) => {
    row.classList.toggle('selected', (row as HTMLElement).dataset.rateId === state.selectedRateId);
  });
}

function setAssetClass(assetClass: AssetClass): void {
  state.assetClass = assetClass;
  state.selectedRateId = null;

  if (assetClass === 'FXVol') {
    state.sortColumn = 'expiry';
  } else if (assetClass === 'Events') {
    state.sortColumn = 'date';
  } else {
    state.sortColumn = 'tenor';
  }
  state.sortDirection = 'asc';

  updateAssetClassToggle();

  if (assetClass === 'IRVol') {
    if (state.irVolQuotes.length === 0) {
      void loadIrVolData();
    } else {
      renderIrVolTable();
      updateIrVolStats();
    }
  } else if (assetClass === 'FXVol') {
    if (state.fxVolQuotes.length === 0) {
      void loadFxVolData();
    } else {
      renderFxVolTable();
      updateFxVolStats();
    }
  } else if (assetClass === 'Events') {
    if (state.events.length === 0) {
      void loadEventsData();
    } else {
      renderEventsTable();
      updateEventsStats();
    }
  } else {
    applyFilters();
  }

  filterAndRenderConventions();
  renderDetailPanel();
  log.info(`Asset class changed to: ${assetClass}`);
}

function updateAssetClassToggle(): void {
  elements.assetClassToggle?.querySelectorAll('.asset-toggle-btn').forEach((btn) => {
    btn.classList.toggle('active', (btn as HTMLElement).dataset.asset === state.assetClass);
  });
}

function applyFilters(): void {
  if (state.assetClass === 'IRVol') {
    renderIrVolTable();
    updateIrVolStats();
    return;
  }
  if (state.assetClass === 'FXVol') {
    renderFxVolTable();
    updateFxVolStats();
    return;
  }
  if (state.assetClass === 'Events') {
    renderEventsTable();
    updateEventsStats();
    return;
  }

  const currency = ((elements.currencyFilter as HTMLSelectElement | null)?.value || '').toLowerCase();
  const assetClassTypes = getAssetClassRateTypes(state.assetClass);

  state.filteredRates = state.rates.filter((rate) => {
    const rateTypeLower = rate.rateType?.toLowerCase() || '';
    if (assetClassTypes.length > 0 && !assetClassTypes.includes(rateTypeLower)) {
      return false;
    }
    if (currency && rate.currency.toLowerCase() !== currency) return false;
    return true;
  });

  sortAndRender();
  updateStats();
}

function getAssetClassRateTypes(assetClass: AssetClass): string[] {
  switch (assetClass) {
    case 'Rates':
      return ['deposit', 'swap', 'ois', 'fra', 'future', 'xccybasis'];
    case 'FX':
      return ['fxspot', 'fxforward'];
    default:
      return [];
  }
}

function sortAndRender(): void {
  const col = state.sortColumn;
  const dir = state.sortDirection === 'asc' ? 1 : -1;

  state.filteredRates.sort((a, b) => {
    const currencyCompare = String(a.currency || '').localeCompare(String(b.currency || ''));
    if (currencyCompare !== 0) {
      return currencyCompare;
    }

    const aVal = a[col as keyof MarketRate];
    const bVal = b[col as keyof MarketRate];

    if (col === 'value') {
      return (Number(aVal) - Number(bVal)) * dir;
    }
    if (col === 'tenor') {
      return compareTenor(String(aVal), String(bVal)) * dir;
    }
    return String(aVal || '').localeCompare(String(bVal || '')) * dir;
  });

  renderTable();
}

function renderTable(): void {
  if (!elements.ratesTbody) return;

  if (state.filteredRates.length === 0) {
    elements.ratesTbody.innerHTML = '';
    if (elements.placeholder) {
      elements.placeholder.style.display = 'flex';
      elements.placeholder.innerHTML = `
        <i class="fas fa-search"></i>
        <p>No rates match the current filters</p>
      `;
    }
    return;
  }

  if (elements.placeholder) elements.placeholder.style.display = 'none';

  const html = state.filteredRates
    .map((rate) => {
      const prevValue = state.previousValues.get(rate.id);
      let highlightClass = '';
      if (prevValue !== undefined && prevValue !== rate.value) {
        highlightClass = rate.value > prevValue ? 'highlight-up' : 'highlight-down';
      }

      const valueClass = rate.value >= 0 ? '' : 'negative';
      const statusClass = rate.isStale ? 'stale' : 'fresh';
      const statusIcon = rate.isStale ? 'fa-clock' : 'fa-check';
      const statusText = rate.isStale ? 'Stale' : 'Live';

      return `
        <tr data-rate-id="${escapeHtml(rate.id)}" class="${state.selectedRateId === rate.id ? 'selected' : ''}">
          <td>${escapeHtml(rate.id)}</td>
          <td>${escapeHtml(rate.currency)}</td>
          <td>${escapeHtml(rate.tenor)}</td>
          <td>${escapeHtml(rate.rateType)}</td>
          <td class="numeric">
            <span class="rate-value ${valueClass} ${highlightClass}">
              ${formatRate(rate.value, rate.rateType)}
            </span>
          </td>
          <td>${rate.rateIndex ? `<span class="index-badge">${escapeHtml(rate.rateIndex)}</span>` : '-'}</td>
          <td>${escapeHtml(rate.source)}</td>
          <td>
            <span class="status-indicator ${statusClass}">
              <i class="fas ${statusIcon}"></i>
              ${statusText}
            </span>
          </td>
        </tr>
      `;
    })
    .join('');

  elements.ratesTbody.innerHTML = html;

  elements.ratesTbody.querySelectorAll('tr').forEach((row) => {
    row.addEventListener('click', () => {
      const rateId = (row as HTMLElement).dataset.rateId || '';
      selectRate(rateId);
    });
  });
}

function renderIrVolTable(): void {
  // Implementation similar to renderTable but for IR Vol quotes
  if (!elements.ratesTbody) return;

  const currency = ((elements.currencyFilter as HTMLSelectElement | null)?.value || '').toUpperCase();
  let filteredQuotes = state.irVolQuotes;
  if (currency) {
    filteredQuotes = filteredQuotes.filter((q) => q.currency.toUpperCase() === currency);
  }

  if (filteredQuotes.length === 0) {
    elements.ratesTbody.innerHTML = '';
    if (elements.placeholder) {
      elements.placeholder.style.display = 'flex';
      elements.placeholder.innerHTML = `
        <i class="fas fa-chart-area"></i>
        <p>No IR volatility data available</p>
      `;
    }
    return;
  }

  if (elements.placeholder) elements.placeholder.style.display = 'none';

  const html = filteredQuotes
    .map(
      (quote) => `
      <tr data-quote-id="${escapeHtml(quote.id)}">
        <td>${escapeHtml(quote.currency)}</td>
        <td>${escapeHtml(quote.expiry)}</td>
        <td>${escapeHtml(quote.tenor)}</td>
        <td class="numeric"><span class="rate-value">${formatVol(quote.atmVol)}</span></td>
        <td>${escapeHtml(quote.volType)}</td>
        <td>${quote.smile.length} points</td>
        <td>${escapeHtml(quote.source)}</td>
        <td><span class="status-indicator fresh"><i class="fas fa-check"></i> Live</span></td>
      </tr>
    `
    )
    .join('');

  elements.ratesTbody.innerHTML = html;
}

function renderFxVolTable(): void {
  if (!elements.ratesTbody) return;

  const pairFilter = ((elements.currencyFilter as HTMLSelectElement | null)?.value || '').toUpperCase();
  let filteredQuotes = state.fxVolQuotes;
  if (pairFilter) {
    filteredQuotes = filteredQuotes.filter((q) => q.pair.toUpperCase().includes(pairFilter));
  }

  if (filteredQuotes.length === 0) {
    elements.ratesTbody.innerHTML = '';
    if (elements.placeholder) {
      elements.placeholder.style.display = 'flex';
      elements.placeholder.innerHTML = `
        <i class="fas fa-chart-area"></i>
        <p>No FX volatility data available</p>
      `;
    }
    return;
  }

  if (elements.placeholder) elements.placeholder.style.display = 'none';

  const html = filteredQuotes
    .map(
      (quote) => `
      <tr data-quote-id="${escapeHtml(quote.id)}">
        <td>${escapeHtml(quote.pair)}</td>
        <td>${escapeHtml(quote.expiryLabel)}</td>
        <td class="numeric"><span class="rate-value">${formatVol(quote.atmVol)}</span></td>
        <td class="numeric ${(quote.rr25d ?? 0) >= 0 ? '' : 'negative'}">${formatVolBps(quote.rr25d)}</td>
        <td class="numeric">${formatVolBps(quote.bf25d)}</td>
        <td class="numeric ${(quote.rr10d ?? 0) >= 0 ? '' : 'negative'}">${formatVolBps(quote.rr10d)}</td>
        <td class="numeric">${formatVolBps(quote.bf10d)}</td>
        <td><span class="status-indicator fresh"><i class="fas fa-check"></i> Live</span></td>
      </tr>
    `
    )
    .join('');

  elements.ratesTbody.innerHTML = html;
}

function renderEventsTable(): void {
  if (!elements.ratesTbody) return;

  const currency = ((elements.currencyFilter as HTMLSelectElement | null)?.value || '').toUpperCase();
  let filteredEvents = [...state.events];
  if (currency) {
    filteredEvents = filteredEvents.filter((e) => e.currency?.toUpperCase() === currency);
  }

  state.filteredEvents = filteredEvents;

  if (state.filteredEvents.length === 0) {
    elements.ratesTbody.innerHTML = '';
    if (elements.placeholder) {
      elements.placeholder.style.display = 'flex';
      elements.placeholder.innerHTML = `
        <i class="fas fa-calendar-alt"></i>
        <p>No events available</p>
      `;
    }
    return;
  }

  if (elements.placeholder) elements.placeholder.style.display = 'none';

  const html = state.filteredEvents
    .map(
      (event) => `
      <tr data-event-id="${escapeHtml(event.id)}">
        <td><span class="event-date">${escapeHtml(event.date)}</span></td>
        <td><span class="event-type-badge ${event.eventType}">${escapeHtml(event.eventType)}</span></td>
        <td><span class="event-title">${escapeHtml(event.title)}</span></td>
        <td>${event.currency ? escapeHtml(event.currency) : '-'}</td>
        <td>${event.region ? escapeHtml(event.region) : '-'}</td>
        <td><span class="importance-badge">${escapeHtml(event.importance)}</span></td>
        <td>${event.time ? escapeHtml(event.time) : '-'}</td>
        <td>${escapeHtml(event.source)}</td>
      </tr>
    `
    )
    .join('');

  elements.ratesTbody.innerHTML = html;
}

function updateIrVolStats(): void {
  const total = state.irVolQuotes.length;
  const currencies = new Set(state.irVolQuotes.map((q) => q.currency)).size;
  if (elements.statsTotal) elements.statsTotal.textContent = String(total);
  if (elements.statsLive) elements.statsLive.textContent = String(currencies);
  if (elements.statsDisplayed) elements.statsDisplayed.textContent = String(total);
}

function updateFxVolStats(): void {
  const total = state.fxVolQuotes.length;
  const pairs = new Set(state.fxVolQuotes.map((q) => q.pair)).size;
  if (elements.statsTotal) elements.statsTotal.textContent = String(total);
  if (elements.statsLive) elements.statsLive.textContent = String(pairs);
  if (elements.statsDisplayed) elements.statsDisplayed.textContent = String(total);
}

function updateEventsStats(): void {
  const total = state.events.length;
  const displayed = state.filteredEvents.length;
  if (elements.statsTotal) elements.statsTotal.textContent = String(total);
  if (elements.statsDisplayed) elements.statsDisplayed.textContent = String(displayed);
}

function filterAndRenderConventions(): void {
  // Implementation for filtering conventions by asset class
  state.filteredConventions = state.allConventions;
  renderConventions();
}

function renderConventions(): void {
  if (!elements.conventionsGrid) return;
  // Simplified convention rendering
  if (state.filteredConventions.length === 0) {
    elements.conventionsGrid.innerHTML = `<div class="conventions-empty"><p>No conventions</p></div>`;
    return;
  }
  const html = state.filteredConventions
    .map(
      (conv) => `
      <div class="convention-card" data-convention-id="${escapeHtml(conv.id)}">
        <span class="convention-id">${escapeHtml(conv.id)}</span>
        <span class="convention-type-badge">${escapeHtml(conv.conventionType)}</span>
      </div>
    `
    )
    .join('');
  elements.conventionsGrid.innerHTML = html;
}

function selectRate(rateId: string): void {
  state.selectedRateId = rateId;
  updateTableSelection();
  renderDetailPanel();
}

function renderDetailPanel(): void {
  if (!state.selectedRateId) {
    if (elements.detailContent) {
      elements.detailContent.innerHTML = `
        <div class="detail-placeholder">
          <i class="fas fa-hand-pointer"></i>
          <p>Select a rate to view details</p>
        </div>
      `;
    }
    return;
  }
  void loadRateDetailAsync(state.selectedRateId);
}

async function loadRateDetailAsync(rateId: string): Promise<void> {
  try {
    const detail = await fetchRateDetail(rateId);
    if (elements.detailContent) {
      elements.detailContent.innerHTML = `
        <div class="detail-section">
          <div class="detail-row">
            <span class="detail-label">ID</span>
            <span class="detail-value">${escapeHtml(detail.rate.id)}</span>
          </div>
          <div class="detail-row">
            <span class="detail-label">Value</span>
            <span class="detail-value">${formatRate(detail.rate.value, detail.rate.rateType)}</span>
          </div>
          <div class="detail-row">
            <span class="detail-label">Currency</span>
            <span class="detail-value">${escapeHtml(detail.rate.currency)}</span>
          </div>
        </div>
      `;
    }
  } catch (error) {
    log.error('Failed to load rate detail', error);
  }
}

async function exportRates(format: 'csv' | 'json'): Promise<void> {
  try {
    const response = await fetch(`/api/market/export/${format}`);
    if (!response.ok) throw new Error('Export failed');
    const blob = await response.blob();
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `market_rates.${format}`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    showToast(`Exported as ${format.toUpperCase()}`, 'success');
  } catch (error) {
    log.error('Export failed', error);
    showToast('Export failed', 'error');
  }
}

// =============================================================================
// Public API
// =============================================================================

export async function init(): Promise<void> {
  if (state.isInitialised) {
    void loadRates();
    return;
  }

  cacheElements();
  bindEvents();
  await loadConfig();
  void loadRates();
  void loadConventions();
  state.isInitialised = true;
  log.info('Market Data Viewer initialised');
}

export const marketDataViewer = {
  init,
  refresh: refreshRates,
  getRates: () => [...state.rates],
  getIrVolQuotes: () => [...state.irVolQuotes],
  getFxVolQuotes: () => [...state.fxVolQuotes],
  getState: () => ({ ...state }),
};

export default marketDataViewer;
