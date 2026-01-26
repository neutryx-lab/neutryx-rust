/**
 * Curve Builder Module
 * Handles yield curve construction from market instruments
 */

const curveBuilder = {
    state: {
        indices: [],
        selectedIndex: null,
        instruments: [],
        originalInstruments: [],
        builders: null,
        buildResult: null,
        hasChanges: false
    },

    /**
     * Converts decimal years to human-readable tenor format.
     * @param {number} years - Tenor in decimal years (e.g., 0.0833 for 1M)
     * @returns {string} Human-readable tenor (e.g., "1M", "3M", "1Y")
     */
    formatTenor(years) {
        if (years == null || isNaN(years)) return '-';

        // Handle zero and near-zero values
        if (Math.abs(years) < 0.001) return '0';

        // Handle exact year values
        if (years >= 1 && Math.abs(years - Math.round(years)) < 0.001) {
            return `${Math.round(years)}Y`;
        }

        // Convert to weeks for very short tenors (up to 4 weeks)
        const weeks = years * 52;
        const roundedWeeks = Math.round(weeks);
        if (roundedWeeks >= 1 && roundedWeeks <= 4 && Math.abs(weeks - roundedWeeks) < 0.5) {
            return `${roundedWeeks}W`;
        }

        // Convert to months
        const months = years * 12;
        const roundedMonths = Math.round(months);

        // Check if it's a clean month value (within 0.1 month tolerance)
        if (roundedMonths > 0 && Math.abs(months - roundedMonths) < 0.1) {
            if (roundedMonths >= 12 && roundedMonths % 12 === 0) {
                return `${roundedMonths / 12}Y`;
            }
            return `${roundedMonths}M`;
        }

        // Convert to days for overnight/short tenors
        const days = years * 365;
        if (days < 7 && days > 0) {
            const roundedDays = Math.round(days);
            if (roundedDays === 1) return 'O/N';
            if (roundedDays > 0) return `${roundedDays}D`;
        }

        // Fallback: show years with reasonable precision
        if (years < 1) {
            return `${(years * 12).toFixed(1)}M`;
        }
        return `${years.toFixed(2)}Y`;
    },

    elements: {
        indexContainer: null,
        instrumentTable: null,
        settingsContainer: null,
        buildBtn: null,
        exportRatesBtn: null,
        importRatesBtn: null,
        resetRatesBtn: null,
        changesIndicator: null,
        rebuildNotification: null,
        buildStatus: null,
        buildSummary: null,
        parameterChartShort: null,
        parameterChartLong: null,
        chartPlaceholder: null,
        parameterTabsContainer: null,
        parameterTableContainer: null,
        errorContainer: null,
        errorMessage: null,
        loadingOverlay: null
    },

    // Central bank meeting dates cache
    centralBankMeetings: null,

    initialized: false,

    async init() {
        if (this.initialized) return;

        this.cacheElements();

        // Debug: check if elements were found
        console.log('[CurveBuilder] Elements cached:', {
            indexContainer: !!this.elements.indexContainer,
            instrumentTable: !!this.elements.instrumentTable,
            settingsContainer: !!this.elements.settingsContainer,
            buildBtn: !!this.elements.buildBtn
        });

        this.attachEventListeners();

        try {
            await this.loadIndices();
            await this.loadBuilders();
            await this.loadCentralBankMeetings();
            this.initialized = true;

            if (typeof Logger !== 'undefined') {
                Logger.info('CurveBuilder', 'Curve builder module initialised');
            }
        } catch (error) {
            console.error('[CurveBuilder] Init failed:', error);
        }
    },

    /**
     * Loads central bank meeting dates for all currencies.
     */
    async loadCentralBankMeetings() {
        try {
            const response = await fetch('/api/curves/central-bank-meetings');
            if (response.ok) {
                this.centralBankMeetings = await response.json();
            }
        } catch (error) {
            console.warn('[CurveBuilder] Failed to load central bank meetings:', error);
            // Non-critical, continue without meetings
        }
    },

    /**
     * Gets central bank meeting dates for the current currency within a year range.
     * @param {string} currency - Currency code (USD, EUR, JPY, etc.)
     * @param {Date} referenceDate - Reference date for calculations
     * @param {number} maxYears - Maximum years to look ahead (default 1)
     * @returns {Array} Array of {yearFraction, date, label} objects
     */
    getCentralBankMeetingsInRange(currency, referenceDate, maxYears = 1) {
        if (!this.centralBankMeetings?.meetings?.[currency]) {
            return [];
        }

        const meetings = [];
        const refDate = new Date(referenceDate);
        const maxDate = new Date(refDate);
        maxDate.setFullYear(maxDate.getFullYear() + maxYears);

        for (const dateStr of this.centralBankMeetings.meetings[currency].dates) {
            const meetingDate = new Date(dateStr);
            if (meetingDate > refDate && meetingDate <= maxDate) {
                const diffMs = meetingDate - refDate;
                const yearFraction = diffMs / (365.25 * 24 * 60 * 60 * 1000);
                meetings.push({
                    yearFraction,
                    date: meetingDate,
                    label: `${this.centralBankMeetings.meetings[currency].centralBank} (${dateStr})`
                });
            }
        }

        return meetings.sort((a, b) => a.yearFraction - b.yearFraction);
    },

    cacheElements() {
        this.elements.indexContainer = document.getElementById('index-selector-container');
        this.elements.instrumentTable = document.getElementById('instrument-table-container');
        this.elements.settingsContainer = document.getElementById('builder-settings-container');
        this.elements.buildBtn = document.getElementById('build-curve-btn');
        this.elements.exportRatesBtn = document.getElementById('export-rates-btn');
        this.elements.importRatesBtn = document.getElementById('import-rates-btn');
        this.elements.resetRatesBtn = document.getElementById('reset-rates-btn');
        this.elements.changesIndicator = document.getElementById('changes-indicator');
        this.elements.rebuildNotification = document.getElementById('rebuild-notification');
        this.elements.buildStatus = document.getElementById('build-status');
        this.elements.buildSummary = document.getElementById('build-summary');
        this.elements.parameterChartShort = document.getElementById('parameter-chart-short');
        this.elements.parameterChartLong = document.getElementById('parameter-chart-long');
        this.elements.chartPlaceholder = document.getElementById('chart-placeholder');
        this.elements.parameterTabsContainer = document.getElementById('parameter-tabs-container');
        this.elements.parameterTableContainer = document.getElementById('parameter-table-container');
        this.elements.errorContainer = document.getElementById('curve-builder-error');
        this.elements.errorMessage = document.getElementById('curve-builder-error-message');
        this.elements.loadingOverlay = document.getElementById('curve-builder-loading');
        this.elements.exportCsvBtn = document.getElementById('export-csv-btn');
        this.elements.exportJsonBtn = document.getElementById('export-json-btn');
    },

    attachEventListeners() {
        if (this.elements.buildBtn) {
            this.elements.buildBtn.addEventListener('click', () => this.buildCurve());
        }
        if (this.elements.exportRatesBtn) {
            this.elements.exportRatesBtn.addEventListener('click', () => this.exportRates());
        }
        if (this.elements.importRatesBtn) {
            this.elements.importRatesBtn.addEventListener('click', () => this.importRates());
        }
        if (this.elements.resetRatesBtn) {
            this.elements.resetRatesBtn.addEventListener('click', () => this.resetRates());
        }
        if (this.elements.exportCsvBtn) {
            this.elements.exportCsvBtn.addEventListener('click', () => this.exportCsv());
        }
        if (this.elements.exportJsonBtn) {
            this.elements.exportJsonBtn.addEventListener('click', () => this.exportJson());
        }
    },

    async loadIndices() {
        try {
            console.log('[CurveBuilder] Loading indices...');
            const response = await fetch('/api/curves/indices');
            if (!response.ok) throw new Error('Failed to load indices');
            this.state.indices = await response.json();
            console.log('[CurveBuilder] Loaded indices:', this.state.indices);
            this.renderIndexSelector();
        } catch (error) {
            console.error('[CurveBuilder] loadIndices error:', error);
            this.showError('Failed to load curve indices: ' + error.message);
        }
    },

    async loadBuilders() {
        try {
            const response = await fetch('/api/curves/builders');
            if (!response.ok) throw new Error('Failed to load builders');
            this.state.builders = await response.json();
            this.renderSettings();
        } catch (error) {
            this.showError('Failed to load builder settings: ' + error.message);
        }
    },

    async loadInstruments(index) {
        console.log('[CurveBuilder] loadInstruments called with index:', index);
        try {
            this.showLoading('Loading instruments...');
            const url = `/api/curves/instruments/${index}`;
            console.log('[CurveBuilder] Fetching:', url);
            const response = await fetch(url);
            console.log('[CurveBuilder] Response status:', response.status, response.ok);
            if (!response.ok) {
                const errorText = await response.text();
                console.error('[CurveBuilder] API error response:', errorText);
                throw new Error('Failed to load instruments: ' + response.status);
            }
            const data = await response.json();
            console.log('[CurveBuilder] Loaded data:', data);
            this.state.instruments = data.instruments || [];
            console.log('[CurveBuilder] Instruments count:', this.state.instruments.length);
            this.state.originalInstruments = JSON.parse(JSON.stringify(this.state.instruments));
            this.state.hasChanges = false;
            this.updateChangesIndicator();
            this.renderInstrumentTable();
        } catch (error) {
            console.error('[CurveBuilder] loadInstruments error:', error);
            this.showError('Failed to load instruments: ' + error.message);
        } finally {
            this.hideLoading();
        }
    },

    renderIndexSelector() {
        console.log('[CurveBuilder] renderIndexSelector called');

        // Re-cache the element in case DOM changed
        const container = document.getElementById('index-selector-container');
        console.log('[CurveBuilder] Container found:', !!container, container);

        if (!container) {
            console.warn('[CurveBuilder] index-selector-container not found in DOM!');
            return;
        }

        console.log('[CurveBuilder] Indices count:', this.state.indices.length, 'Data:', this.state.indices);

        // Default to USD-SOFR if available
        const defaultIndex = this.state.indices.find(idx =>
            idx.toLowerCase() === 'usd-sofr'
        ) || '';

        const optionsHtml = this.state.indices.map(idx =>
            `<option value="${idx}" ${idx === defaultIndex ? 'selected' : ''}>${idx.toUpperCase()}</option>`
        ).join('');

        const html = `
            <label for="curve-index-selector" class="sr-only">Select Rate Index</label>
            <select id="curve-index-selector" class="fancy-select" style="min-width: 200px;">
                ${defaultIndex ? '' : '<option value="">Select index...</option>'}
                ${optionsHtml}
            </select>
            <span style="color: #10b981; margin-left: 8px; font-size: 12px;">(${this.state.indices.length} indices loaded)</span>
        `;

        console.log('[CurveBuilder] Setting innerHTML:', html);
        container.innerHTML = html;
        console.log('[CurveBuilder] Container innerHTML after set:', container.innerHTML);

        // Verify DOM was updated
        const selectEl = document.getElementById('curve-index-selector');
        console.log('[CurveBuilder] Select element created:', !!selectEl, 'Options count:', selectEl?.options?.length);

        // Check again after a delay to see if something overwrites the content
        setTimeout(() => {
            const containerAfter = document.getElementById('index-selector-container');
            const selectAfter = document.getElementById('curve-index-selector');
            console.log('[CurveBuilder] Delayed check - container innerHTML:', containerAfter?.innerHTML?.substring(0, 100));
            console.log('[CurveBuilder] Delayed check - select exists:', !!selectAfter, 'options:', selectAfter?.options?.length);
        }, 500);

        // Update cached reference
        this.elements.indexContainer = container;

        const selector = document.getElementById('curve-index-selector');
        if (selector) {
            selector.addEventListener('change', (e) => {
                this.state.selectedIndex = e.target.value;
                if (this.state.selectedIndex) {
                    this.loadInstruments(this.state.selectedIndex);
                }
            });

            // Auto-load instruments for default index (USD-SOFR)
            if (defaultIndex && !this.state.selectedIndex) {
                this.state.selectedIndex = defaultIndex;
                this.loadInstruments(defaultIndex);
            }
        }
    },

    renderSettings() {
        if (!this.elements.settingsContainer || !this.state.builders) return;

        const { interpolationMethods, bootstrapMethods } = this.state.builders;

        const html = `
            <div class="settings-row">
                <div class="setting-group">
                    <label for="interpolation-method">Interpolation</label>
                    <select id="interpolation-method" class="fancy-select">
                        ${interpolationMethods.map(m => `
                            <option value="${m.id}" ${m.recommended ? 'selected' : ''}>
                                ${m.name}${m.recommended ? ' (Recommended)' : ''}
                            </option>
                        `).join('')}
                    </select>
                </div>
                <div class="setting-group">
                    <label for="bootstrap-method">Bootstrap</label>
                    <select id="bootstrap-method" class="fancy-select">
                        ${bootstrapMethods.map(m => `
                            <option value="${m.id}" ${!m.enabled ? 'disabled' : ''}>
                                ${m.name}
                            </option>
                        `).join('')}
                    </select>
                </div>
            </div>
        `;
        this.elements.settingsContainer.innerHTML = html;

        // Add change listeners for rebuild notification
        ['interpolation-method', 'bootstrap-method'].forEach(id => {
            const el = document.getElementById(id);
            if (el) {
                el.addEventListener('change', () => this.showRebuildNotification());
            }
        });
    },

    renderInstrumentTable() {
        if (!this.elements.instrumentTable) return;

        if (this.state.instruments.length === 0) {
            this.elements.instrumentTable.innerHTML = '<p class="placeholder-text">Select an index to view instruments</p>';
            return;
        }

        const html = `
            <table class="instrument-table">
                <thead>
                    <tr>
                        <th>Type</th>
                        <th>Tenor</th>
                        <th>Rate (%)</th>
                        <th>Frequency</th>
                    </tr>
                </thead>
                <tbody>
                    ${this.state.instruments.map((inst, idx) => `
                        <tr>
                            <td>${inst.instrumentType}</td>
                            <td>${inst.tenor}</td>
                            <td>
                                <input type="number"
                                       class="rate-input"
                                       data-index="${idx}"
                                       value="${(inst.rate * 100).toFixed(4)}"
                                       step="0.0001">
                            </td>
                            <td>${inst.frequency || '-'}</td>
                        </tr>
                    `).join('')}
                </tbody>
            </table>
        `;
        this.elements.instrumentTable.innerHTML = html;

        // Add change listeners to rate inputs
        this.elements.instrumentTable.querySelectorAll('.rate-input').forEach(input => {
            input.addEventListener('change', (e) => {
                const idx = parseInt(e.target.dataset.index);
                this.state.instruments[idx].rate = parseFloat(e.target.value) / 100;
                this.state.hasChanges = true;
                this.updateChangesIndicator();
                this.showRebuildNotification();
            });
        });
    },

    updateChangesIndicator() {
        if (this.elements.changesIndicator) {
            this.elements.changesIndicator.style.display = this.state.hasChanges ? 'flex' : 'none';
        }
    },

    showRebuildNotification() {
        if (this.elements.rebuildNotification && this.state.buildResult) {
            this.elements.rebuildNotification.style.display = 'flex';
        }
    },

    hideRebuildNotification() {
        if (this.elements.rebuildNotification) {
            this.elements.rebuildNotification.style.display = 'none';
        }
    },

    async buildCurve() {
        if (!this.state.selectedIndex || this.state.instruments.length === 0) {
            this.showError('Please select an index first');
            return;
        }

        const interpolationMethod = document.getElementById('interpolation-method')?.value || 'linear_on_log_df';
        const bootstrapMethod = document.getElementById('bootstrap-method')?.value || 'sequential';

        const request = {
            index: this.state.selectedIndex,
            instruments: this.state.instruments.map(inst => ({
                instrumentType: inst.instrumentType,
                tenor: inst.tenor,
                tenorYears: inst.tenorYears,
                rate: inst.rate,
                frequency: inst.frequency
            })),
            interpolationMethod,
            bootstrapMethod
        };

        try {
            this.showLoading('Building curve...');
            this.hideError();

            const response = await fetch('/api/curves/build', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(request)
            });

            if (!response.ok) {
                const errorData = await response.json().catch(() => ({}));
                throw new Error(errorData.message || 'Build failed');
            }

            this.state.buildResult = await response.json();
            this.hideRebuildNotification();
            this.renderBuildResult();
            this.renderParameterCurve();

            if (typeof Logger !== 'undefined') {
                Logger.info('CurveBuilder', 'Curve built successfully', { index: this.state.selectedIndex });
            }
        } catch (error) {
            this.showError('Build failed: ' + error.message);
        } finally {
            this.hideLoading();
        }
    },

    renderBuildResult() {
        if (!this.state.buildResult) return;

        const result = this.state.buildResult;

        // Update status
        if (this.elements.buildStatus) {
            this.elements.buildStatus.innerHTML = `
                <span class="status-dot success"></span>
                <span>Built</span>
            `;
        }

        // Update summary
        if (this.elements.buildSummary) {
            this.elements.buildSummary.style.display = 'block';
            this.elements.buildSummary.innerHTML = `
                <div class="build-summary-grid">
                    <div class="summary-item">
                        <span class="summary-label">Index</span>
                        <span class="summary-value">${result.index?.toUpperCase() || '-'}</span>
                    </div>
                    <div class="summary-item">
                        <span class="summary-label">Points</span>
                        <span class="summary-value">${result.parameters?.length || 0}</span>
                    </div>
                    <div class="summary-item">
                        <span class="summary-label">Method</span>
                        <span class="summary-value">${result.interpolationMethod || '-'}</span>
                    </div>
                    <div class="summary-item">
                        <span class="summary-label">Build Time</span>
                        <span class="summary-value">${result.buildTimeMs || 0}ms</span>
                    </div>
                </div>
            `;
        }
    },

    /**
     * Generates curve data points with specified granularity.
     * @param {Array} params - Original curve parameters (pillar points)
     * @param {number} startYear - Start of range in years
     * @param {number} endYear - End of range in years
     * @param {string} granularity - 'daily', 'weekly', or 'monthly'
     * @returns {Array} Generated data points
     */
    generateCurveData(params, startYear, endYear, granularity) {
        const data = [];
        let step;

        switch (granularity) {
            case 'daily':
                step = 1 / 365; // 1 day
                break;
            case 'weekly':
                step = 1 / 52; // 1 week
                break;
            case 'monthly':
            default:
                step = 1 / 12; // 1 month
                break;
        }

        // Sort params by tenor
        const sortedParams = [...params].sort((a, b) => a.tenorYears - b.tenorYears);

        for (let t = startYear; t <= endYear + step / 2; t += step) {
            const point = this.interpolateAtTenor(sortedParams, t);
            data.push(point);
        }

        return data;
    },

    /**
     * Interpolates curve values at a given tenor using log-linear interpolation.
     * @param {Array} params - Sorted curve parameters
     * @param {number} t - Tenor in years
     * @returns {Object} Interpolated point
     */
    interpolateAtTenor(params, t) {
        if (params.length === 0) {
            return { tenorYears: t, discountFactor: 1, zeroRate: 0, forwardRate: 0 };
        }

        // Handle edge cases
        if (t <= 0) {
            return { tenorYears: t, discountFactor: 1, zeroRate: params[0]?.zeroRate || 0, forwardRate: params[0]?.zeroRate || 0 };
        }

        if (t <= params[0].tenorYears) {
            // Extrapolate from first point
            const logDf = Math.log(params[0].discountFactor) * t / params[0].tenorYears;
            const df = Math.exp(logDf);
            const zr = -logDf / t;
            return { tenorYears: t, discountFactor: df, zeroRate: zr, forwardRate: zr };
        }

        if (t >= params[params.length - 1].tenorYears) {
            // Extrapolate from last two points
            const n = params.length;
            const logDfLast = Math.log(params[n - 1].discountFactor);
            const logDfPrev = Math.log(params[n - 2].discountFactor);
            const slope = (logDfLast - logDfPrev) / (params[n - 1].tenorYears - params[n - 2].tenorYears);
            const logDf = logDfLast + slope * (t - params[n - 1].tenorYears);
            const df = Math.exp(logDf);
            const zr = -logDf / t;
            return { tenorYears: t, discountFactor: df, zeroRate: zr, forwardRate: zr };
        }

        // Find bracketing points and interpolate
        for (let i = 1; i < params.length; i++) {
            if (t <= params[i].tenorYears) {
                const t0 = params[i - 1].tenorYears;
                const t1 = params[i].tenorYears;
                const logDf0 = Math.log(params[i - 1].discountFactor);
                const logDf1 = Math.log(params[i].discountFactor);

                // Log-linear interpolation
                const w = (t - t0) / (t1 - t0);
                const logDf = logDf0 + w * (logDf1 - logDf0);
                const df = Math.exp(logDf);
                const zr = -logDf / t;

                // Forward rate
                const fr = (logDf0 - logDf1) / (t1 - t0);

                return { tenorYears: t, discountFactor: df, zeroRate: zr, forwardRate: fr };
            }
        }

        return { tenorYears: t, discountFactor: 1, zeroRate: 0, forwardRate: 0 };
    },

    /**
     * Generates short-term curve data at specific tenor points.
     * X-axis: 0, 1W, 2W, 3W, 1M, 2M, 3M, 4M, 5M, 6M, 7M, 8M, 9M, 10M, 11M, 1Y
     * @param {Array} params - Original curve parameters
     * @returns {Array} Data points at specific tenors
     */
    generateShortTermData(params) {
        // Specific tenor points in years: 0, 1W, 2W, 3W, then 1M to 12M
        const tenorPoints = [
            0,           // 0
            1/52,        // 1W
            2/52,        // 2W
            3/52,        // 3W
            1/12,        // 1M
            2/12,        // 2M
            3/12,        // 3M
            4/12,        // 4M
            5/12,        // 5M
            6/12,        // 6M
            7/12,        // 7M
            8/12,        // 8M
            9/12,        // 9M
            10/12,       // 10M
            11/12,       // 11M
            1.0          // 1Y
        ];

        const sortedParams = [...params].sort((a, b) => a.tenorYears - b.tenorYears);
        return tenorPoints.map(t => this.interpolateAtTenor(sortedParams, t));
    },

    /**
     * Generates long-term curve data at yearly intervals.
     * X-axis: 0, 1Y, 2Y, 3Y, ... up to max tenor (default 30Y)
     * @param {Array} params - Original curve parameters
     * @returns {Array} Data points at yearly intervals
     */
    generateLongTermData(params) {
        // Get max tenor from params, default to 30
        const maxTenor = Math.max(...params.map(p => p.tenorYears), 30);
        const maxYear = Math.ceil(maxTenor);

        // Generate yearly points: 0, 1, 2, ... maxYear
        const tenorPoints = [];
        for (let y = 0; y <= maxYear; y++) {
            tenorPoints.push(y);
        }

        const sortedParams = [...params].sort((a, b) => a.tenorYears - b.tenorYears);
        return tenorPoints.map(t => this.interpolateAtTenor(sortedParams, t));
    },

    renderParameterCurve() {
        if (!this.state.buildResult?.parameters) return;

        const params = this.state.buildResult.parameters;

        // Hide placeholder, show charts
        if (this.elements.chartPlaceholder) {
            this.elements.chartPlaceholder.style.display = 'none';
        }

        // Show chart containers - ensure both are visible
        const shortContainer = this.elements.parameterChartShort?.closest('.parameter-chart-container');
        const longContainer = this.elements.parameterChartLong?.closest('.parameter-chart-container');

        console.log('[CurveBuilder] Chart containers:', { shortContainer, longContainer });

        if (shortContainer) {
            shortContainer.classList.add('visible');
            shortContainer.style.display = 'block';
        }
        if (longContainer) {
            longContainer.classList.add('visible');
            longContainer.style.display = 'block';
        }

        if (this.elements.parameterChartShort) {
            this.elements.parameterChartShort.style.display = 'block';
        }
        if (this.elements.parameterChartLong) {
            this.elements.parameterChartLong.style.display = 'block';
        }

        // Render tabs
        this.renderParameterTabs();

        // Generate data for both charts
        const shortTermData = this.generateShortTermData(params);
        const longTermData = this.generateLongTermData(params);

        console.log('[CurveBuilder] Generated data:', {
            shortTermPoints: shortTermData.length,
            longTermPoints: longTermData.length
        });

        // Get currency from index (e.g., "usd-sofr" -> "USD")
        const currency = this.state.selectedIndex?.split('-')[0]?.toUpperCase() || 'USD';
        const referenceDate = new Date(); // Use current date as reference

        // Get central bank meetings for short-term chart
        const cbMeetings = this.getCentralBankMeetingsInRange(currency, referenceDate, 1);

        // Render both charts
        if (typeof Chart !== 'undefined') {
            try {
                this.renderShortTermChart(shortTermData, cbMeetings);
                console.log('[CurveBuilder] Short-term chart rendered');
            } catch (e) {
                console.error('[CurveBuilder] Error rendering short-term chart:', e);
            }

            try {
                this.renderLongTermChart(longTermData);
                console.log('[CurveBuilder] Long-term chart rendered');
            } catch (e) {
                console.error('[CurveBuilder] Error rendering long-term chart:', e);
            }

            // Force resize after render to ensure proper fit
            setTimeout(() => {
                if (this.chartShort) this.chartShort.resize();
                if (this.chartLong) this.chartLong.resize();
            }, 50);
        }

        // Render table with original pillar data
        this.renderParameterTable(params);
        this.updateChartVisibility('fwd');
    },

    /**
     * Renders the short-term curve chart (0-1Y).
     */
    renderShortTermChart(data, cbMeetings = []) {
        if (!this.elements.parameterChartShort) return;

        const ctx = this.elements.parameterChartShort.getContext('2d');

        // Destroy existing chart
        if (this.chartShort) {
            this.chartShort.destroy();
        }

        const tenors = data.map(p => this.formatTenor(p.tenorYears));
        const zeroRates = data.map(p => (p.zeroRate * 100));
        const discountFactors = data.map(p => p.discountFactor);
        const forwardRates = data.map(p => ((p.forwardRate || 0) * 100));

        // Create vertical line annotations for CB meetings
        const annotations = {};
        cbMeetings.forEach((meeting, idx) => {
            // Find the closest data point index
            let closestIdx = 0;
            let minDiff = Infinity;
            data.forEach((d, i) => {
                const diff = Math.abs(d.tenorYears - meeting.yearFraction);
                if (diff < minDiff) {
                    minDiff = diff;
                    closestIdx = i;
                }
            });

            annotations[`cbMeeting${idx}`] = {
                type: 'line',
                xMin: closestIdx,
                xMax: closestIdx,
                borderColor: 'rgba(239, 68, 68, 0.8)',
                borderWidth: 2,
                borderDash: [5, 5],
                label: {
                    display: true,
                    content: meeting.label.split('(')[0].trim(),
                    position: 'start',
                    backgroundColor: 'rgba(239, 68, 68, 0.9)',
                    color: '#fff',
                    font: { size: 9 },
                    padding: 3,
                    rotation: -90,
                    yAdjust: -60
                }
            };
        });

        this.chartShort = new Chart(ctx, {
            type: 'line',
            data: {
                labels: tenors,
                datasets: [
                    {
                        label: 'Zero Rate (%)',
                        data: zeroRates,
                        borderColor: '#6366f1',
                        backgroundColor: 'rgba(99, 102, 241, 0.1)',
                        fill: false,
                        tension: 0.1,
                        pointRadius: 0,
                        borderWidth: 2,
                        yAxisID: 'y',
                        hidden: true  // Default: show Forward Rates only
                    },
                    {
                        label: 'Discount Factor',
                        data: discountFactors,
                        borderColor: '#10b981',
                        backgroundColor: 'rgba(16, 185, 129, 0.1)',
                        fill: false,
                        tension: 0.1,
                        pointRadius: 0,
                        borderWidth: 2,
                        yAxisID: 'y1',
                        hidden: true  // Default: show Forward Rates only
                    },
                    {
                        label: 'Forward Rate (%)',
                        data: forwardRates,
                        borderColor: '#f59e0b',
                        backgroundColor: 'rgba(245, 158, 11, 0.1)',
                        fill: false,
                        tension: 0.1,
                        pointRadius: 0,
                        borderWidth: 2,
                        yAxisID: 'y',
                        hidden: false  // Default: show Forward Rates
                    }
                ]
            },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                interaction: {
                    mode: 'index',
                    intersect: false
                },
                plugins: {
                    legend: {
                        display: true,
                        position: 'top',
                        labels: { color: '#8b8b9a', usePointStyle: true, boxWidth: 8, font: { size: 10 } }
                    },
                    annotation: {
                        annotations: annotations
                    },
                    tooltip: {
                        backgroundColor: 'rgba(26, 26, 46, 0.95)',
                        titleColor: '#fff',
                        bodyColor: '#8b8b9a',
                        callbacks: {
                            title: (items) => {
                                const idx = items[0]?.dataIndex;
                                if (idx !== undefined && data[idx]) {
                                    // Check if this is near a CB meeting
                                    const meeting = cbMeetings.find(m => {
                                        const meetingIdx = data.findIndex(d => Math.abs(d.tenorYears - m.yearFraction) < 0.01);
                                        return Math.abs(meetingIdx - idx) <= 1;
                                    });
                                    if (meeting) {
                                        return `${tenors[idx]} - ${meeting.label}`;
                                    }
                                    return tenors[idx];
                                }
                                return '';
                            }
                        }
                    }
                },
                scales: {
                    x: {
                        title: { display: true, text: 'Tenor', color: '#8b8b9a', font: { size: 10 } },
                        ticks: { color: '#8b8b9a', font: { size: 9 }, maxRotation: 45, maxTicksLimit: 15 },
                        grid: { color: 'rgba(255, 255, 255, 0.05)' }
                    },
                    y: {
                        type: 'linear',
                        position: 'left',
                        title: { display: true, text: 'Rate (%)', color: '#8b8b9a', font: { size: 10 } },
                        ticks: { color: '#8b8b9a', font: { size: 9 } },
                        grid: { color: 'rgba(255, 255, 255, 0.05)' }
                    },
                    y1: {
                        type: 'linear',
                        position: 'right',
                        title: { display: true, text: 'Discount Factor', color: '#8b8b9a', font: { size: 10 } },
                        ticks: { color: '#8b8b9a', font: { size: 9 } },
                        grid: { drawOnChartArea: false }
                    }
                }
            }
        });
    },

    /**
     * Renders the long-term curve chart (0-30Y).
     */
    renderLongTermChart(data) {
        if (!this.elements.parameterChartLong) return;

        const ctx = this.elements.parameterChartLong.getContext('2d');

        // Destroy existing chart
        if (this.chartLong) {
            this.chartLong.destroy();
        }

        const tenors = data.map(p => this.formatTenor(p.tenorYears));
        const discountFactors = data.map(p => p.discountFactor);
        const zeroRates = data.map(p => (p.zeroRate * 100));
        const forwardRates = data.map(p => ((p.forwardRate || 0) * 100));

        this.chartLong = new Chart(ctx, {
            type: 'line',
            data: {
                labels: tenors,
                datasets: [
                    {
                        label: 'Zero Rate (%)',
                        data: zeroRates,
                        borderColor: '#6366f1',
                        backgroundColor: 'rgba(99, 102, 241, 0.1)',
                        fill: false,
                        tension: 0.2,
                        pointRadius: 0,
                        borderWidth: 2,
                        yAxisID: 'y',
                        hidden: true  // Default: show Forward Rates only
                    },
                    {
                        label: 'Discount Factor',
                        data: discountFactors,
                        borderColor: '#10b981',
                        backgroundColor: 'rgba(16, 185, 129, 0.1)',
                        fill: false,
                        tension: 0.2,
                        pointRadius: 0,
                        borderWidth: 2,
                        yAxisID: 'y1',
                        hidden: true  // Default: show Forward Rates only
                    },
                    {
                        label: 'Forward Rate (%)',
                        data: forwardRates,
                        borderColor: '#f59e0b',
                        backgroundColor: 'rgba(245, 158, 11, 0.1)',
                        fill: false,
                        tension: 0.2,
                        pointRadius: 0,
                        borderWidth: 2,
                        yAxisID: 'y',
                        hidden: false  // Default: show Forward Rates
                    }
                ]
            },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                interaction: {
                    mode: 'index',
                    intersect: false
                },
                plugins: {
                    legend: {
                        display: true,
                        position: 'top',
                        labels: { color: '#8b8b9a', usePointStyle: true, boxWidth: 8, font: { size: 10 } }
                    }
                },
                scales: {
                    x: {
                        title: { display: true, text: 'Tenor', color: '#8b8b9a', font: { size: 10 } },
                        ticks: { color: '#8b8b9a', font: { size: 9 }, maxRotation: 45, maxTicksLimit: 20 },
                        grid: { color: 'rgba(255, 255, 255, 0.05)' }
                    },
                    y: {
                        type: 'linear',
                        position: 'left',
                        title: { display: true, text: 'Rate (%)', color: '#8b8b9a', font: { size: 10 } },
                        ticks: { color: '#8b8b9a', font: { size: 9 } },
                        grid: { color: 'rgba(255, 255, 255, 0.05)' }
                    },
                    y1: {
                        type: 'linear',
                        position: 'right',
                        title: { display: true, text: 'Discount Factor', color: '#8b8b9a', font: { size: 10 } },
                        ticks: { color: '#8b8b9a', font: { size: 9 } },
                        grid: { drawOnChartArea: false }
                    }
                }
            }
        });
    },

    renderParameterTabs() {
        if (!this.elements.parameterTabsContainer) return;

        this.elements.parameterTabsContainer.innerHTML = `
            <div class="param-tabs-row">
                <div class="param-tabs">
                    <button class="param-tab" data-type="zero">Zero Rates</button>
                    <button class="param-tab" data-type="df">Discount Factors</button>
                    <button class="param-tab active" data-type="fwd">Forward Rates</button>
                </div>
                <div class="cb-meeting-toggle">
                    <label class="toggle-label">
                        <input type="checkbox" id="cb-meeting-toggle" checked>
                        <span class="toggle-text"><i class="fas fa-landmark"></i> CB Meetings</span>
                    </label>
                </div>
            </div>
        `;

        // Attach event listeners to tabs
        this.elements.parameterTabsContainer.querySelectorAll('.param-tab').forEach(tab => {
            tab.addEventListener('click', (e) => {
                // Update active state
                this.elements.parameterTabsContainer.querySelectorAll('.param-tab').forEach(t =>
                    t.classList.remove('active')
                );
                e.target.classList.add('active');

                // Filter display based on selected type
                const type = e.target.dataset.type;
                this.filterParameterView(type);
            });
        });

        // Attach event listener to CB Meeting toggle
        const cbToggle = document.getElementById('cb-meeting-toggle');
        if (cbToggle) {
            cbToggle.addEventListener('change', (e) => {
                this.toggleCbMeetingAnnotations(e.target.checked);
            });
        }
    },

    /**
     * Toggles visibility of CB Meeting annotations on the short-term chart.
     */
    toggleCbMeetingAnnotations(show) {
        if (!this.chartShort?.options?.plugins?.annotation?.annotations) return;

        const annotations = this.chartShort.options.plugins.annotation.annotations;
        Object.keys(annotations).forEach(key => {
            if (key.startsWith('cbMeeting')) {
                annotations[key].display = show;
            }
        });
        this.chartShort.update();
    },

    /**
     * Filters the parameter table and chart based on selected type.
     * @param {string} type - 'all', 'zero', 'df', or 'fwd'
     */
    filterParameterView(type) {
        if (!this.state.buildResult?.parameters) return;

        const params = this.state.buildResult.parameters;

        // Update table columns visibility
        this.renderFilteredTable(params, type);

        // Update chart datasets visibility
        this.updateChartVisibility(type);
    },

    /**
     * Renders a filtered parameter table based on type.
     */
    renderFilteredTable(params, type) {
        if (!this.elements.parameterTableContainer) return;

        const showDf = type === 'all' || type === 'df';
        const showZero = type === 'all' || type === 'zero';
        const showFwd = type === 'all' || type === 'fwd';

        const html = `
            <table class="parameter-table">
                <thead>
                    <tr>
                        <th>Tenor</th>
                        ${showDf ? '<th>DF</th>' : ''}
                        ${showZero ? '<th>Zero Rate</th>' : ''}
                        ${showFwd ? '<th>Fwd Rate</th>' : ''}
                    </tr>
                </thead>
                <tbody>
                    ${params.map(p => `
                        <tr>
                            <td>${this.formatTenor(p.tenorYears)}</td>
                            ${showDf ? `<td>${p.discountFactor?.toFixed(6) || '-'}</td>` : ''}
                            ${showZero ? `<td>${((p.zeroRate || 0) * 100).toFixed(4)}%</td>` : ''}
                            ${showFwd ? `<td>${((p.forwardRate || 0) * 100).toFixed(4)}%</td>` : ''}
                        </tr>
                    `).join('')}
                </tbody>
            </table>
        `;
        this.elements.parameterTableContainer.innerHTML = html;
    },

    /**
     * Updates chart dataset visibility based on selected type.
     * Both charts now have: [0] Zero Rate, [1] Discount Factor, [2] Forward Rate
     */
    updateChartVisibility(type) {
        const showZero = type === 'all' || type === 'zero';
        const showDf = type === 'all' || type === 'df';
        const showFwd = type === 'all' || type === 'fwd';

        // Update short-term chart (has Zero Rate, Discount Factor, Forward Rate)
        if (this.chartShort) {
            if (this.chartShort.data.datasets[0]) {
                this.chartShort.data.datasets[0].hidden = !showZero;
            }
            if (this.chartShort.data.datasets[1]) {
                this.chartShort.data.datasets[1].hidden = !showDf;
            }
            if (this.chartShort.data.datasets[2]) {
                this.chartShort.data.datasets[2].hidden = !showFwd;
            }
            this.chartShort.update();
        }

        // Update long-term chart (has Zero Rate, Discount Factor, Forward Rate)
        if (this.chartLong) {
            if (this.chartLong.data.datasets[0]) {
                this.chartLong.data.datasets[0].hidden = !showZero;
            }
            if (this.chartLong.data.datasets[1]) {
                this.chartLong.data.datasets[1].hidden = !showDf;
            }
            if (this.chartLong.data.datasets[2]) {
                this.chartLong.data.datasets[2].hidden = !showFwd;
            }
            this.chartLong.update();
        }
    },

    renderParameterTable(params) {
        // Delegate to filtered table with 'zero' type (default)
        this.renderFilteredTable(params, 'fwd');
    },

    exportRates() {
        const data = {
            index: this.state.selectedIndex,
            instruments: this.state.instruments
        };
        this.downloadJson(data, `${this.state.selectedIndex || 'curve'}_rates.json`);
    },

    importRates() {
        const input = document.createElement('input');
        input.type = 'file';
        input.accept = '.json';
        input.onchange = async (e) => {
            const file = e.target.files[0];
            if (!file) return;

            try {
                const text = await file.text();
                const data = JSON.parse(text);
                if (data.instruments && Array.isArray(data.instruments)) {
                    this.state.instruments = data.instruments;
                    this.state.hasChanges = true;
                    this.updateChangesIndicator();
                    this.renderInstrumentTable();
                }
            } catch (error) {
                this.showError('Failed to import rates: ' + error.message);
            }
        };
        input.click();
    },

    resetRates() {
        this.state.instruments = JSON.parse(JSON.stringify(this.state.originalInstruments));
        this.state.hasChanges = false;
        this.updateChangesIndicator();
        this.renderInstrumentTable();
    },

    exportCsv() {
        if (!this.state.buildResult?.parameters) return;

        const params = this.state.buildResult.parameters;
        const csv = [
            'Tenor,TenorYears,DiscountFactor,ZeroRate,ForwardRate',
            ...params.map(p => `${this.formatTenor(p.tenorYears)},${p.tenorYears},${p.discountFactor},${p.zeroRate},${p.forwardRate || ''}`)
        ].join('\n');

        this.downloadFile(csv, `${this.state.selectedIndex || 'curve'}_parameters.csv`, 'text/csv');
    },

    exportJson() {
        if (!this.state.buildResult) return;
        this.downloadJson(this.state.buildResult, `${this.state.selectedIndex || 'curve'}_result.json`);
    },

    downloadJson(data, filename) {
        const json = JSON.stringify(data, null, 2);
        this.downloadFile(json, filename, 'application/json');
    },

    downloadFile(content, filename, mimeType) {
        const blob = new Blob([content], { type: mimeType });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = filename;
        a.click();
        URL.revokeObjectURL(url);
    },

    showLoading(message = 'Processing...') {
        if (this.elements.loadingOverlay) {
            this.elements.loadingOverlay.style.display = 'flex';
            const textEl = document.getElementById('curve-builder-loading-text');
            if (textEl) textEl.textContent = message;
        }
    },

    hideLoading() {
        if (this.elements.loadingOverlay) {
            this.elements.loadingOverlay.style.display = 'none';
        }
    },

    showError(message) {
        if (this.elements.errorContainer && this.elements.errorMessage) {
            this.elements.errorMessage.textContent = message;
            this.elements.errorContainer.style.display = 'flex';
        }
        if (typeof Logger !== 'undefined') {
            Logger.error('CurveBuilder', message);
        }
    },

    hideError() {
        if (this.elements.errorContainer) {
            this.elements.errorContainer.style.display = 'none';
        }
    }
};

