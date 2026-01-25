// ===========================================
// Market Data Viewer Module (market-data-viewer-webapp)
// ===========================================

const marketDataViewer = (() => {
    // State
    const state = {
        rates: [],
        filteredRates: [],
        selectedRateId: null,
        selectedConventionId: null,
        sortColumn: 'id',
        sortDirection: 'asc',
        lastUpdated: null,
        previousValues: new Map(), // For change highlighting
        isInitialised: false,
        assetClass: 'Rates', // Default asset class filter
        allConventions: [], // All conventions (unfiltered)
        filteredConventions: [] // Filtered conventions for current asset class
    };

    // DOM Elements (cached)
    let elements = {};

    // ===========================================
    // Initialisation
    // ===========================================

    function init() {
        if (state.isInitialised) {
            loadRates();
            return;
        }

        cacheElements();
        bindEvents();
        loadRates();
        loadConventions();
        state.isInitialised = true;

        log('Market Data Viewer initialised');
    }

    function cacheElements() {
        elements = {
            assetClassToggle: document.getElementById('market-asset-class-toggle'),
            currencyFilter: document.getElementById('market-currency-filter'),
            typeFilter: document.getElementById('market-type-filter'),
            indexFilter: document.getElementById('market-index-filter'),
            refreshBtn: document.getElementById('market-refresh-btn'),
            exportBtn: document.getElementById('market-export-btn'),
            exportMenu: document.getElementById('market-export-menu'),
            ratesTable: document.getElementById('market-rates-table'),
            ratesTbody: document.getElementById('market-rates-tbody'),
            totalCount: document.getElementById('market-total-count'),
            staleCount: document.getElementById('market-stale-count'),
            lastUpdated: document.getElementById('market-last-updated'),
            placeholder: document.getElementById('market-placeholder'),
            loading: document.getElementById('market-loading'),
            detailPanel: document.getElementById('market-detail-panel'),
            detailContent: document.getElementById('market-detail-content'),
            closeDetailBtn: document.getElementById('close-detail-panel'),
            conventionsGrid: document.getElementById('conventions-grid'),
            // New stats badges
            statsTotal: document.getElementById('market-stats-total'),
            statsLive: document.getElementById('market-stats-live'),
            statsDisplayed: document.getElementById('market-stats-displayed'),
            // Convention detail panel
            conventionDetailPanel: document.getElementById('convention-detail-panel'),
            conventionDetailContent: document.getElementById('convention-detail-content'),
            closeConventionDetailBtn: document.getElementById('close-convention-detail'),
            conventionsCount: document.getElementById('conventions-count')
        };
    }

    function bindEvents() {
        // Asset Class Toggle
        elements.assetClassToggle?.querySelectorAll('.asset-toggle-btn').forEach(btn => {
            btn.addEventListener('click', () => {
                const assetClass = btn.dataset.asset;
                setAssetClass(assetClass);
            });
        });

        // Filters
        elements.currencyFilter?.addEventListener('change', applyFilters);
        elements.typeFilter?.addEventListener('change', applyFilters);
        elements.indexFilter?.addEventListener('change', applyFilters);

        // Refresh button
        elements.refreshBtn?.addEventListener('click', refreshRates);

        // Export buttons
        document.querySelectorAll('.export-option').forEach(btn => {
            btn.addEventListener('click', (e) => {
                const format = e.currentTarget.dataset.format;
                exportRates(format);
            });
        });

        // Table header sorting
        elements.ratesTable?.querySelectorAll('th.sortable').forEach(th => {
            th.addEventListener('click', () => {
                const column = th.dataset.sort;
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

        // Close detail panel
        elements.closeDetailBtn?.addEventListener('click', () => {
            state.selectedRateId = null;
            renderDetailPanel();
            updateTableSelection();
        });

        // Close convention detail panel
        elements.closeConventionDetailBtn?.addEventListener('click', () => {
            state.selectedConventionId = null;
            renderConventionDetailPanel();
            updateConventionSelection();
        });
    }

    // ===========================================
    // Data Loading
    // ===========================================

    async function loadRates() {
        showLoading(true);
        try {
            const query = buildQueryParams();
            const response = await fetch(`/api/market/rates${query}`);
            if (!response.ok) throw new Error('Failed to fetch rates');

            const data = await response.json();

            // Store previous values for change highlighting
            state.previousValues.clear();
            state.rates.forEach(r => state.previousValues.set(r.id, r.value));

            state.rates = data.rates || [];
            state.lastUpdated = data.lastUpdated;

            applyFilters();
            updateStats();
            updateLastUpdated();

            log(`Loaded ${state.rates.length} market rates`);
        } catch (error) {
            logError('Failed to load market rates:', error);
            showError('Failed to load market data');
        } finally {
            showLoading(false);
        }
    }

    async function refreshRates() {
        elements.refreshBtn?.classList.add('spinning');
        try {
            const response = await fetch('/api/market/rates/refresh', { method: 'POST' });
            if (!response.ok) throw new Error('Failed to refresh');

            await loadRates();
            showToast('Market data refreshed', 'success');
        } catch (error) {
            logError('Failed to refresh rates:', error);
            showToast('Failed to refresh data', 'error');
        } finally {
            elements.refreshBtn?.classList.remove('spinning');
        }
    }

    async function loadRateDetail(rateId) {
        try {
            const response = await fetch(`/api/market/rates/${encodeURIComponent(rateId)}`);
            if (!response.ok) throw new Error('Not found');

            const detail = await response.json();
            renderDetailContent(detail);
        } catch (error) {
            logError('Failed to load rate detail:', error);
            elements.detailContent.innerHTML = `
                <div class="detail-placeholder">
                    <i class="fas fa-exclamation-circle"></i>
                    <p>Failed to load rate detail</p>
                </div>
            `;
        }
    }

    async function loadConventions() {
        try {
            const response = await fetch('/api/market/conventions');
            if (!response.ok) throw new Error('Failed to fetch conventions');

            const data = await response.json();
            state.allConventions = data.conventions || [];
            filterAndRenderConventions();
        } catch (error) {
            logError('Failed to load conventions:', error);
        }
    }

    /**
     * Filters and renders conventions based on current asset class.
     */
    function filterAndRenderConventions() {
        if (!state.allConventions) return;

        const assetClass = state.assetClass;
        const conventionTypes = getAssetClassConventionTypes(assetClass);

        state.filteredConventions = state.allConventions.filter(conv => {
            if (conventionTypes.length === 0) return true;
            const convType = conv.conventionType?.toLowerCase() || '';
            return conventionTypes.some(t => convType.includes(t));
        });

        renderConventions(state.filteredConventions);
        updateConventionsCount();

        // Clear selection if selected convention is no longer visible
        if (state.selectedConventionId) {
            const stillVisible = state.filteredConventions.some(c => c.id === state.selectedConventionId);
            if (!stillVisible) {
                state.selectedConventionId = null;
                renderConventionDetailPanel();
            }
        }
    }

    /**
     * Maps asset class to convention type patterns.
     */
    function getAssetClassConventionTypes(assetClass) {
        switch (assetClass) {
            case 'Rates':
                return ['ois', 'swap', 'deposit', 'depo', 'xccy', 'basis'];
            case 'FX':
                return ['fx', 'spot', 'forward'];
            case 'IRVol':
                return ['swaption', 'cap', 'floor', 'irvol'];
            case 'FXVol':
                return ['fxvol', 'fxoption'];
            default:
                return [];
        }
    }

    // ===========================================
    // Asset Class Selection
    // ===========================================

    function setAssetClass(assetClass) {
        state.assetClass = assetClass;
        updateAssetClassToggle();
        applyFilters();
        filterAndRenderConventions();
        log(`Asset class changed to: ${assetClass}`);
    }

    function updateAssetClassToggle() {
        elements.assetClassToggle?.querySelectorAll('.asset-toggle-btn').forEach(btn => {
            btn.classList.toggle('active', btn.dataset.asset === state.assetClass);
        });
    }

    /**
     * Maps asset class to rateType values.
     * Rates: Deposit, Swap, OIS, XccyBasis
     * FX: FxSpot, FxForward
     * IRVol: Uses dedicated API /api/irvol/*
     * FXVol: Uses dedicated API /api/fxvol/*
     */
    function getAssetClassRateTypes(assetClass) {
        switch (assetClass) {
            case 'Rates':
                return ['deposit', 'swap', 'ois', 'xccybasis'];
            case 'FX':
                return ['fxspot', 'fxforward'];
            case 'IRVol':
                return []; // IRVol uses dedicated API
            case 'FXVol':
                return []; // FXVol uses dedicated API
            default:
                return [];
        }
    }

    // ===========================================
    // Filtering and Sorting
    // ===========================================

    function buildQueryParams() {
        const params = new URLSearchParams();
        const currency = elements.currencyFilter?.value;
        const rateType = elements.typeFilter?.value;
        const index = elements.indexFilter?.value;

        if (currency) params.set('currency', currency);
        if (rateType) params.set('rateType', rateType);
        if (index) params.set('index', index);

        return params.toString() ? `?${params.toString()}` : '';
    }

    function applyFilters() {
        const currency = elements.currencyFilter?.value?.toLowerCase() || '';
        const rateType = elements.typeFilter?.value?.toLowerCase() || '';
        const index = elements.indexFilter?.value?.toLowerCase() || '';
        const assetClassTypes = getAssetClassRateTypes(state.assetClass);

        state.filteredRates = state.rates.filter(rate => {
            // Filter by asset class (always applied)
            const rateTypeLower = rate.rateType?.toLowerCase() || '';
            if (assetClassTypes.length > 0 && !assetClassTypes.includes(rateTypeLower)) {
                return false;
            }

            // Additional filters
            if (currency && rate.currency.toLowerCase() !== currency) return false;
            if (rateType && rateTypeLower !== rateType) return false;
            if (index && (!rate.rateIndex || rate.rateIndex.toLowerCase() !== index)) return false;
            return true;
        });

        sortAndRender();
        updateStats();
    }

    function sortAndRender() {
        const col = state.sortColumn;
        const dir = state.sortDirection === 'asc' ? 1 : -1;

        state.filteredRates.sort((a, b) => {
            let aVal = a[col];
            let bVal = b[col];

            // Handle null/undefined
            if (aVal == null) aVal = '';
            if (bVal == null) bVal = '';

            // Numeric comparison for value
            if (col === 'value') {
                return (parseFloat(aVal) - parseFloat(bVal)) * dir;
            }

            // String comparison
            return String(aVal).localeCompare(String(bVal)) * dir;
        });

        renderTable();
    }

    function updateSortIndicators() {
        elements.ratesTable?.querySelectorAll('th.sortable').forEach(th => {
            th.classList.remove('sorted-asc', 'sorted-desc');
            if (th.dataset.sort === state.sortColumn) {
                th.classList.add(`sorted-${state.sortDirection}`);
            }
        });
    }

    // ===========================================
    // Rendering
    // ===========================================

    function renderTable() {
        if (!elements.ratesTbody) return;

        if (state.filteredRates.length === 0) {
            elements.ratesTbody.innerHTML = '';
            if (elements.placeholder) {
                elements.placeholder.style.display = 'flex';
                // Show asset-class specific message
                const assetClass = state.assetClass;
                if (assetClass === 'IRVol') {
                    elements.placeholder.innerHTML = `
                        <i class="fas fa-chart-area"></i>
                        <p>IR Volatility Surface</p>
                        <span class="placeholder-hint">View Swaption volatilities in the VolCube Calibration screen</span>
                        <a href="#" onclick="navigateTo('volcube-calibration-view'); return false;" class="vol-link">Open VolCube Calibration</a>
                    `;
                } else if (assetClass === 'FXVol') {
                    elements.placeholder.innerHTML = `
                        <i class="fas fa-chart-area"></i>
                        <p>FX Volatility Surface</p>
                        <span class="placeholder-hint">View FX volatility smile in the VolCube Calibration screen</span>
                        <a href="#" onclick="navigateTo('volcube-calibration-view'); return false;" class="vol-link">Open VolCube Calibration</a>
                    `;
                } else {
                    elements.placeholder.innerHTML = `
                        <i class="fas fa-search"></i>
                        <p>No rates match the current filters</p>
                    `;
                }
            }
            return;
        }

        if (elements.placeholder) elements.placeholder.style.display = 'none';

        const html = state.filteredRates.map(rate => {
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
                <tr data-rate-id="${rate.id}" class="${state.selectedRateId === rate.id ? 'selected' : ''}">
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
        }).join('');

        elements.ratesTbody.innerHTML = html;

        // Bind row click events
        elements.ratesTbody.querySelectorAll('tr').forEach(row => {
            row.addEventListener('click', () => {
                const rateId = row.dataset.rateId;
                selectRate(rateId);
            });
        });
    }

    function renderDetailPanel() {
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

        loadRateDetail(state.selectedRateId);
    }

    function renderDetailContent(detail) {
        const { rate, instrument, convention } = detail;

        let html = `
            <div class="detail-section">
                <div class="detail-section-title"><i class="fas fa-tag"></i> Rate Information</div>
                <div class="detail-row">
                    <span class="detail-label">ID</span>
                    <span class="detail-value">${escapeHtml(rate.id)}</span>
                </div>
                <div class="detail-row">
                    <span class="detail-label">Value</span>
                    <span class="detail-value large">${formatRate(rate.value, rate.rateType)}</span>
                </div>
                <div class="detail-row">
                    <span class="detail-label">Currency</span>
                    <span class="detail-value">${escapeHtml(rate.currency)}</span>
                </div>
                <div class="detail-row">
                    <span class="detail-label">Tenor</span>
                    <span class="detail-value">${escapeHtml(rate.tenor)}</span>
                </div>
                <div class="detail-row">
                    <span class="detail-label">Type</span>
                    <span class="detail-value">${escapeHtml(rate.rateType)}</span>
                </div>
                ${rate.rateIndex ? `
                <div class="detail-row">
                    <span class="detail-label">Index</span>
                    <span class="detail-value">${escapeHtml(rate.rateIndex)}</span>
                </div>
                ` : ''}
                <div class="detail-row">
                    <span class="detail-label">Quote Type</span>
                    <span class="detail-value">${escapeHtml(rate.quoteType)}</span>
                </div>
                <div class="detail-row">
                    <span class="detail-label">Source</span>
                    <span class="detail-value">${escapeHtml(rate.source)}</span>
                </div>
                <div class="detail-row">
                    <span class="detail-label">Timestamp</span>
                    <span class="detail-value">${formatTimestamp(rate.timestamp)}</span>
                </div>
            </div>
        `;

        if (instrument) {
            html += `
                <div class="detail-section">
                    <div class="detail-section-title"><i class="fas fa-puzzle-piece"></i> Instrument</div>
                    <div class="detail-row">
                        <span class="detail-label">Type</span>
                        <span class="detail-value">${escapeHtml(instrument.instrumentType)}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">Start Date</span>
                        <span class="detail-value">${escapeHtml(instrument.startDate)}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">End Date</span>
                        <span class="detail-value">${escapeHtml(instrument.endDate)}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">Rate</span>
                        <span class="detail-value">${formatPercent(instrument.rate)}</span>
                    </div>
                </div>
            `;
        }

        if (convention) {
            html += `
                <div class="detail-section">
                    <div class="detail-section-title"><i class="fas fa-book"></i> Convention</div>
                    <div class="detail-row">
                        <span class="detail-label">Type</span>
                        <span class="detail-value">${escapeHtml(convention.conventionType)}</span>
                    </div>
                    ${convention.fields.map(f => `
                    <div class="detail-row">
                        <span class="detail-label">${escapeHtml(f.label)}</span>
                        <span class="detail-value">${escapeHtml(f.value)}</span>
                    </div>
                    `).join('')}
                </div>
            `;
        }

        if (elements.detailContent) {
            elements.detailContent.innerHTML = html;
        }
    }

    function renderConventions(conventions) {
        if (!elements.conventionsGrid) return;

        if (conventions.length === 0) {
            const assetClass = state.assetClass;
            elements.conventionsGrid.innerHTML = `
                <div class="conventions-empty">
                    <i class="fas fa-info-circle"></i>
                    <p>No conventions for ${assetClass}</p>
                </div>
            `;
            return;
        }

        const html = conventions.map(conv => `
            <div class="convention-card ${state.selectedConventionId === conv.id ? 'selected' : ''}"
                 data-convention-id="${escapeHtml(conv.id)}">
                <div class="convention-card-header">
                    <span class="convention-id">${escapeHtml(conv.id)}</span>
                    <span class="convention-type-badge">${escapeHtml(conv.conventionType)}</span>
                </div>
                <div class="convention-currency">${escapeHtml(conv.currency)}</div>
                ${conv.isDefault ? `
                <span class="convention-default-badge">
                    <i class="fas fa-check"></i> Default
                </span>
                ` : ''}
            </div>
        `).join('');

        elements.conventionsGrid.innerHTML = html;

        // Add click handlers to convention cards
        elements.conventionsGrid.querySelectorAll('.convention-card').forEach(card => {
            card.addEventListener('click', () => {
                const conventionId = card.dataset.conventionId;
                selectConvention(conventionId);
            });
        });
    }

    // ===========================================
    // Actions
    // ===========================================

    function selectRate(rateId) {
        state.selectedRateId = rateId;
        updateTableSelection();
        renderDetailPanel();
    }

    function updateTableSelection() {
        elements.ratesTbody?.querySelectorAll('tr').forEach(row => {
            row.classList.toggle('selected', row.dataset.rateId === state.selectedRateId);
        });
    }

    function selectConvention(conventionId) {
        state.selectedConventionId = conventionId;
        updateConventionSelection();
        renderConventionDetailPanel();
    }

    function updateConventionSelection() {
        elements.conventionsGrid?.querySelectorAll('.convention-card').forEach(card => {
            card.classList.toggle('selected', card.dataset.conventionId === state.selectedConventionId);
        });
    }

    function renderConventionDetailPanel() {
        if (!elements.conventionDetailContent) return;

        if (!state.selectedConventionId) {
            elements.conventionDetailContent.innerHTML = `
                <div class="detail-placeholder">
                    <i class="fas fa-hand-pointer"></i>
                    <p>Select a convention to view details</p>
                </div>
            `;
            return;
        }

        const convention = state.filteredConventions.find(c => c.id === state.selectedConventionId);
        if (!convention) {
            loadConventionDetail(state.selectedConventionId);
            return;
        }

        renderConventionDetailContent(convention);
    }

    async function loadConventionDetail(conventionId) {
        try {
            const response = await fetch(`/api/market/conventions/${encodeURIComponent(conventionId)}`);
            if (!response.ok) throw new Error('Not found');

            const detail = await response.json();
            renderConventionDetailContent(detail);
        } catch (error) {
            logError('Failed to load convention detail:', error);
            elements.conventionDetailContent.innerHTML = `
                <div class="detail-placeholder">
                    <i class="fas fa-exclamation-circle"></i>
                    <p>Failed to load convention detail</p>
                </div>
            `;
        }
    }

    function renderConventionDetailContent(convention) {
        if (!elements.conventionDetailContent) return;

        let html = `
            <div class="detail-section">
                <div class="detail-section-title"><i class="fas fa-book"></i> Convention Information</div>
                <div class="detail-row">
                    <span class="detail-label">ID</span>
                    <span class="detail-value">${escapeHtml(convention.id)}</span>
                </div>
                <div class="detail-row">
                    <span class="detail-label">Type</span>
                    <span class="detail-value">${escapeHtml(convention.conventionType)}</span>
                </div>
                <div class="detail-row">
                    <span class="detail-label">Currency</span>
                    <span class="detail-value">${escapeHtml(convention.currency)}</span>
                </div>
                ${convention.isDefault ? `
                <div class="detail-row">
                    <span class="detail-label">Status</span>
                    <span class="detail-value"><span class="status-badge success">Default</span></span>
                </div>
                ` : ''}
            </div>
        `;

        // Add fields section if available
        if (convention.fields && convention.fields.length > 0) {
            html += `
                <div class="detail-section">
                    <div class="detail-section-title"><i class="fas fa-cog"></i> Parameters</div>
                    ${convention.fields.map(f => `
                    <div class="detail-row">
                        <span class="detail-label">${escapeHtml(f.label)}</span>
                        <span class="detail-value">${escapeHtml(f.value)}</span>
                    </div>
                    `).join('')}
                </div>
            `;
        }

        elements.conventionDetailContent.innerHTML = html;
    }

    async function exportRates(format) {
        try {
            const query = buildQueryParams();
            const endpoint = format === 'csv' ? '/api/market/export/csv' : '/api/market/export/json';
            const response = await fetch(`${endpoint}${query}`);

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
            logError('Export failed:', error);
            showToast('Export failed', 'error');
        }
    }

    // ===========================================
    // UI Updates
    // ===========================================

    function showLoading(show) {
        if (elements.loading) {
            elements.loading.style.display = show ? 'flex' : 'none';
        }
        if (elements.ratesTbody) {
            elements.ratesTbody.style.display = show ? 'none' : '';
        }
    }

    function updateStats() {
        const totalAll = state.rates.length;
        const liveAll = state.rates.filter(r => !r.isStale).length;
        const displayed = state.filteredRates.length;
        const stale = state.filteredRates.filter(r => r.isStale).length;

        // Update header stats badges
        if (elements.statsTotal) elements.statsTotal.textContent = totalAll;
        if (elements.statsLive) elements.statsLive.textContent = liveAll;
        if (elements.statsDisplayed) elements.statsDisplayed.textContent = displayed;

        // Update table stats (legacy)
        if (elements.totalCount) elements.totalCount.textContent = displayed;
        if (elements.staleCount) elements.staleCount.textContent = stale;
    }

    function updateConventionsCount() {
        if (elements.conventionsCount) {
            elements.conventionsCount.textContent = state.filteredConventions.length;
        }
    }

    function updateLastUpdated() {
        if (!elements.lastUpdated || !state.lastUpdated) return;
        const date = new Date(state.lastUpdated);
        elements.lastUpdated.textContent = date.toLocaleTimeString();
    }

    // ===========================================
    // Formatting Helpers
    // ===========================================

    function formatRate(value, rateType) {
        if (rateType === 'FxSpot') {
            return value.toFixed(4);
        }
        if (rateType === 'FxForward') {
            // FX Forward points
            return value.toFixed(2) + ' pts';
        }
        if (rateType === 'XccyBasis') {
            // XCCY Basis spread in basis points
            return (value * 10000).toFixed(2) + ' bps';
        }
        // Interest rates as percentage with basis point precision
        return (value * 100).toFixed(4) + '%';
    }

    function formatPercent(value) {
        return (value * 100).toFixed(4) + '%';
    }

    function formatTimestamp(ts) {
        const date = new Date(ts);
        return date.toLocaleString();
    }

    function escapeHtml(str) {
        if (str == null) return '';
        return String(str)
            .replace(/&/g, '&amp;')
            .replace(/</g, '&lt;')
            .replace(/>/g, '&gt;')
            .replace(/"/g, '&quot;');
    }

    // ===========================================
    // Utilities
    // ===========================================

    function showToast(message, type) {
        // Use existing toast function if available
        if (typeof window.showToast === 'function') {
            window.showToast(message, type);
        } else {
            console.log(`[${type}] ${message}`);
        }
    }

    function showError(message) {
        showToast(message, 'error');
    }

    function log(...args) {
        if (typeof window.log === 'function') {
            window.log('MarketDataViewer', ...args);
        } else {
            console.log('[MarketDataViewer]', ...args);
        }
    }

    function logError(...args) {
        if (typeof window.logError === 'function') {
            window.logError('MarketDataViewer', ...args);
        } else {
            console.error('[MarketDataViewer]', ...args);
        }
    }

    // ===========================================
    // Public API
    // ===========================================

    return {
        init,
        refresh: loadRates,
        getRates: () => [...state.rates],
        getState: () => ({ ...state })
    };
})();

// Navigation hook removed - marketDataViewer.init() is called directly from app.js navigateTo()

// Initialise Market Data Viewer when DOM is ready and view is shown
document.addEventListener('DOMContentLoaded', () => {
    // Initialise if market-data view is visible
    const marketDataView = document.getElementById('market-data-view');
    if (marketDataView && marketDataView.classList.contains('active')) {
        marketDataViewer.init();
    }
});
