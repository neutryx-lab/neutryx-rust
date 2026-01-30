/**
 * VolCube Builder Module
 * Handles volatility surface calibration for swaptions and FX options
 */

const volcubeBuilder = {
    state: {
        activeTab: 'swaption',
        // Swaption state
        swaptionIndices: [],
        selectedSwaptionIndex: null,
        swaptionInstruments: [],
        swaptionModels: [],
        // FX state
        fxPairs: [],
        selectedFxPair: null,
        fxQuotes: [],
        deltaTypes: [],
        fxSurfaceId: null,
        // Calibration result
        calibrationResult: null,
        // Slice selectors
        expiries: [],
        tenors: [],
        selectedExpiry: null,
        selectedTenor: null
    },

    elements: {},

    async init() {
        this.cacheElements();
        this.attachEventListeners();
        await this.loadSwaptionIndices();
        await this.loadSwaptionModels();
        await this.loadFxPairs();
        await this.loadDeltaTypes();

        if (typeof Logger !== 'undefined') {
            Logger.info('VolCubeBuilder', 'VolCube builder module initialised');
        }
    },

    cacheElements() {
        // Tabs and panels
        this.elements.assetTabs = document.getElementById('volcube-asset-tabs');
        this.elements.swaptionPanel = document.getElementById('volcube-swaption-panel');
        this.elements.fxPanel = document.getElementById('volcube-fx-panel');

        // Swaption elements
        this.elements.indexSelector = document.getElementById('volcube-index-selector');
        this.elements.referenceDate = document.getElementById('volcube-reference-date');
        this.elements.calibSettings = document.getElementById('volcube-calib-settings');
        this.elements.instrumentsTable = document.getElementById('volcube-instruments-table');

        // FX elements
        this.elements.fxPairSelector = document.getElementById('fxvol-pair-selector');
        this.elements.fxModelSelector = document.getElementById('fxvol-model-selector');
        this.elements.fxSpot = document.getElementById('fxvol-spot');
        this.elements.fxDomesticRate = document.getElementById('fxvol-domestic-rate');
        this.elements.fxForeignRate = document.getElementById('fxvol-foreign-rate');
        this.elements.fxQuotesTable = document.getElementById('fxvol-quotes-table');

        // Result elements
        this.elements.expirySelector = document.getElementById('volcube-expiry-selector');
        this.elements.tenorSelector = document.getElementById('volcube-tenor-selector');
        this.elements.calibrateBtn = document.getElementById('volcube-calibrate-btn');
        this.elements.exportCsvBtn = document.getElementById('volcube-export-csv');
        this.elements.exportJsonBtn = document.getElementById('volcube-export-json');

        // Set default reference date
        if (this.elements.referenceDate) {
            this.elements.referenceDate.value = new Date().toISOString().split('T')[0];
        }
    },

    attachEventListeners() {
        // Calibrate button
        if (this.elements.calibrateBtn) {
            this.elements.calibrateBtn.addEventListener('click', () => this.calibrate());
        }

        // Index selector
        if (this.elements.indexSelector) {
            this.elements.indexSelector.addEventListener('change', (e) => {
                this.state.selectedSwaptionIndex = e.target.value;
                if (this.state.selectedSwaptionIndex) {
                    this.loadSwaptionInstruments(this.state.selectedSwaptionIndex);
                }
            });
        }

        // FX pair selector
        if (this.elements.fxPairSelector) {
            this.elements.fxPairSelector.addEventListener('change', (e) => {
                this.state.selectedFxPair = e.target.value;
                if (this.state.selectedFxPair) {
                    this.loadFxQuotes(this.state.selectedFxPair);
                }
            });
        }

        // Slice selectors
        if (this.elements.expirySelector) {
            this.elements.expirySelector.addEventListener('change', (e) => {
                this.state.selectedExpiry = parseFloat(e.target.value);
                this.updateVisualization();
            });
        }
        if (this.elements.tenorSelector) {
            this.elements.tenorSelector.addEventListener('change', (e) => {
                this.state.selectedTenor = parseFloat(e.target.value);
                this.updateVisualization();
            });
        }

        // Export buttons
        if (this.elements.exportCsvBtn) {
            this.elements.exportCsvBtn.addEventListener('click', () => this.exportCsv());
        }
        if (this.elements.exportJsonBtn) {
            this.elements.exportJsonBtn.addEventListener('click', () => this.exportJson());
        }
    },

    renderAssetTabs() {
        if (!this.elements.assetTabs) return;

        this.elements.assetTabs.innerHTML = `
            <button class="volcube-tab ${this.state.activeTab === 'swaption' ? 'active' : ''}" data-tab="swaption">
                <i class="fas fa-percentage"></i> Swaption
            </button>
            <button class="volcube-tab ${this.state.activeTab === 'fx' ? 'active' : ''}" data-tab="fx">
                <i class="fas fa-exchange-alt"></i> FX
            </button>
        `;

        this.elements.assetTabs.querySelectorAll('.volcube-tab').forEach(tab => {
            tab.addEventListener('click', () => {
                this.state.activeTab = tab.dataset.tab;
                this.renderAssetTabs();
                this.togglePanels();
            });
        });
    },

    togglePanels() {
        if (this.elements.swaptionPanel) {
            this.elements.swaptionPanel.style.display = this.state.activeTab === 'swaption' ? 'block' : 'none';
        }
        if (this.elements.fxPanel) {
            this.elements.fxPanel.style.display = this.state.activeTab === 'fx' ? 'block' : 'none';
        }
        // Hide tenor selector for FX (FX only has expiry, no tenor)
        const tenorContainer = document.getElementById('volcube-tenor-selector-container');
        if (tenorContainer) {
            tenorContainer.style.display = this.state.activeTab === 'fx' ? 'none' : 'block';
        }
    },

    async loadSwaptionIndices() {
        try {
            const response = await fetch('/api/volcube/indices');
            if (!response.ok) throw new Error('Failed to load indices');
            this.state.swaptionIndices = await response.json();
            this.renderIndexSelector();
            this.renderAssetTabs();
        } catch (error) {
            console.error('Failed to load swaption indices:', error);
        }
    },

    async loadSwaptionModels() {
        try {
            const response = await fetch('/api/volcube/models');
            if (!response.ok) throw new Error('Failed to load models');
            this.state.swaptionModels = await response.json();
            this.renderCalibSettings();
        } catch (error) {
            console.error('Failed to load calibration models:', error);
        }
    },

    async loadSwaptionInstruments(index) {
        try {
            const response = await fetch(`/api/volcube/instruments/${index}`);
            if (!response.ok) throw new Error('Failed to load instruments');
            const data = await response.json();
            this.state.swaptionInstruments = data.instruments || [];
            this.extractExpiriesAndTenors();
            this.renderInstrumentsTable();
        } catch (error) {
            console.error('Failed to load instruments:', error);
        }
    },

    async loadFxPairs() {
        try {
            const response = await fetch('/api/fxvol/pairs');
            if (!response.ok) throw new Error('Failed to load FX pairs');
            this.state.fxPairs = await response.json();
            this.renderFxPairSelector();
        } catch (error) {
            console.error('Failed to load FX pairs:', error);
        }
    },

    async loadDeltaTypes() {
        try {
            const response = await fetch('/api/fxvol/delta-types');
            if (!response.ok) throw new Error('Failed to load delta types');
            this.state.deltaTypes = await response.json();
        } catch (error) {
            console.error('Failed to load delta types:', error);
        }
    },

    async loadFxQuotes(pair) {
        try {
            const response = await fetch(`/api/fxvol/quotes/${pair}`);
            if (!response.ok) throw new Error('Failed to load quotes');
            const data = await response.json();
            this.state.fxQuotes = data.quotes || [];

            // Update input fields with data from API
            if (this.elements.fxSpot && data.spot) {
                this.elements.fxSpot.value = data.spot.toFixed(4);
            }
            if (this.elements.fxDomesticRate && data.domesticRate !== undefined) {
                this.elements.fxDomesticRate.value = (data.domesticRate * 100).toFixed(2);
            }
            if (this.elements.fxForeignRate && data.foreignRate !== undefined) {
                this.elements.fxForeignRate.value = (data.foreignRate * 100).toFixed(2);
            }

            this.renderFxQuotesTable();
        } catch (error) {
            console.error('Failed to load FX quotes:', error);
        }
    },

    extractExpiriesAndTenors() {
        const expiries = new Set();
        const tenors = new Set();

        this.state.swaptionInstruments.forEach(inst => {
            expiries.add(inst.expiry);
            tenors.add(inst.tenor);
        });

        this.state.expiries = Array.from(expiries).sort((a, b) => a - b);
        this.state.tenors = Array.from(tenors).sort((a, b) => a - b);

        this.renderSliceSelectors();
    },

    renderIndexSelector() {
        if (!this.elements.indexSelector) return;

        const indices = this.state.swaptionIndices.indices || this.state.swaptionIndices;
        const defaultIndex = 'usd'; // Currency-based selection

        this.elements.indexSelector.innerHTML = `
            <option value="">Select currency...</option>
            ${indices.map(idx => `
                <option value="${idx.id}" ${idx.id === defaultIndex ? 'selected' : ''}>${idx.name || idx.currency || idx.id.toUpperCase()}</option>
            `).join('')}
        `;

        // Auto-select default index and load instruments
        const hasDefault = indices.some(idx => idx.id === defaultIndex);
        if (hasDefault && !this.state.selectedSwaptionIndex) {
            this.state.selectedSwaptionIndex = defaultIndex;
            this.loadSwaptionInstruments(defaultIndex);
        }
    },

    renderFxPairSelector() {
        if (!this.elements.fxPairSelector) return;

        const pairs = this.state.fxPairs.pairs || this.state.fxPairs;
        const defaultPair = 'EURUSD';

        this.elements.fxPairSelector.innerHTML = `
            <option value="">Select pair...</option>
            ${pairs.map(p => `
                <option value="${p.pair || p.id || p}" ${(p.pair || p.id || p) === defaultPair ? 'selected' : ''}>${p.name || p}</option>
            `).join('')}
        `;

        // Auto-select default pair and load quotes
        const hasDefault = pairs.some(p => (p.pair || p.id || p) === defaultPair);
        if (hasDefault && !this.state.selectedFxPair) {
            this.state.selectedFxPair = defaultPair;
            this.loadFxQuotes(defaultPair);
        }
    },

    renderCalibSettings() {
        if (!this.elements.calibSettings) return;

        const models = this.state.swaptionModels.models || this.state.swaptionModels;

        this.elements.calibSettings.innerHTML = `
            <div class="volcube-input-section">
                <label for="volcube-model-selector">Model</label>
                <select id="volcube-model-selector" class="fancy-select">
                    ${models.map(m => `
                        <option value="${m.id}" ${m.recommended ? 'selected' : ''}>
                            ${m.name}${m.recommended ? ' (Recommended)' : ''}
                        </option>
                    `).join('')}
                </select>
            </div>
        `;
    },

    renderSliceSelectors() {
        if (this.elements.expirySelector) {
            this.elements.expirySelector.innerHTML = `
                ${this.state.expiries.map(e => `
                    <option value="${e}">${e}Y</option>
                `).join('')}
            `;
            this.state.selectedExpiry = this.state.expiries[0];
        }

        if (this.elements.tenorSelector) {
            this.elements.tenorSelector.innerHTML = `
                ${this.state.tenors.map(t => `
                    <option value="${t}">${t}Y</option>
                `).join('')}
            `;
            this.state.selectedTenor = this.state.tenors[0];
        }
    },

    renderInstrumentsTable() {
        if (!this.elements.instrumentsTable) return;

        if (this.state.swaptionInstruments.length === 0) {
            this.elements.instrumentsTable.innerHTML = '<p class="volcube-placeholder">Select an index to view instruments</p>';
            return;
        }

        // Group by expiry
        const grouped = {};
        this.state.swaptionInstruments.forEach(inst => {
            const key = inst.expiry;
            if (!grouped[key]) grouped[key] = [];
            grouped[key].push(inst);
        });

        const html = `
            <table class="volcube-table">
                <thead>
                    <tr>
                        <th>Expiry</th>
                        <th>Tenor</th>
                        <th>Strike</th>
                        <th>IV (%)</th>
                        <th>Forward</th>
                    </tr>
                </thead>
                <tbody>
                    ${this.state.swaptionInstruments.slice(0, 20).map(inst => `
                        <tr>
                            <td>${inst.expiry}Y</td>
                            <td>${inst.tenor}Y</td>
                            <td>${(inst.strike * 100).toFixed(2)}%</td>
                            <td>${(inst.impliedVol * 100).toFixed(2)}%</td>
                            <td>${(inst.forward * 100).toFixed(2)}%</td>
                        </tr>
                    `).join('')}
                </tbody>
            </table>
            ${this.state.swaptionInstruments.length > 20 ? `<p class="volcube-more">+ ${this.state.swaptionInstruments.length - 20} more instruments</p>` : ''}
        `;
        this.elements.instrumentsTable.innerHTML = html;
    },

    renderFxQuotesTable() {
        if (!this.elements.fxQuotesTable) return;

        if (this.state.fxQuotes.length === 0) {
            this.elements.fxQuotesTable.innerHTML = '<p class="volcube-placeholder">Select a pair to view quotes</p>';
            return;
        }

        const html = `
            <table class="volcube-table">
                <thead>
                    <tr>
                        <th>Tenor</th>
                        <th>ATM</th>
                        <th>25D RR</th>
                        <th>25D BF</th>
                        <th>10D RR</th>
                        <th>10D BF</th>
                    </tr>
                </thead>
                <tbody>
                    ${this.state.fxQuotes.map(q => `
                        <tr>
                            <td>${q.expiryLabel || '-'}</td>
                            <td>${((q.atmVol || 0) * 100).toFixed(2)}%</td>
                            <td>${((q.rr25d || 0) * 100).toFixed(2)}%</td>
                            <td>${((q.bf25d || 0) * 100).toFixed(2)}%</td>
                            <td>${((q.rr10d || 0) * 100).toFixed(2)}%</td>
                            <td>${((q.bf10d || 0) * 100).toFixed(2)}%</td>
                        </tr>
                    `).join('')}
                </tbody>
            </table>
        `;
        this.elements.fxQuotesTable.innerHTML = html;
    },

    async calibrate() {
        if (this.state.activeTab === 'swaption') {
            await this.calibrateSwaption();
        } else {
            await this.calibrateFx();
        }
    },

    async calibrateSwaption() {
        if (!this.state.selectedSwaptionIndex) {
            alert('Please select an index first');
            return;
        }

        const modelSelector = document.getElementById('volcube-model-selector');
        const model = modelSelector?.value || 'sabr';

        const request = {
            index: this.state.selectedSwaptionIndex,
            referenceDate: this.elements.referenceDate?.value || new Date().toISOString().split('T')[0],
            model,
            instruments: this.state.swaptionInstruments
        };

        try {
            if (this.elements.calibrateBtn) {
                this.elements.calibrateBtn.disabled = true;
                this.elements.calibrateBtn.innerHTML = '<i class="fas fa-spinner fa-spin"></i> Calibrating...';
            }

            const response = await fetch('/api/volcube/calibrate', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(request)
            });

            if (!response.ok) {
                const error = await response.json().catch(() => ({}));
                throw new Error(error.message || 'Calibration failed');
            }

            this.state.calibrationResult = await response.json();
            this.updateVisualization();

            if (typeof Logger !== 'undefined') {
                Logger.info('VolCubeBuilder', 'Calibration completed', { index: this.state.selectedSwaptionIndex });
            }
        } catch (error) {
            console.error('Calibration failed:', error);
            alert('Calibration failed: ' + error.message);
        } finally {
            if (this.elements.calibrateBtn) {
                this.elements.calibrateBtn.disabled = false;
                this.elements.calibrateBtn.innerHTML = '<i class="fas fa-play"></i> Calibrate';
            }
        }
    },

    async calibrateFx() {
        if (!this.state.selectedFxPair) {
            alert('Please select an FX pair first');
            return;
        }

        if (this.state.fxQuotes.length === 0) {
            alert('No FX quotes loaded');
            return;
        }

        const fxModel = this.elements.fxModelSelector?.value || 'linear';
        const spot = parseFloat(this.elements.fxSpot?.value || 1);
        const domesticRate = parseFloat(this.elements.fxDomesticRate?.value || 4.5) / 100;
        const foreignRate = parseFloat(this.elements.fxForeignRate?.value || 3.5) / 100;

        // Build API request
        const request = {
            currencyPair: this.state.selectedFxPair,
            referenceDate: new Date().toISOString().split('T')[0],
            spot,
            domesticRate,
            foreignRate,
            quotes: this.state.fxQuotes.map(q => ({
                expiry: q.expiry,
                atmVol: q.atmVol,
                rr25d: q.rr25d,
                bf25d: q.bf25d,
                rr10d: q.rr10d || null,
                bf10d: q.bf10d || null
            })),
            allowExtrapolation: true
        };

        try {
            if (this.elements.calibrateBtn) {
                this.elements.calibrateBtn.disabled = true;
                this.elements.calibrateBtn.innerHTML = '<i class="fas fa-spinner fa-spin"></i> Building...';
            }

            const response = await fetch('/api/fxvol/build', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(request)
            });

            if (!response.ok) {
                const error = await response.json().catch(() => ({}));
                throw new Error(error.message || 'Build failed');
            }

            const buildResult = await response.json();

            // Store surface_id and model for subsequent API calls
            this.state.fxSurfaceId = buildResult.surfaceId;
            this.state.calibrationResult = {
                type: 'fx',
                model: fxModel,
                surfaceId: buildResult.surfaceId,
                currencyPair: this.state.selectedFxPair,
                spot,
                domesticRate,
                foreignRate,
                expiryPoints: buildResult.expiryPoints || [],
                deltaPoints: buildResult.deltaPoints || [],
                processingTimeMs: buildResult.processingTimeMs,
                quotes: this.state.fxQuotes
            };

            await this.updateVisualization();

            if (typeof Logger !== 'undefined') {
                Logger.info('VolCubeBuilder', 'FX surface built', {
                    pair: this.state.selectedFxPair,
                    surfaceId: buildResult.surfaceId,
                    processingTime: buildResult.processingTimeMs
                });
            }
        } catch (error) {
            console.error('FX build failed:', error);
            alert('FX surface build failed: ' + error.message);
        } finally {
            if (this.elements.calibrateBtn) {
                this.elements.calibrateBtn.disabled = false;
                this.elements.calibrateBtn.innerHTML = '<i class="fas fa-play"></i> Calibrate';
            }
        }
    },

    async updateVisualization() {
        // Show results section, hide placeholder
        const placeholder = document.getElementById('volcube-results-placeholder');
        const swaptionResults = document.getElementById('volcube-results-content');
        const fxResults = document.getElementById('fxvol-results-content');

        if (placeholder) placeholder.style.display = 'none';

        if (this.state.activeTab === 'fx') {
            // Show FX results, hide swaption results
            if (swaptionResults) swaptionResults.style.display = 'none';
            if (fxResults) fxResults.style.display = 'block';

            // Fetch FX smile and density from API
            const surfaceId = this.state.fxSurfaceId;
            const result = this.state.calibrationResult;

            if (!surfaceId || !result?.expiryPoints?.length) {
                console.warn('[VolCubeBuilder] No FX surface ID or expiry points');
                return;
            }

            const expiry = result.expiryPoints[0];

            try {
                const [smileResponse, densityResponse] = await Promise.all([
                    fetch(`/api/fxvol/smile?surface_id=${surfaceId}&expiry=${expiry}&num_points=20`),
                    fetch(`/api/fxvol/density?surface_id=${surfaceId}&expiry=${expiry}&num_points=100`)
                ]);

                if (smileResponse.ok) {
                    this.renderFxSmileChart(await smileResponse.json());
                } else {
                    console.error('[VolCubeBuilder] FX smile API error:', smileResponse.status);
                }

                if (densityResponse.ok) {
                    this.renderFxDensityChart(await densityResponse.json());
                } else {
                    console.error('[VolCubeBuilder] FX density API error:', densityResponse.status);
                }
            } catch (error) {
                console.error('[VolCubeBuilder] FX API fetch failed:', error);
            }
        } else {
            // Swaption visualisation
            const result = this.state.calibrationResult;
            if (!result?.cubeId) return;

            // Show swaption results, hide FX results
            if (swaptionResults) swaptionResults.style.display = 'block';
            if (fxResults) fxResults.style.display = 'none';

            // Update SABR parameters display
            this.updateSabrParameters();

            // Update fit quality metrics
            this.updateFitMetrics();

            // Fetch smile and density from API
            const cubeId = result.cubeId;
            const expiry = this.state.selectedExpiry || this.state.expiries[0];
            const tenor = this.state.selectedTenor || this.state.tenors[0];

            if (cubeId && expiry && tenor) {
                try {
                    const [smileResponse, densityResponse, surfaceResponse] = await Promise.all([
                        fetch(`/api/volcube/smile?cube_id=${cubeId}&expiry=${expiry}&tenor=${tenor}&num_points=50`),
                        fetch(`/api/volcube/density?cube_id=${cubeId}&expiry=${expiry}&tenor=${tenor}&num_points=100`),
                        typeof Plotly !== 'undefined'
                            ? fetch(`/api/volcube/surface?cube_id=${cubeId}&tenor=${tenor}`)
                            : Promise.resolve(null)
                    ]);

                    if (smileResponse.ok) {
                        this.renderSmileChart(await smileResponse.json());
                    } else {
                        console.error('[VolCubeBuilder] Smile API error:', smileResponse.status);
                    }

                    if (densityResponse.ok) {
                        this.renderDensityChart(await densityResponse.json());
                    } else {
                        console.error('[VolCubeBuilder] Density API error:', densityResponse.status);
                    }

                    if (surfaceResponse?.ok && typeof Plotly !== 'undefined') {
                        this.render3DSurface(await surfaceResponse.json());
                    }
                } catch (error) {
                    console.error('[VolCubeBuilder] Swaption API fetch failed:', error);
                }
            }
        }
    },

    updateSabrParameters() {
        const result = this.state.calibrationResult;
        // Parameters is an array - get the first one (ATM point) for display
        const params = result?.parameters?.[0];
        if (!params) return;

        const setElement = (id, value) => {
            const el = document.getElementById(id);
            if (el) el.textContent = typeof value === 'number' ? value.toFixed(4) : '-';
        };

        setElement('volcube-sabr-alpha', params.alpha);
        setElement('volcube-sabr-beta', params.beta);
        setElement('volcube-sabr-rho', params.rho);
        setElement('volcube-sabr-nu', params.nu);
    },

    updateFitMetrics() {
        const result = this.state.calibrationResult;
        const metrics = result?.fitMetrics;
        if (!metrics) return;

        const setElement = (id, value, suffix = '') => {
            const el = document.getElementById(id);
            if (el) el.textContent = typeof value === 'number' ? value.toFixed(4) + suffix : '-';
        };

        setElement('volcube-rmse', metrics.rmse);
        setElement('volcube-max-error', metrics.maxError);
        setElement('volcube-r-squared', metrics.rSquared);
        setElement('volcube-calib-time', result.processingTimeMs, 'ms');
    },

    /**
     * Renders swaption smile chart using API data.
     * @param {Object} apiData - Response from /api/volcube/smile endpoint
     */
    renderSmileChart(apiData) {
        const canvas = document.getElementById('volcube-smile-chart');
        if (!canvas || typeof Chart === 'undefined') return;

        if (this.smileChart) {
            this.smileChart.destroy();
        }

        if (!apiData?.strikes || !apiData?.modelVols) {
            console.warn('[VolCubeBuilder] No smile API data available');
            return;
        }

        // Model curve from API
        const modelData = apiData.strikes.map((strike, i) => ({
            x: strike * 100,
            y: apiData.modelVols[i] * 100
        }));

        // Market points from API
        const marketData = (apiData.marketPoints || []).map(p => ({
            x: p.strike * 100,
            y: p.impliedVol * 100
        })).sort((a, b) => a.x - b.x);

        this.smileChart = new Chart(canvas, {
            type: 'line',
            data: {
                datasets: [
                    {
                        label: 'SABR Model',
                        data: modelData,
                        borderColor: 'rgb(99, 102, 241)',
                        backgroundColor: 'rgba(99, 102, 241, 0.1)',
                        borderWidth: 2,
                        fill: true,
                        tension: 0.4,
                        pointRadius: 0
                    },
                    {
                        label: 'Market',
                        data: marketData,
                        borderColor: 'rgb(16, 185, 129)',
                        backgroundColor: 'rgb(16, 185, 129)',
                        borderWidth: 0,
                        pointRadius: 6,
                        pointStyle: 'circle',
                        showLine: false
                    }
                ]
            },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                plugins: {
                    legend: {
                        display: true,
                        position: 'top',
                        labels: { color: '#8b8b9a', usePointStyle: true }
                    },
                    tooltip: {
                        backgroundColor: 'rgba(26, 26, 46, 0.9)',
                        titleColor: '#fff',
                        bodyColor: '#8b8b9a',
                        callbacks: {
                            label: ctx => `${ctx.dataset.label}: ${ctx.parsed.y.toFixed(2)}%`
                        }
                    }
                },
                scales: {
                    x: {
                        type: 'linear',
                        title: { display: true, text: 'Strike (%)', color: '#8b8b9a' },
                        grid: { color: 'rgba(255, 255, 255, 0.05)' },
                        ticks: { color: '#8b8b9a' }
                    },
                    y: {
                        title: { display: true, text: 'Implied Vol (%)', color: '#8b8b9a' },
                        grid: { color: 'rgba(255, 255, 255, 0.05)' },
                        ticks: { color: '#8b8b9a' }
                    }
                }
            }
        });
    },

    /**
     * Renders swaption density chart using API data.
     * @param {Object} apiData - Response from /api/volcube/density endpoint
     */
    renderDensityChart(apiData) {
        const canvas = document.getElementById('volcube-density-chart');
        if (!canvas || typeof Chart === 'undefined') return;

        if (this.densityChart) {
            this.densityChart.destroy();
        }

        if (!apiData?.strikes || !apiData?.densities) {
            console.warn('[VolCubeBuilder] No density API data available');
            return;
        }

        // Normalise densities for display
        const maxDensity = Math.max(...apiData.densities);
        const densityData = apiData.strikes.map((strike, i) => ({
            x: strike * 100,
            y: maxDensity > 0 ? apiData.densities[i] / maxDensity : 0
        }));

        this.densityChart = new Chart(canvas, {
            type: 'line',
            data: {
                datasets: [{
                    label: 'Risk-Neutral Density',
                    data: densityData,
                    borderColor: 'rgb(245, 158, 11)',
                    backgroundColor: 'rgba(245, 158, 11, 0.2)',
                    borderWidth: 2,
                    fill: true,
                    tension: 0.4,
                    pointRadius: 0
                }]
            },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                plugins: {
                    legend: {
                        display: true,
                        position: 'top',
                        labels: { color: '#8b8b9a' }
                    },
                    tooltip: {
                        backgroundColor: 'rgba(26, 26, 46, 0.9)',
                        titleColor: '#fff',
                        bodyColor: '#8b8b9a'
                    }
                },
                scales: {
                    x: {
                        type: 'linear',
                        title: { display: true, text: 'Rate (%)', color: '#8b8b9a' },
                        grid: { color: 'rgba(255, 255, 255, 0.05)' },
                        ticks: { color: '#8b8b9a' }
                    },
                    y: {
                        title: { display: true, text: 'Density (normalised)', color: '#8b8b9a' },
                        grid: { color: 'rgba(255, 255, 255, 0.05)' },
                        ticks: { color: '#8b8b9a' }
                    }
                }
            }
        });

        // Update density stats from API
        const statsEl = document.getElementById('volcube-density-stats');
        if (statsEl && apiData.statistics) {
            const stats = apiData.statistics;
            statsEl.innerHTML = `
                <div class="stat-item"><span class="stat-label">Mean</span><span class="stat-value">${(stats.mean * 100).toFixed(2)}%</span></div>
                <div class="stat-item"><span class="stat-label">Std Dev</span><span class="stat-value">${(Math.sqrt(stats.variance) * 100).toFixed(2)}%</span></div>
                <div class="stat-item"><span class="stat-label">Skewness</span><span class="stat-value">${stats.skewness.toFixed(3)}</span></div>
                <div class="stat-item"><span class="stat-label">Kurtosis</span><span class="stat-value">${stats.kurtosis.toFixed(3)}</span></div>
            `;
        }
    },

    /**
     * Renders FX smile chart using API data.
     * @param {Object} apiData - Response from /api/fxvol/smile endpoint
     */
    renderFxSmileChart(apiData) {
        const canvas = document.getElementById('fxvol-smile-chart');
        if (!canvas || typeof Chart === 'undefined') return;

        if (this.fxSmileChart) {
            this.fxSmileChart.destroy();
        }

        if (!apiData?.points || apiData.points.length === 0) {
            console.warn('[VolCubeBuilder] No FX smile API data available');
            return;
        }

        // Get selected model name
        const modelNames = {
            'linear': 'Linear',
            'vanna-volga': 'Vanna-Volga',
            'sabr': 'SABR',
            'svi': 'SVI',
            'polynomial': 'Polynomial'
        };
        const selectedModel = this.state.calibrationResult?.model || 'linear';
        const modelLabel = modelNames[selectedModel] || 'Model';

        // Market points from API
        const marketData = apiData.points.map(p => ({
            x: (p.delta > 0 ? p.delta : 1 + p.delta) * 100,
            y: p.volatility * 100,
            label: p.label
        })).sort((a, b) => a.x - b.x);

        // Model curve from API points using linear interpolation
        const modelData = [];
        const minDelta = Math.min(...marketData.map(d => d.x));
        const maxDelta = Math.max(...marketData.map(d => d.x));

        for (let d = minDelta; d <= maxDelta; d += 1) {
            const lower = marketData.filter(p => p.x <= d).pop();
            const upper = marketData.find(p => p.x >= d);
            let vol = apiData.atmVol * 100;

            if (lower && upper && lower !== upper) {
                const t = (d - lower.x) / (upper.x - lower.x);
                vol = lower.y + t * (upper.y - lower.y);
            } else if (lower) {
                vol = lower.y;
            } else if (upper) {
                vol = upper.y;
            }
            modelData.push({ x: d, y: vol });
        }

        this.fxSmileChart = new Chart(canvas, {
            type: 'line',
            data: {
                datasets: [
                    {
                        label: modelLabel,
                        data: modelData,
                        borderColor: 'rgb(99, 102, 241)',
                        backgroundColor: 'rgba(99, 102, 241, 0.1)',
                        borderWidth: 2,
                        fill: true,
                        tension: 0.4,
                        pointRadius: 0
                    },
                    {
                        label: 'Market',
                        data: marketData,
                        borderColor: 'rgb(16, 185, 129)',
                        backgroundColor: 'rgb(16, 185, 129)',
                        borderWidth: 0,
                        pointRadius: 6,
                        pointStyle: 'circle',
                        showLine: false
                    }
                ]
            },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                plugins: {
                    legend: {
                        display: true,
                        position: 'top',
                        labels: { color: '#8b8b9a', usePointStyle: true }
                    },
                    tooltip: {
                        backgroundColor: 'rgba(26, 26, 46, 0.9)',
                        titleColor: '#fff',
                        bodyColor: '#8b8b9a',
                        callbacks: {
                            label: ctx => `${ctx.dataset.label}: ${ctx.parsed.y.toFixed(2)}%`
                        }
                    }
                },
                scales: {
                    x: {
                        type: 'linear',
                        title: { display: true, text: 'Delta (%)', color: '#8b8b9a' },
                        grid: { color: 'rgba(255, 255, 255, 0.05)' },
                        ticks: { color: '#8b8b9a' },
                        min: 0,
                        max: 100
                    },
                    y: {
                        title: { display: true, text: 'Implied Vol (%)', color: '#8b8b9a' },
                        grid: { color: 'rgba(255, 255, 255, 0.05)' },
                        ticks: { color: '#8b8b9a' }
                    }
                }
            }
        });
    },

    /**
     * Renders FX density chart using API data.
     * @param {Object} apiData - Response from /api/fxvol/density endpoint
     */
    renderFxDensityChart(apiData) {
        const canvas = document.getElementById('fxvol-density-chart');
        if (!canvas || typeof Chart === 'undefined') return;

        if (this.fxDensityChart) {
            this.fxDensityChart.destroy();
        }

        if (!apiData?.strikes || !apiData?.densities || apiData.densities.length === 0) {
            console.warn('[VolCubeBuilder] No FX density API data available');
            return;
        }

        // Normalise densities for display
        const maxDensity = Math.max(...apiData.densities);
        const densityData = apiData.strikes.map((strike, i) => ({
            x: strike,
            y: maxDensity > 0 ? apiData.densities[i] / maxDensity : 0
        }));

        this.fxDensityChart = new Chart(canvas, {
            type: 'line',
            data: {
                datasets: [{
                    label: 'Risk-Neutral Density',
                    data: densityData,
                    borderColor: 'rgb(245, 158, 11)',
                    backgroundColor: 'rgba(245, 158, 11, 0.2)',
                    borderWidth: 2,
                    fill: true,
                    tension: 0.4,
                    pointRadius: 0
                }]
            },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                plugins: {
                    legend: {
                        display: true,
                        position: 'top',
                        labels: { color: '#8b8b9a' }
                    },
                    tooltip: {
                        backgroundColor: 'rgba(26, 26, 46, 0.9)',
                        titleColor: '#fff',
                        bodyColor: '#8b8b9a'
                    }
                },
                scales: {
                    x: {
                        type: 'linear',
                        title: { display: true, text: 'Spot Rate', color: '#8b8b9a' },
                        grid: { color: 'rgba(255, 255, 255, 0.05)' },
                        ticks: { color: '#8b8b9a' }
                    },
                    y: {
                        title: { display: true, text: 'Density (normalised)', color: '#8b8b9a' },
                        grid: { color: 'rgba(255, 255, 255, 0.05)' },
                        ticks: { color: '#8b8b9a' }
                    }
                }
            }
        });

        // Update FX density stats from API
        const statsEl = document.getElementById('fxvol-density-stats');
        if (statsEl && apiData.statistics) {
            const stats = apiData.statistics;
            statsEl.innerHTML = `
                <div class="stat-item"><span class="stat-label">Forward</span><span class="stat-value">${apiData.forward.toFixed(4)}</span></div>
                <div class="stat-item"><span class="stat-label">Mean</span><span class="stat-value">${stats.mean.toFixed(4)}</span></div>
                <div class="stat-item"><span class="stat-label">Std Dev</span><span class="stat-value">${Math.sqrt(stats.variance).toFixed(4)}</span></div>
                <div class="stat-item"><span class="stat-label">Skewness</span><span class="stat-value">${stats.skewness?.toFixed(3) || '-'}</span></div>
                <div class="stat-item"><span class="stat-label">Kurtosis</span><span class="stat-value">${stats.kurtosis?.toFixed(3) || '-'}</span></div>
            `;
        }
    },

    /**
     * Renders 3D volatility surface using API data.
     * @param {Object} apiData - Response from /api/volcube/surface endpoint
     */
    render3DSurface(apiData) {
        const container = document.getElementById('volcube-surface-3d');
        if (!container || typeof Plotly === 'undefined') return;

        if (!apiData?.expiries || !apiData?.strikes || !apiData?.volatilities) {
            console.warn('[VolCubeBuilder] No 3D surface API data available');
            return;
        }

        // Convert volatilities to percentages
        const z = apiData.volatilities.map(row => row.map(v => v * 100));

        const data = [{
            type: 'surface',
            x: apiData.strikes.map(s => (s * 100).toFixed(2)),
            y: apiData.expiries.map(e => e + 'Y'),
            z: z,
            colorscale: 'Viridis',
            showscale: true,
            colorbar: { title: 'IV (%)' }
        }];

        // Add market points as scatter3d if available
        if (apiData.marketPoints?.length > 0) {
            data.push({
                type: 'scatter3d',
                mode: 'markers',
                x: apiData.marketPoints.map(p => (p.strike * 100).toFixed(2)),
                y: apiData.marketPoints.map(p => p.expiry + 'Y'),
                z: apiData.marketPoints.map(p => p.impliedVol * 100),
                marker: {
                    size: 4,
                    color: 'rgb(16, 185, 129)',
                    opacity: 0.8
                },
                name: 'Market'
            });
        }

        const layout = {
            title: {
                text: `Volatility Surface (${apiData.tenor}Y Tenor)`,
                font: { color: '#e0e0e0' }
            },
            paper_bgcolor: 'rgba(26, 26, 46, 0)',
            plot_bgcolor: 'rgba(26, 26, 46, 0)',
            scene: {
                xaxis: {
                    title: { text: 'Strike (%)', font: { color: '#8b8b9a' } },
                    tickfont: { color: '#8b8b9a' },
                    gridcolor: 'rgba(255, 255, 255, 0.1)',
                    zerolinecolor: 'rgba(255, 255, 255, 0.2)'
                },
                yaxis: {
                    title: { text: 'Expiry', font: { color: '#8b8b9a' } },
                    tickfont: { color: '#8b8b9a' },
                    gridcolor: 'rgba(255, 255, 255, 0.1)',
                    zerolinecolor: 'rgba(255, 255, 255, 0.2)'
                },
                zaxis: {
                    title: { text: 'IV (%)', font: { color: '#8b8b9a' } },
                    tickfont: { color: '#8b8b9a' },
                    gridcolor: 'rgba(255, 255, 255, 0.1)',
                    zerolinecolor: 'rgba(255, 255, 255, 0.2)'
                },
                bgcolor: 'rgba(26, 26, 46, 1)'
            },
            margin: { l: 0, r: 0, b: 0, t: 40 }
        };

        Plotly.newPlot(container, data, layout, { responsive: true });
    },

    exportCsv() {
        if (!this.state.calibrationResult) return;

        const result = this.state.calibrationResult;
        let csv = '';

        if (result.parameters) {
            csv = Object.keys(result.parameters[0] || {}).join(',') + '\n';
            csv += result.parameters.map(p => Object.values(p).join(',')).join('\n');
        }

        this.downloadFile(csv, 'volcube_calibration.csv', 'text/csv');
    },

    exportJson() {
        if (!this.state.calibrationResult) return;
        this.downloadFile(
            JSON.stringify(this.state.calibrationResult, null, 2),
            'volcube_calibration.json',
            'application/json'
        );
    },

    downloadFile(content, filename, mimeType) {
        const blob = new Blob([content], { type: mimeType });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = filename;
        a.click();
        URL.revokeObjectURL(url);
    }
};

// Auto-initialise when DOM is ready
(function() {
    function setupVolcubeBuilder() {
        window.addEventListener('viewChanged', (e) => {
            if (e.detail?.view === 'model-calib') {
                volcubeBuilder.init();
            }
        });

        if (document.getElementById('model-calib-view')?.classList.contains('active')) {
            volcubeBuilder.init();
        }
    }

    // Handle both cases: DOM already ready or still loading
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', setupVolcubeBuilder);
    } else {
        setupVolcubeBuilder();
    }
})();