// Auto-initialise when DOM is ready and view is shown
(function() {
    console.log('[CurveBuilder] Module loaded, setting up...');

    function setupCurveBuilder() {
        console.log('[CurveBuilder] Setup running, DOM state:', document.readyState);

        // Initialize when curve-builder view becomes visible
        window.addEventListener('viewChanged', (e) => {
            console.log('[CurveBuilder] viewChanged event received:', e.detail?.view);
            if (e.detail?.view === 'curve-builder') {
                // Force re-render when view becomes visible
                if (curveBuilder.initialized) {
                    console.log('[CurveBuilder] View activated, re-rendering...');
                    curveBuilder.cacheElements();
                    curveBuilder.renderIndexSelector();
                    curveBuilder.renderSettings();
                    if (curveBuilder.state.instruments.length > 0) {
                        curveBuilder.renderInstrumentTable();
                    }
                } else {
                    console.log('[CurveBuilder] View activated, initializing...');
                    curveBuilder.init();
                }
            }
        });

        // Check if already on curve-builder view
        const curveBuilderView = document.getElementById('curve-builder-view');
        console.log('[CurveBuilder] curve-builder-view element:', !!curveBuilderView, 'active:', curveBuilderView?.classList.contains('active'));

        if (curveBuilderView?.classList.contains('active')) {
            console.log('[CurveBuilder] Already on curve-builder view, initializing...');
            curveBuilder.init();
        }
    }

    // Handle both cases: DOM already ready or still loading
    if (document.readyState === 'loading') {
        console.log('[CurveBuilder] DOM still loading, waiting for DOMContentLoaded...');
        document.addEventListener('DOMContentLoaded', setupCurveBuilder);
    } else {
        console.log('[CurveBuilder] DOM already ready, setting up immediately...');
        setupCurveBuilder();
    }
})();
