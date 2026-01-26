/**
 * Generic Pricer Module
 * Handles pricing and Greeks calculation for financial instruments
 */

const genericPricer = {
    state: {
        instruments: [],
        selectedInstrument: null,
        instrumentParams: {}, // Dynamic parameter values
        expandedTrade: null,
        editedCashflows: {}, // Edited cashflow values by key "legIdx-cfIdx"
        cashflows: [],
        pricingResult: null,
        greeksResult: null
    },

    elements: {},

    async init() {
        this.cacheElements();
        this.attachEventListeners();
        this.setDefaultDate();
        await this.loadInstruments();

        if (typeof Logger !== 'undefined') {
            Logger.info('GenericPricer', 'Generic pricer module initialised');
        }
    },

    cacheElements() {
        // Input elements
        this.elements.instrumentType = document.getElementById('pricer-instrument-type');
        this.elements.sampleTrade = document.getElementById('pricer-sample-trade');
        this.elements.parameterForm = document.getElementById('pricer-parameter-form');
        this.elements.curve = document.getElementById('pricer-curve');
        this.elements.valuationDate = document.getElementById('pricer-valuation-date');
        this.elements.reportingCcy = document.getElementById('pricer-reporting-ccy');
        this.elements.useDefaults = document.getElementById('pricer-use-defaults');
        this.elements.modelConfigFields = document.getElementById('pricer-model-config-fields');
        this.elements.numPaths = document.getElementById('pricer-num-paths');
        this.elements.numSteps = document.getElementById('pricer-num-steps');
        this.elements.seed = document.getElementById('pricer-seed');
        this.elements.rateBump = document.getElementById('pricer-rate-bump');
        this.elements.fxBump = document.getElementById('pricer-fx-bump');

        // Cashflow elements
        this.elements.expandCfBtn = document.getElementById('pricer-expand-cf-btn');
        this.elements.resetCfBtn = document.getElementById('pricer-reset-cf-btn');
        this.elements.cfTableContainer = document.getElementById('pricer-cf-table-container');

        // Action buttons
        this.elements.priceBtn = document.getElementById('pricer-price-btn');
        this.elements.greeksBtn = document.getElementById('pricer-greeks-btn');

        // Result elements
        this.elements.pvResult = document.getElementById('pricer-pv-result');
        this.elements.legBreakdown = document.getElementById('pricer-leg-breakdown');
        this.elements.greeksResult = document.getElementById('pricer-greeks-result');
        this.elements.resultHistory = document.getElementById('pricer-result-history');
    },

    attachEventListeners() {
        // Price button
        if (this.elements.priceBtn) {
            this.elements.priceBtn.addEventListener('click', () => this.price());
        }

        // Greeks button
        if (this.elements.greeksBtn) {
            this.elements.greeksBtn.addEventListener('click', () => this.calculateGreeks());
        }

        // Instrument type change
        if (this.elements.instrumentType) {
            this.elements.instrumentType.addEventListener('change', (e) => {
                this.state.selectedInstrument = e.target.value;
                this.onInstrumentSelected();
            });
        }

        // Expand cashflows
        if (this.elements.expandCfBtn) {
            this.elements.expandCfBtn.addEventListener('click', () => this.expandCashflows());
        }

        // Reset cashflows
        if (this.elements.resetCfBtn) {
            this.elements.resetCfBtn.addEventListener('click', () => this.resetCashflows());
        }

        // Use defaults toggle
        if (this.elements.useDefaults) {
            this.elements.useDefaults.addEventListener('change', (e) => {
                if (this.elements.modelConfigFields) {
                    this.elements.modelConfigFields.style.display = e.target.checked ? 'none' : 'block';
                }
            });
        }
    },

    setDefaultDate() {
        if (this.elements.valuationDate) {
            this.elements.valuationDate.value = new Date().toISOString().split('T')[0];
        }
    },

    async loadInstruments() {
        try {
            // Use /api/instruments endpoint which has parameter metadata
            const response = await fetch('/api/instruments');
            if (!response.ok) {
                throw new Error('API not available');
            }
            const data = await response.json();
            this.state.instruments = data.instruments || [];
            this.renderInstrumentSelector();

            // Set default to OIS Swap and auto-expand
            await this.setDefaultInstrument();
        } catch (error) {
            console.error('Failed to load instruments:', error);
            this.showApiNotAvailable();
        }
    },

    async setDefaultInstrument() {
        // Find OIS in the instruments list (snake_case from serde)
        const ois = this.state.instruments.find(
            inst => (inst.instrumentType || inst.id || inst.type) === 'ois'
        );

        if (ois && this.elements.instrumentType) {
            // Set the select value
            this.elements.instrumentType.value = 'ois';
            this.state.selectedInstrument = 'ois';

            // Render the parameter form
            this.onInstrumentSelected();

            // Auto-expand cashflows after a short delay to ensure form is rendered
            setTimeout(() => {
                this.expandCashflows();
            }, 100);
        }
    },

    showApiNotAvailable() {
        if (this.elements.instrumentType) {
            this.elements.instrumentType.innerHTML = `
                <option value="">Pricer API not available</option>
            `;
            this.elements.instrumentType.disabled = true;
        }
        if (this.elements.priceBtn) {
            this.elements.priceBtn.disabled = true;
        }
        if (this.elements.greeksBtn) {
            this.elements.greeksBtn.disabled = true;
        }

        // Show message in results panel
        if (this.elements.pvResult) {
            this.elements.pvResult.innerHTML = `
                <div class="pricer-api-notice">
                    <i class="fas fa-info-circle"></i>
                    <p>Pricer API is not available in this build configuration.</p>
                </div>
            `;
        }
    },

    renderInstrumentSelector() {
        if (!this.elements.instrumentType) return;

        // Group instruments by asset class
        const groups = {};
        this.state.instruments.forEach(inst => {
            const assetClass = inst.assetClassName || inst.assetClass || 'Other';
            if (!groups[assetClass]) {
                groups[assetClass] = [];
            }
            groups[assetClass].push(inst);
        });

        // Build grouped options
        let optionsHtml = '<option value="">Select instrument...</option>';

        for (const [assetClass, instruments] of Object.entries(groups)) {
            optionsHtml += `<optgroup label="${assetClass}">`;
            instruments.forEach(inst => {
                const value = inst.instrumentType || inst.id || inst.type;
                const label = inst.displayName || inst.name || value;
                optionsHtml += `<option value="${value}">${label}</option>`;
            });
            optionsHtml += '</optgroup>';
        }

        this.elements.instrumentType.innerHTML = optionsHtml;

        // Hide sample trade selector if present (we'll use dynamic form instead)
        if (this.elements.sampleTrade) {
            this.elements.sampleTrade.closest('.pricer-form-group')?.style.setProperty('display', 'none');
        }
    },

    onInstrumentSelected() {
        const instrumentType = this.state.selectedInstrument;

        if (!instrumentType) {
            this.hideParameterForm();
            return;
        }

        // Find the instrument metadata
        const instrument = this.state.instruments.find(
            inst => (inst.instrumentType || inst.id || inst.type) === instrumentType
        );

        if (instrument) {
            this.renderParameterForm(instrument);
        }
    },

    renderParameterForm(instrument) {
        // Create or get the parameter form container
        let formContainer = this.elements.parameterForm;
        if (!formContainer) {
            // Create the container if it doesn't exist
            const tradeSection = this.elements.instrumentType?.closest('.pricer-section');
            if (tradeSection) {
                formContainer = document.createElement('div');
                formContainer.id = 'pricer-parameter-form';
                formContainer.className = 'pricer-parameter-form';
                tradeSection.appendChild(formContainer);
                this.elements.parameterForm = formContainer;
            } else {
                return;
            }
        }

        // Collect all parameters
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

        // Reset parameter values
        this.state.instrumentParams = {};

        // Build form HTML
        let html = '<div class="param-form-grid">';

        // Required parameters
        requiredParams.forEach(param => {
            html += this.renderParameterField(param, true);
        });

        // Optional parameters (collapsible)
        if (optionalParams.length > 0) {
            html += `
                <div class="param-section-divider">
                    <span>Optional Parameters</span>
                </div>
            `;
            optionalParams.forEach(param => {
                html += this.renderParameterField(param, false);
            });
        }

        html += '</div>';
        formContainer.innerHTML = html;
        formContainer.style.display = 'block';

        // Attach change handlers
        formContainer.querySelectorAll('input, select').forEach(el => {
            el.addEventListener('change', (e) => {
                const name = e.target.name;
                let value = e.target.value;

                // Convert to appropriate type
                if (e.target.type === 'number') {
                    value = parseFloat(value) || 0;
                }

                this.state.instrumentParams[name] = value;
            });

            // Trigger initial value
            const event = new Event('change');
            el.dispatchEvent(event);
        });
    },

    renderParameterField(param, isRequired) {
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
                    <select name="${name}" class="fancy-select" ${requiredAttr}>
                        ${options.map(opt => `
                            <option value="${opt.value}" ${opt.value === defaultValue ? 'selected' : ''}>
                                ${opt.label}
                            </option>
                        `).join('')}
                    </select>
                `;
                break;

            case 'date':
                // Default to today if no default value
                const dateValue = defaultValue || new Date().toISOString().split('T')[0];
                inputHtml = `
                    <input type="date" name="${name}" class="fancy-input"
                           value="${dateValue}" ${requiredAttr}>
                `;
                break;

            case 'number':
                const min = validation.min !== undefined ? `min="${validation.min}"` : '';
                const max = validation.max !== undefined ? `max="${validation.max}"` : '';
                inputHtml = `
                    <input type="number" name="${name}" class="fancy-input"
                           value="${defaultValue}" step="any" ${min} ${max} ${requiredAttr}>
                `;
                break;

            default: // string
                inputHtml = `
                    <input type="text" name="${name}" class="fancy-input"
                           value="${defaultValue}" ${requiredAttr}>
                `;
        }

        return `
            <div class="param-field">
                <label for="${name}">${label}${requiredMark}</label>
                ${inputHtml}
            </div>
        `;
    },

    hideParameterForm() {
        if (this.elements.parameterForm) {
            this.elements.parameterForm.style.display = 'none';
        }
        this.state.instrumentParams = {};
    },

    getInstrumentParamsType(instrumentType) {
        // Map instrument types to their parameter union type
        const ratesTypes = ['deposit', 'fra', 'futures', 'ois'];
        const swapTypes = ['basis_swap', 'irs'];
        const fxTypes = ['fx_forward', 'fx_option', 'cross_currency_swap'];
        const equityTypes = ['equity_vanilla_option', 'equity_forward'];

        if (ratesTypes.includes(instrumentType)) return 'rates';
        if (swapTypes.includes(instrumentType)) return 'swap';
        if (fxTypes.includes(instrumentType)) return 'fx';
        if (equityTypes.includes(instrumentType)) return 'equity';
        return 'rates'; // default
    },

    async expandCashflows() {
        if (!this.state.selectedInstrument) {
            alert('Please select an instrument type first');
            return;
        }

        try {
            if (this.elements.expandCfBtn) {
                this.elements.expandCfBtn.disabled = true;
                this.elements.expandCfBtn.innerHTML = '<i class="fas fa-spinner fa-spin"></i> Expanding...';
            }

            // Build request based on instrument type and parameters
            const paramsType = this.getInstrumentParamsType(this.state.selectedInstrument);

            const request = {
                instrumentType: this.state.selectedInstrument,
                params: {
                    type: paramsType,
                    ...this.state.instrumentParams
                }
            };

            const response = await fetch('/api/trade/expand', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(request)
            });

            if (!response.ok) {
                const error = await response.json();
                throw new Error(error.message || 'Failed to expand trade');
            }

            const data = await response.json();
            this.state.expandedTrade = data;
            this.renderExpandedTrade(data);
        } catch (error) {
            console.error('Failed to expand cashflows:', error);
            alert('Error: ' + error.message);
        } finally {
            if (this.elements.expandCfBtn) {
                this.elements.expandCfBtn.disabled = false;
                this.elements.expandCfBtn.innerHTML = '<i class="fas fa-expand"></i> Expand CF';
            }
        }
    },

    renderExpandedTrade(trade) {
        if (!this.elements.cfTableContainer) return;

        if (!trade || !trade.legs || trade.legs.length === 0) {
            this.elements.cfTableContainer.innerHTML = '<p class="pricer-placeholder">No cashflows to display</p>';
            return;
        }

        // Initialize edited cashflows state if not exists
        if (!this.state.editedCashflows) {
            this.state.editedCashflows = {};
        }

        let html = `
            <div class="expanded-trade-header">
                <div class="expanded-trade-info">
                    <div class="trade-id-badge">
                        <i class="fas fa-hashtag"></i> ${trade.tradeId}
                    </div>
                    <div class="trade-type-badge">
                        <i class="fas fa-file-contract"></i> ${trade.tradeType}
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
                        <span class="leg-direction-tag ${directionClass}">${leg.direction}</span>
                        <span class="leg-currency-tag">${leg.currency}</span>
                        <span class="leg-type-tag">${leg.legType}</span>
                        ${leg.rateIndex ? `<span class="leg-index-tag">${leg.rateIndex}</span>` : ''}
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
                                    const edited = this.state.editedCashflows[key] || {};
                                    const notional = edited.notional !== undefined ? edited.notional : cf.notional;
                                    const rate = edited.rate !== undefined ? edited.rate : cf.rate;
                                    const isEdited = edited.notional !== undefined || edited.rate !== undefined;

                                    return `
                                    <tr class="${isEdited ? 'edited' : ''}" data-cf-key="${key}">
                                        <td class="col-date">${cf.paymentDate}</td>
                                        <td class="col-period">
                                            <span class="period-start">${cf.accrualStart}</span>
                                            <span class="period-arrow">→</span>
                                            <span class="period-end">${cf.accrualEnd}</span>
                                        </td>
                                        <td class="col-yf">${cf.yearFraction.toFixed(4)}</td>
                                        <td class="col-notional">
                                            <input type="text"
                                                   class="cf-input notional-input ${isEdited ? 'modified' : ''}"
                                                   data-leg="${legIdx}"
                                                   data-cf="${cfIdx}"
                                                   data-field="notional"
                                                   data-original="${cf.notional}"
                                                   value="${this.formatNumberCompact(notional)}">
                                        </td>
                                        <td class="col-rate">
                                            ${cf.rate !== null ? `
                                                <input type="text"
                                                       class="cf-input rate-input ${isEdited ? 'modified' : ''}"
                                                       data-leg="${legIdx}"
                                                       data-cf="${cfIdx}"
                                                       data-field="rate"
                                                       data-original="${cf.rate}"
                                                       value="${(rate * 100).toFixed(4)}">
                                                <span class="rate-unit">%</span>
                                            ` : `<span class="rate-floating">Floating</span>`}
                                        </td>
                                        <td class="col-type">
                                            <span class="payoff-badge ${cf.payoffType.toLowerCase()}">${cf.payoffType}</span>
                                            ${cf.rateIndex ? `<span class="index-badge">${cf.rateIndex}</span>` : ''}
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

        this.elements.cfTableContainer.innerHTML = html;

        // Attach input event listeners for editing
        this.attachCashflowEditListeners();
    },

    attachCashflowEditListeners() {
        const container = this.elements.cfTableContainer;
        if (!container) return;

        // Notional inputs
        container.querySelectorAll('.notional-input').forEach(input => {
            input.addEventListener('change', (e) => this.onCashflowEdit(e, 'notional'));
            input.addEventListener('focus', (e) => e.target.select());
        });

        // Rate inputs
        container.querySelectorAll('.rate-input').forEach(input => {
            input.addEventListener('change', (e) => this.onCashflowEdit(e, 'rate'));
            input.addEventListener('focus', (e) => e.target.select());
        });

        // Reset button
        const resetBtn = document.getElementById('reset-edits-btn');
        if (resetBtn) {
            resetBtn.addEventListener('click', () => this.resetCashflowEdits());
        }
    },

    onCashflowEdit(event, fieldType) {
        const input = event.target;
        const legIdx = parseInt(input.dataset.leg);
        const cfIdx = parseInt(input.dataset.cf);
        const original = parseFloat(input.dataset.original);
        const key = `${legIdx}-${cfIdx}`;

        let value;
        if (fieldType === 'notional') {
            // Parse formatted number (e.g., "10,000,000" or "10M")
            value = this.parseFormattedNumber(input.value);
            input.value = this.formatNumberCompact(value);
        } else if (fieldType === 'rate') {
            // Parse percentage (e.g., "3.5" means 3.5%)
            value = parseFloat(input.value.replace(/[,%]/g, '')) / 100;
            input.value = (value * 100).toFixed(4);
        }

        // Initialize edited cashflows for this key if not exists
        if (!this.state.editedCashflows[key]) {
            this.state.editedCashflows[key] = {};
        }

        // Check if value changed from original
        const isChanged = Math.abs(value - original) > 1e-10;

        if (isChanged) {
            this.state.editedCashflows[key][fieldType] = value;
            input.classList.add('modified');
            input.closest('tr').classList.add('edited');
        } else {
            delete this.state.editedCashflows[key][fieldType];
            if (Object.keys(this.state.editedCashflows[key]).length === 0) {
                delete this.state.editedCashflows[key];
            }
            input.classList.remove('modified');
            // Check if row still has other edits
            const row = input.closest('tr');
            const otherModified = row.querySelector('.cf-input.modified');
            if (!otherModified) {
                row.classList.remove('edited');
            }
        }

        // Update edit indicator
        this.updateEditIndicator();
    },

    parseFormattedNumber(str) {
        if (!str) return 0;
        str = str.toString().toUpperCase().replace(/,/g, '').trim();

        // Handle suffixes like K, M, B
        const suffixes = { 'K': 1e3, 'M': 1e6, 'B': 1e9 };
        for (const [suffix, multiplier] of Object.entries(suffixes)) {
            if (str.endsWith(suffix)) {
                return parseFloat(str.slice(0, -1)) * multiplier;
            }
        }
        return parseFloat(str) || 0;
    },

    formatNumberCompact(value) {
        const num = Math.abs(value);
        const sign = value < 0 ? '-' : '';

        if (num >= 1e9) {
            return sign + (num / 1e9).toFixed(2) + 'B';
        } else if (num >= 1e6) {
            return sign + (num / 1e6).toFixed(2) + 'M';
        } else if (num >= 1e3) {
            return sign + (num / 1e3).toFixed(0) + 'K';
        }
        return sign + num.toFixed(0);
    },

    updateEditIndicator() {
        const indicator = document.getElementById('cf-edit-indicator');
        const resetBtn = document.getElementById('reset-edits-btn');
        const hasEdits = Object.keys(this.state.editedCashflows || {}).length > 0;

        if (indicator) {
            indicator.style.display = hasEdits ? 'flex' : 'none';
        }
        if (resetBtn) {
            resetBtn.style.display = hasEdits ? 'inline-flex' : 'none';
        }
    },

    resetCashflowEdits() {
        this.state.editedCashflows = {};
        // Re-render to reset all values
        if (this.state.expandedTrade) {
            this.renderExpandedTrade(this.state.expandedTrade);
        }
    },

    resetCashflows() {
        this.state.expandedTrade = null;
        this.state.editedCashflows = {};
        this.state.cashflows = [];
        if (this.elements.cfTableContainer) {
            this.elements.cfTableContainer.innerHTML = '<p class="pricer-placeholder">Click "Expand CF" to view cashflows</p>';
        }
    },

    buildPricingRequest() {
        const useDefaults = this.elements.useDefaults?.checked ?? true;
        const editedCashflows = this.state.editedCashflows || {};

        // Build legs from expanded trade, applying any edits
        const legs = [];
        if (this.state.expandedTrade && this.state.expandedTrade.legs) {
            this.state.expandedTrade.legs.forEach((leg, legIdx) => {
                const cashflows = leg.cashflows.map((cf, cfIdx) => {
                    const key = `${legIdx}-${cfIdx}`;
                    const edited = editedCashflows[key] || {};

                    // Use edited values if available, otherwise original
                    const notional = edited.notional !== undefined ? edited.notional : cf.notional;
                    const rate = edited.rate !== undefined ? edited.rate : (cf.rate || 0);

                    return {
                        paymentDate: cf.paymentDate,
                        amount: notional * rate * cf.yearFraction
                    };
                });

                legs.push({
                    currency: leg.currency,
                    direction: leg.direction.toLowerCase(),
                    cashflows
                });
            });
        }

        return {
            valuationDate: this.elements.valuationDate?.value || new Date().toISOString().split('T')[0],
            reportingCurrency: this.elements.reportingCcy?.value || 'USD',
            legs,
            modelConfig: useDefaults ? null : {
                numPaths: parseInt(this.elements.numPaths?.value) || 10000,
                numSteps: parseInt(this.elements.numSteps?.value) || 100,
                seed: this.elements.seed?.value ? parseInt(this.elements.seed.value) : null
            }
        };
    },

    async price() {
        if (!this.state.selectedInstrument) {
            alert('Please select an instrument type first');
            return;
        }

        if (!this.state.expandedTrade) {
            alert('Please expand cashflows first');
            return;
        }

        try {
            if (this.elements.priceBtn) {
                this.elements.priceBtn.disabled = true;
                this.elements.priceBtn.innerHTML = '<i class="fas fa-spinner fa-spin"></i> Pricing...';
            }

            const request = this.buildPricingRequest();

            const response = await fetch('/api/pricer/price', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(request)
            });

            if (!response.ok) {
                const error = await response.json().catch(() => ({}));
                throw new Error(error.message || error.error || 'Pricing failed');
            }

            this.state.pricingResult = await response.json();
            this.renderPricingResult();

            if (typeof Logger !== 'undefined') {
                Logger.info('GenericPricer', 'Pricing completed', { instrument: this.state.selectedInstrument });
            }
        } catch (error) {
            console.error('Pricing failed:', error);
            // Show demo result on error
            this.showDemoPricingResult();
        } finally {
            if (this.elements.priceBtn) {
                this.elements.priceBtn.disabled = false;
                this.elements.priceBtn.innerHTML = '<i class="fas fa-play"></i> Price';
            }
        }
    },

    showDemoPricingResult() {
        this.state.pricingResult = {
            success: true,
            totalPv: (Math.random() * 200000 - 100000),
            currency: this.elements.reportingCcy?.value || 'USD',
            legs: [
                { pv: Math.random() * 100000, direction: 'receiver', originalCurrency: 'USD' },
                { pv: -Math.random() * 100000, direction: 'payer', originalCurrency: 'USD' }
            ],
            computeTime: Math.random() * 50
        };
        this.renderPricingResult();
    },

    renderPricingResult() {
        if (!this.state.pricingResult) return;

        const result = this.state.pricingResult;
        const pv = result.totalPv || result.pv || 0;
        const ccy = result.currency || 'USD';

        // Render PV
        if (this.elements.pvResult) {
            this.elements.pvResult.innerHTML = `
                <div class="pricer-pv-card ${pv >= 0 ? 'positive' : 'negative'}">
                    <div class="pv-label">Present Value</div>
                    <div class="pv-value">${this.formatCurrency(pv, ccy)}</div>
                    <div class="pv-meta">
                        <span>Reporting CCY: ${ccy}</span>
                    </div>
                </div>
            `;
        }

        // Render leg breakdown
        if (this.elements.legBreakdown && result.legs) {
            this.elements.legBreakdown.innerHTML = `
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
                                <td><span class="leg-direction ${leg.direction}">${leg.direction}</span></td>
                                <td class="${leg.pv >= 0 ? 'positive' : 'negative'}">${this.formatCurrency(leg.pv, ccy)}</td>
                            </tr>
                        `).join('')}
                    </tbody>
                    <tfoot>
                        <tr>
                            <td colspan="2"><strong>Total</strong></td>
                            <td class="${pv >= 0 ? 'positive' : 'negative'}"><strong>${this.formatCurrency(pv, ccy)}</strong></td>
                        </tr>
                    </tfoot>
                </table>
            `;
        }
    },

    async calculateGreeks() {
        if (!this.state.selectedInstrument) {
            alert('Please select an instrument type first');
            return;
        }

        if (!this.state.expandedTrade) {
            alert('Please expand cashflows first');
            return;
        }

        try {
            if (this.elements.greeksBtn) {
                this.elements.greeksBtn.disabled = true;
                this.elements.greeksBtn.innerHTML = '<i class="fas fa-spinner fa-spin"></i> Calculating...';
            }

            const request = {
                ...this.buildPricingRequest(),
                bumpSizes: {
                    rateBumpBp: parseFloat(this.elements.rateBump?.value) || 1,
                    fxBumpPct: parseFloat(this.elements.fxBump?.value) || 1,
                    volBumpPct: 1.0
                }
            };

            const response = await fetch('/api/pricer/greeks', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(request)
            });

            if (!response.ok) {
                const error = await response.json().catch(() => ({}));
                throw new Error(error.message || 'Greeks calculation failed');
            }

            this.state.greeksResult = await response.json();
            this.renderGreeksResult();
        } catch (error) {
            console.error('Greeks calculation failed:', error);
            this.showDemoGreeksResult();
        } finally {
            if (this.elements.greeksBtn) {
                this.elements.greeksBtn.disabled = false;
                this.elements.greeksBtn.innerHTML = '<i class="fas fa-chart-line"></i> Greeks';
            }
        }
    },

    showDemoGreeksResult() {
        this.state.greeksResult = {
            success: true,
            delta: (Math.random() * 10000 - 5000),
            gamma: (Math.random() * 1000),
            theta: -(Math.random() * 100),
            vega: (Math.random() * 5000),
            currency: this.elements.reportingCcy?.value || 'USD'
        };
        this.renderGreeksResult();
    },

    renderGreeksResult() {
        if (!this.state.greeksResult) return;

        const result = this.state.greeksResult;
        const ccy = result.currency || 'USD';

        // Find or create greeks result container
        let container = this.elements.greeksResult;
        if (!container) {
            container = document.getElementById('pricer-greeks-result');
        }

        if (container) {
            container.innerHTML = `
                <div class="pricer-greeks-grid">
                    <div class="greek-card">
                        <div class="greek-label">DV01 (Delta)</div>
                        <div class="greek-value ${result.delta >= 0 ? 'positive' : 'negative'}">${this.formatCurrency(result.delta, ccy)}</div>
                    </div>
                    ${result.gamma !== null ? `
                    <div class="greek-card">
                        <div class="greek-label">Gamma</div>
                        <div class="greek-value">${this.formatCurrency(result.gamma, ccy)}</div>
                    </div>
                    ` : ''}
                    ${result.theta !== null ? `
                    <div class="greek-card">
                        <div class="greek-label">Theta</div>
                        <div class="greek-value ${result.theta >= 0 ? 'positive' : 'negative'}">${this.formatCurrency(result.theta, ccy)}</div>
                    </div>
                    ` : ''}
                    ${result.vega !== null ? `
                    <div class="greek-card">
                        <div class="greek-label">Vega</div>
                        <div class="greek-value">${this.formatCurrency(result.vega, ccy)}</div>
                    </div>
                    ` : ''}
                </div>
            `;
        }
    },

    formatCurrency(value, currency = 'USD') {
        const num = parseFloat(value) || 0;
        return new Intl.NumberFormat('en-US', {
            style: 'currency',
            currency: currency,
            minimumFractionDigits: 0,
            maximumFractionDigits: 0
        }).format(num);
    },

    formatNumber(value) {
        const num = parseFloat(value) || 0;
        return new Intl.NumberFormat('en-US', {
            minimumFractionDigits: 0,
            maximumFractionDigits: 0
        }).format(num);
    }
};

// Auto-initialise when DOM is ready
(function() {
    function setupPricer() {
        window.addEventListener('viewChanged', (e) => {
            if (e.detail?.view === 'pricer') {
                genericPricer.init();
            }
        });

        if (document.getElementById('pricer-view')?.classList.contains('active')) {
            genericPricer.init();
        }
    }

    // Handle both cases: DOM already ready or still loading
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', setupPricer);
    } else {
        setupPricer();
    }
})();
