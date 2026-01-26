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
        sortColumn: 'tenor',
        sortDirection: 'asc',
        lastUpdated: null,
        previousValues: new Map(), // For change highlighting
        isInitialised: false,
        assetClass: 'Rates', // Default asset class filter
        allConventions: [], // All conventions (unfiltered)
        filteredConventions: [], // Filtered conventions for current asset class
        // IRVol state
        irVolCurrencies: [],
        irVolQuotes: [], // Flattened quotes for table display
        selectedIrVolCurrency: null,
        // FXVol state
        fxVolPairs: [],
        fxVolQuotes: [], // Flattened quotes for table display
        selectedFxVolPair: null
    };

    // DOM Elements (cached)
    let elements = {};

    // Tenor order map (populated from infra_master via API)
    let TENOR_ORDER = {};

    /**
     * Compares two tenor strings using INFRA_MASTER Tenor enum order.
     * Falls back to string comparison for unknown tenors.
     */
    function compareTenor(a, b) {
        const aUpper = String(a).toUpperCase();
        const bUpper = String(b).toUpperCase();
        const aOrder = TENOR_ORDER[aUpper];
        const bOrder = TENOR_ORDER[bUpper];

        // Both known tenors
        if (aOrder !== undefined && bOrder !== undefined) {
            return aOrder - bOrder;
        }
        // Unknown tenor goes to the end
        if (aOrder === undefined && bOrder !== undefined) return 1;
        if (aOrder !== undefined && bOrder === undefined) return -1;
        // Both unknown: fallback to string comparison
        return aUpper.localeCompare(bUpper);
    }

    // ===========================================
    // Initialisation
    // ===========================================

    /**
     * Load market configuration (tenor order) from API.
     */
    async function loadConfig() {
        try {
            const response = await fetch('/api/market/config');
            if (!response.ok) {
                throw new Error(`Config fetch failed: ${response.status}`);
            }
            const config = await response.json();

            // Build tenor order map from API response
            TENOR_ORDER = {};
            config.tenorOrder.forEach((tenor, index) => {
                TENOR_ORDER[tenor.toUpperCase()] = index;
            });
            // Add alternative notation for Overnight
            if (TENOR_ORDER['ON'] !== undefined) {
                TENOR_ORDER['O/N'] = TENOR_ORDER['ON'];
            }

            log('Market config loaded:', Object.keys(TENOR_ORDER).length, 'tenors');
        } catch (error) {
            log('Failed to load market config:', error);
            // Fallback to default order if API fails
            TENOR_ORDER = {
                'ON': 0, 'O/N': 0, '1W': 1, '2W': 2, '1M': 3, '2M': 4, '3M': 5,
                '6M': 6, '9M': 7, '1Y': 8, '2Y': 9, '3Y': 10, '5Y': 11, '7Y': 12,
                '10Y': 13, '15Y': 14, '20Y': 15, '30Y': 16
            };
        }
    }

    function init() {
        if (state.isInitialised) {
            loadRates();
            return;
        }

        cacheElements();
        bindEvents();
        // Load config first, then load data
        loadConfig().then(() => {
            loadRates();
            loadConventions();
        });
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
            if (state.assetClass === 'IRVol') {
                // Reload IRVol data
                state.irVolQuotes = []; // Clear cache to force reload
                await loadIrVolData();
                showToast('IR Vol data refreshed', 'success');
            } else if (state.assetClass === 'FXVol') {
                // Reload FXVol data
                state.fxVolQuotes = []; // Clear cache to force reload
                await loadFxVolData();
                showToast('FX Vol data refreshed', 'success');
            } else {
                // Original rates refresh
                const response = await fetch('/api/market/rates/refresh', { method: 'POST' });
                if (!response.ok) throw new Error('Failed to refresh');
                await loadRates();
                showToast('Market data refreshed', 'success');
            }
        } catch (error) {
            logError('Failed to refresh data:', error);
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

    // ===========================================
    // IRVol Data Loading
    // ===========================================

    async function loadIrVolData() {
        showLoading(true);
        try {
            // Load available currencies
            const currenciesResp = await fetch('/api/irvol/currencies');
            if (!currenciesResp.ok) throw new Error('Failed to fetch IR vol currencies');

            const currenciesData = await currenciesResp.json();
            state.irVolCurrencies = currenciesData.currencies || [];

            // Load quotes for all currencies and flatten for table display
            const allQuotes = [];
            for (const currency of state.irVolCurrencies) {
                try {
                    const quotesResp = await fetch(`/api/irvol/quotes/${currency.currency}`);
                    if (quotesResp.ok) {
                        const quotesData = await quotesResp.json();
                        for (const quote of quotesData.quotes || []) {
                            allQuotes.push({
                                id: `${currency.currency}-${quote.expiry}-${quote.tenor}`,
                                currency: currency.currency,
                                expiry: quote.expiry,
                                tenor: quote.tenor,
                                atmVol: quote.atmVol,
                                volType: quotesData.volType || 'Normal',
                                smile: quote.smile || [],
                                source: quotesData.source || 'Demo'
                            });
                        }
                    }
                } catch (e) {
                    logError(`Failed to load quotes for ${currency.currency}:`, e);
                }
            }

            state.irVolQuotes = allQuotes;
            state.lastUpdated = new Date().toISOString();

            sortIrVolQuotes();
            renderIrVolTable();
            updateIrVolStats();
            updateLastUpdated();

            log(`Loaded ${allQuotes.length} IR vol quotes`);
        } catch (error) {
            logError('Failed to load IR vol data:', error);
            showError('Failed to load IR volatility data');
        } finally {
            showLoading(false);
        }
    }

    function renderIrVolTable() {
        if (!elements.ratesTbody) return;

        // Update table headers for IRVol
        updateTableHeadersForAssetClass('IRVol');

        // Filter by selected currency if any
        let filteredQuotes = state.irVolQuotes;
        const currency = elements.currencyFilter?.value;
        if (currency) {
            filteredQuotes = filteredQuotes.filter(q => q.currency === currency);
        }

        if (filteredQuotes.length === 0) {
            elements.ratesTbody.innerHTML = '';
            if (elements.placeholder) {
                elements.placeholder.style.display = 'flex';
                elements.placeholder.innerHTML = `
                    <i class="fas fa-chart-area"></i>
                    <p>No IR volatility data available</p>
                    <span class="placeholder-hint">Check that the IR vol API is returning data</span>
                `;
            }
            return;
        }

        if (elements.placeholder) elements.placeholder.style.display = 'none';

        const html = filteredQuotes.map(quote => {
            // Calculate smile range for display
            const smileInfo = quote.smile.length > 0
                ? `${quote.smile.length} points`
                : 'ATM only';

            return `
                <tr data-quote-id="${quote.id}" class="${state.selectedRateId === quote.id ? 'selected' : ''}">
                    <td>${escapeHtml(quote.currency)}</td>
                    <td>${escapeHtml(quote.expiry)}</td>
                    <td>${escapeHtml(quote.tenor)}</td>
                    <td class="numeric">
                        <span class="rate-value">${formatVol(quote.atmVol)}</span>
                    </td>
                    <td>${escapeHtml(quote.volType)}</td>
                    <td>${smileInfo}</td>
                    <td>${escapeHtml(quote.source)}</td>
                    <td>
                        <span class="status-indicator fresh">
                            <i class="fas fa-check"></i>
                            Live
                        </span>
                    </td>
                </tr>
            `;
        }).join('');

        elements.ratesTbody.innerHTML = html;

        // Bind row click events for IRVol
        elements.ratesTbody.querySelectorAll('tr').forEach(row => {
            row.addEventListener('click', () => {
                const quoteId = row.dataset.quoteId;
                selectIrVolQuote(quoteId);
            });
        });
    }

    function selectIrVolQuote(quoteId) {
        state.selectedRateId = quoteId;
        const quote = state.irVolQuotes.find(q => q.id === quoteId);
        if (quote) {
            renderIrVolDetail(quote);
        }
        updateIrVolTableSelection();
    }

    function updateIrVolTableSelection() {
        elements.ratesTbody?.querySelectorAll('tr').forEach(row => {
            row.classList.toggle('selected', row.dataset.quoteId === state.selectedRateId);
        });
    }

    function renderIrVolDetail(quote) {
        if (!elements.detailContent) return;

        const smileHtml = quote.smile.length > 0
            ? quote.smile.map(p => `
                <div class="detail-row">
                    <span class="detail-label">${p.strikeOffsetBp > 0 ? '+' : ''}${p.strikeOffsetBp}bp</span>
                    <span class="detail-value">${formatVol(p.vol)}</span>
                </div>
            `).join('')
            : '<div class="detail-row"><span class="detail-label">No smile data</span></div>';

        elements.detailContent.innerHTML = `
            <div class="detail-section">
                <div class="detail-section-title"><i class="fas fa-chart-line"></i> Swaption Vol Quote</div>
                <div class="detail-row">
                    <span class="detail-label">Currency</span>
                    <span class="detail-value">${escapeHtml(quote.currency)}</span>
                </div>
                <div class="detail-row">
                    <span class="detail-label">Expiry</span>
                    <span class="detail-value">${escapeHtml(quote.expiry)}</span>
                </div>
                <div class="detail-row">
                    <span class="detail-label">Tenor</span>
                    <span class="detail-value">${escapeHtml(quote.tenor)}</span>
                </div>
                <div class="detail-row">
                    <span class="detail-label">ATM Vol</span>
                    <span class="detail-value large">${formatVol(quote.atmVol)}</span>
                </div>
                <div class="detail-row">
                    <span class="detail-label">Vol Type</span>
                    <span class="detail-value">${escapeHtml(quote.volType)}</span>
                </div>
            </div>
            <div class="detail-section">
                <div class="detail-section-title"><i class="fas fa-smile"></i> Smile Curve</div>
                ${smileHtml}
            </div>
        `;
    }

    function updateIrVolStats() {
        const total = state.irVolQuotes.length;
        const currencies = new Set(state.irVolQuotes.map(q => q.currency)).size;

        if (elements.statsTotal) elements.statsTotal.textContent = total;
        if (elements.statsLive) elements.statsLive.textContent = currencies;
        if (elements.statsDisplayed) elements.statsDisplayed.textContent = total;
        if (elements.totalCount) elements.totalCount.textContent = total;
        if (elements.staleCount) elements.staleCount.textContent = '0';
    }

    // ===========================================
    // FXVol Data Loading
    // ===========================================

    async function loadFxVolData() {
        showLoading(true);
        try {
            // Load available pairs
            const pairsResp = await fetch('/api/fxvol/pairs');
            if (!pairsResp.ok) throw new Error('Failed to fetch FX vol pairs');

            const pairsData = await pairsResp.json();
            state.fxVolPairs = pairsData.pairs || [];

            // Load quotes for all pairs and flatten for table display
            const allQuotes = [];
            for (const pairInfo of state.fxVolPairs) {
                try {
                    const quotesResp = await fetch(`/api/fxvol/quotes/${pairInfo.pair}`);
                    if (quotesResp.ok) {
                        const quotesData = await quotesResp.json();
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
                                source: 'Demo'
                            });
                        }
                    }
                } catch (e) {
                    logError(`Failed to load quotes for ${pairInfo.pair}:`, e);
                }
            }

            state.fxVolQuotes = allQuotes;
            state.lastUpdated = new Date().toISOString();

            sortFxVolQuotes();
            renderFxVolTable();
            updateFxVolStats();
            updateLastUpdated();

            log(`Loaded ${allQuotes.length} FX vol quotes`);
        } catch (error) {
            logError('Failed to load FX vol data:', error);
            showError('Failed to load FX volatility data');
        } finally {
            showLoading(false);
        }
    }

    function expiryToLabel(expiry) {
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

    function renderFxVolTable() {
        if (!elements.ratesTbody) return;

        // Update table headers for FXVol
        updateTableHeadersForAssetClass('FXVol');

        // Filter by selected currency pair (using currency filter for pair)
        let filteredQuotes = state.fxVolQuotes;
        const pairFilter = elements.currencyFilter?.value;
        if (pairFilter) {
            filteredQuotes = filteredQuotes.filter(q => q.pair.includes(pairFilter));
        }

        if (filteredQuotes.length === 0) {
            elements.ratesTbody.innerHTML = '';
            if (elements.placeholder) {
                elements.placeholder.style.display = 'flex';
                elements.placeholder.innerHTML = `
                    <i class="fas fa-chart-area"></i>
                    <p>No FX volatility data available</p>
                    <span class="placeholder-hint">Check that the FX vol API is returning data</span>
                `;
            }
            return;
        }

        if (elements.placeholder) elements.placeholder.style.display = 'none';

        const html = filteredQuotes.map(quote => {
            return `
                <tr data-quote-id="${quote.id}" class="${state.selectedRateId === quote.id ? 'selected' : ''}">
                    <td>${escapeHtml(quote.pair)}</td>
                    <td>${escapeHtml(quote.expiryLabel)}</td>
                    <td class="numeric">
                        <span class="rate-value">${formatVol(quote.atmVol)}</span>
                    </td>
                    <td class="numeric ${quote.rr25d >= 0 ? '' : 'negative'}">${formatVolBps(quote.rr25d)}</td>
                    <td class="numeric">${formatVolBps(quote.bf25d)}</td>
                    <td class="numeric ${quote.rr10d >= 0 ? '' : 'negative'}">${formatVolBps(quote.rr10d)}</td>
                    <td class="numeric">${formatVolBps(quote.bf10d)}</td>
                    <td>
                        <span class="status-indicator fresh">
                            <i class="fas fa-check"></i>
                            Live
                        </span>
                    </td>
                </tr>
            `;
        }).join('');

        elements.ratesTbody.innerHTML = html;

        // Bind row click events for FXVol
        elements.ratesTbody.querySelectorAll('tr').forEach(row => {
            row.addEventListener('click', () => {
                const quoteId = row.dataset.quoteId;
                selectFxVolQuote(quoteId);
            });
        });
    }

    function selectFxVolQuote(quoteId) {
        state.selectedRateId = quoteId;
        const quote = state.fxVolQuotes.find(q => q.id === quoteId);
        if (quote) {
            renderFxVolDetail(quote);
        }
        updateFxVolTableSelection();
    }

    function updateFxVolTableSelection() {
        elements.ratesTbody?.querySelectorAll('tr').forEach(row => {
            row.classList.toggle('selected', row.dataset.quoteId === state.selectedRateId);
        });
    }

    function renderFxVolDetail(quote) {
        if (!elements.detailContent) return;

        // Calculate delta vols from RR/BF
        const vol25c = quote.atmVol + quote.bf25d + quote.rr25d / 2;
        const vol25p = quote.atmVol + quote.bf25d - quote.rr25d / 2;
        // 10D vols are optional
        const has10d = quote.rr10d != null && quote.bf10d != null;
        const vol10c = has10d ? quote.atmVol + quote.bf10d + quote.rr10d / 2 : null;
        const vol10p = has10d ? quote.atmVol + quote.bf10d - quote.rr10d / 2 : null;

        elements.detailContent.innerHTML = `
            <div class="detail-section">
                <div class="detail-section-title"><i class="fas fa-chart-line"></i> FX Vol Quote</div>
                <div class="detail-row">
                    <span class="detail-label">Pair</span>
                    <span class="detail-value">${escapeHtml(quote.pair)}</span>
                </div>
                <div class="detail-row">
                    <span class="detail-label">Expiry</span>
                    <span class="detail-value">${escapeHtml(quote.expiryLabel)} (${quote.expiry.toFixed(4)}Y)</span>
                </div>
                <div class="detail-row">
                    <span class="detail-label">Spot</span>
                    <span class="detail-value">${quote.spot ? quote.spot.toFixed(4) : '-'}</span>
                </div>
            </div>
            <div class="detail-section">
                <div class="detail-section-title"><i class="fas fa-smile"></i> ATM & Smile</div>
                <div class="detail-row">
                    <span class="detail-label">ATM Vol</span>
                    <span class="detail-value large">${formatVol(quote.atmVol)}</span>
                </div>
                <div class="detail-row">
                    <span class="detail-label">25D RR</span>
                    <span class="detail-value">${formatVolBps(quote.rr25d)}</span>
                </div>
                <div class="detail-row">
                    <span class="detail-label">25D BF</span>
                    <span class="detail-value">${formatVolBps(quote.bf25d)}</span>
                </div>
                <div class="detail-row">
                    <span class="detail-label">10D RR</span>
                    <span class="detail-value">${formatVolBps(quote.rr10d)}</span>
                </div>
                <div class="detail-row">
                    <span class="detail-label">10D BF</span>
                    <span class="detail-value">${formatVolBps(quote.bf10d)}</span>
                </div>
            </div>
            <div class="detail-section">
                <div class="detail-section-title"><i class="fas fa-calculator"></i> Delta Vols (Derived)</div>
                <div class="detail-row">
                    <span class="detail-label">10D Put</span>
                    <span class="detail-value">${formatVol(vol10p)}</span>
                </div>
                <div class="detail-row">
                    <span class="detail-label">25D Put</span>
                    <span class="detail-value">${formatVol(vol25p)}</span>
                </div>
                <div class="detail-row">
                    <span class="detail-label">ATM</span>
                    <span class="detail-value">${formatVol(quote.atmVol)}</span>
                </div>
                <div class="detail-row">
                    <span class="detail-label">25D Call</span>
                    <span class="detail-value">${formatVol(vol25c)}</span>
                </div>
                <div class="detail-row">
                    <span class="detail-label">10D Call</span>
                    <span class="detail-value">${formatVol(vol10c)}</span>
                </div>
            </div>
        `;
    }

    function updateFxVolStats() {
        const total = state.fxVolQuotes.length;
        const pairs = new Set(state.fxVolQuotes.map(q => q.pair)).size;

        if (elements.statsTotal) elements.statsTotal.textContent = total;
        if (elements.statsLive) elements.statsLive.textContent = pairs;
        if (elements.statsDisplayed) elements.statsDisplayed.textContent = total;
        if (elements.totalCount) elements.totalCount.textContent = total;
        if (elements.staleCount) elements.staleCount.textContent = '0';
    }

    // ===========================================
    // Dynamic Table Headers
    // ===========================================

    function updateTableHeadersForAssetClass(assetClass) {
        const thead = elements.ratesTable?.querySelector('thead tr');
        if (!thead) return;

        switch (assetClass) {
            case 'IRVol':
                thead.innerHTML = `
                    <th class="sortable" data-sort="currency">Currency <i class="fas fa-sort"></i></th>
                    <th class="sortable" data-sort="expiry">Expiry <i class="fas fa-sort"></i></th>
                    <th class="sortable" data-sort="tenor">Tenor <i class="fas fa-sort"></i></th>
                    <th class="sortable numeric" data-sort="atmVol">ATM Vol <i class="fas fa-sort"></i></th>
                    <th>Vol Type</th>
                    <th>Smile</th>
                    <th>Source</th>
                    <th>Status</th>
                `;
                break;
            case 'FXVol':
                thead.innerHTML = `
                    <th class="sortable" data-sort="pair">Pair <i class="fas fa-sort"></i></th>
                    <th class="sortable" data-sort="expiry">Expiry <i class="fas fa-sort"></i></th>
                    <th class="sortable numeric" data-sort="atmVol">ATM Vol <i class="fas fa-sort"></i></th>
                    <th class="sortable numeric" data-sort="rr25d">25D RR <i class="fas fa-sort"></i></th>
                    <th class="sortable numeric" data-sort="bf25d">25D BF <i class="fas fa-sort"></i></th>
                    <th class="sortable numeric" data-sort="rr10d">10D RR <i class="fas fa-sort"></i></th>
                    <th class="sortable numeric" data-sort="bf10d">10D BF <i class="fas fa-sort"></i></th>
                    <th>Status</th>
                `;
                break;
            default:
                // Rates and FX use the original headers
                thead.innerHTML = `
                    <th class="sortable" data-sort="id">ID <i class="fas fa-sort"></i></th>
                    <th class="sortable" data-sort="currency">Currency <i class="fas fa-sort"></i></th>
                    <th class="sortable" data-sort="tenor">Tenor <i class="fas fa-sort"></i></th>
                    <th class="sortable" data-sort="rateType">Type <i class="fas fa-sort"></i></th>
                    <th class="sortable numeric" data-sort="value">Value <i class="fas fa-sort"></i></th>
                    <th>Index</th>
                    <th>Source</th>
                    <th>Status</th>
                `;
                break;
        }

        // Re-bind sorting events for new headers
        thead.querySelectorAll('th.sortable').forEach(th => {
            th.addEventListener('click', () => {
                const column = th.dataset.sort;
                if (state.sortColumn === column) {
                    state.sortDirection = state.sortDirection === 'asc' ? 'desc' : 'asc';
                } else {
                    state.sortColumn = column;
                    state.sortDirection = 'asc';
                }
                updateSortIndicators();
                // Re-render appropriate table
                if (assetClass === 'IRVol') {
                    sortIrVolQuotes();
                    renderIrVolTable();
                } else if (assetClass === 'FXVol') {
                    sortFxVolQuotes();
                    renderFxVolTable();
                } else {
                    sortAndRender();
                }
            });
        });
    }

    function sortIrVolQuotes() {
        const col = state.sortColumn;
        const dir = state.sortDirection === 'asc' ? 1 : -1;

        state.irVolQuotes.sort((a, b) => {
            // Primary sort: always by currency
            const currencyCompare = String(a.currency || '').localeCompare(String(b.currency || ''));
            if (currencyCompare !== 0) {
                return currencyCompare;
            }

            // Secondary sort: by selected column
            let aVal = a[col];
            let bVal = b[col];

            if (aVal == null) aVal = '';
            if (bVal == null) bVal = '';

            if (col === 'atmVol') {
                return (parseFloat(aVal) - parseFloat(bVal)) * dir;
            }
            // Tenor/expiry comparison using INFRA_MASTER order
            if (col === 'tenor' || col === 'expiry') {
                return compareTenor(aVal, bVal) * dir;
            }
            return String(aVal).localeCompare(String(bVal)) * dir;
        });
    }

    function sortFxVolQuotes() {
        const col = state.sortColumn;
        const dir = state.sortDirection === 'asc' ? 1 : -1;

        state.fxVolQuotes.sort((a, b) => {
            // Primary sort: always by pair
            const pairCompare = String(a.pair || '').localeCompare(String(b.pair || ''));
            if (pairCompare !== 0) {
                return pairCompare;
            }

            // Secondary sort: by selected column
            let aVal = a[col];
            let bVal = b[col];

            if (aVal == null) aVal = '';
            if (bVal == null) bVal = '';

            if (['atmVol', 'rr25d', 'bf25d', 'rr10d', 'bf10d', 'expiry'].includes(col)) {
                return (parseFloat(aVal) - parseFloat(bVal)) * dir;
            }
            return String(aVal).localeCompare(String(bVal)) * dir;
        });
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
        state.selectedRateId = null; // Clear selection when switching

        // Set default sort column for each asset class
        if (assetClass === 'FXVol') {
            state.sortColumn = 'expiry';
        } else {
            state.sortColumn = 'tenor';
        }
        state.sortDirection = 'asc';

        updateAssetClassToggle();

        // Load data based on asset class
        if (assetClass === 'IRVol') {
            if (state.irVolQuotes.length === 0) {
                loadIrVolData();
            } else {
                updateTableHeadersForAssetClass('IRVol');
                sortIrVolQuotes();
                renderIrVolTable();
                updateIrVolStats();
            }
        } else if (assetClass === 'FXVol') {
            if (state.fxVolQuotes.length === 0) {
                loadFxVolData();
            } else {
                updateTableHeadersForAssetClass('FXVol');
                sortFxVolQuotes();
                renderFxVolTable();
                updateFxVolStats();
            }
        } else {
            // Rates or FX
            updateTableHeadersForAssetClass(assetClass);
            applyFilters();
        }

        filterAndRenderConventions();
        renderDetailPanel(); // Clear detail panel
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
        // For IRVol and FXVol, re-render with updated filters
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

        // Original filtering for Rates and FX
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
            // Primary sort: always by currency
            const currencyCompare = String(a.currency || '').localeCompare(String(b.currency || ''));
            if (currencyCompare !== 0) {
                return currencyCompare;
            }

            // Secondary sort: by selected column
            let aVal = a[col];
            let bVal = b[col];

            // Handle null/undefined
            if (aVal == null) aVal = '';
            if (bVal == null) bVal = '';

            // Numeric comparison for value
            if (col === 'value') {
                return (parseFloat(aVal) - parseFloat(bVal)) * dir;
            }

            // Tenor comparison using INFRA_MASTER order
            if (col === 'tenor') {
                return compareTenor(aVal, bVal) * dir;
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

    function formatVol(value) {
        // Format volatility as percentage (e.g., 0.12 -> 12.00%)
        if (value == null) return '-';
        return (value * 100).toFixed(2) + '%';
    }

    function formatVolBps(value) {
        // Format volatility difference as basis points (e.g., 0.005 -> 50 bps)
        if (value == null) return '-';
        const bps = value * 10000;
        const sign = bps >= 0 ? '+' : '';
        return sign + bps.toFixed(1) + ' bps';
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
        refresh: refreshRates,
        getRates: () => [...state.rates],
        getIrVolQuotes: () => [...state.irVolQuotes],
        getFxVolQuotes: () => [...state.fxVolQuotes],
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
