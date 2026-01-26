/**
 * Configuration Loader Module
 *
 * Provides centralised configuration management for the WebApp.
 * Fetches Enum values and defaults from the /api/config endpoint,
 * eliminating hardcoded values in the frontend.
 *
 * Usage:
 *   await ConfigLoader.load();
 *   const currencies = ConfigLoader.getEnum('currency');
 *   const defaultSpot = ConfigLoader.getDefault('pricer.equity.spot');
 */
const ConfigLoader = (() => {
    'use strict';

    // ==========================================================================
    // Private State
    // ==========================================================================

    let _config = null;
    let _loaded = false;
    let _loading = null; // Promise for deduplicating concurrent load calls

    // Fallback configuration (used only if API fails)
    const FALLBACK_CONFIG = {
        enums: {
            currency: ['USD', 'EUR', 'GBP', 'JPY', 'CHF'],
            tenor: ['ON', '1W', '2W', '1M', '2M', '3M', '6M', '9M', '1Y', '2Y', '3Y', '5Y', '7Y', '10Y', '15Y', '20Y', '30Y'],
            frequency: [
                { code: 'Daily', name: 'Daily', periodsPerYear: 365 },
                { code: 'Weekly', name: 'Weekly', periodsPerYear: 52 },
                { code: 'Monthly', name: 'Monthly', periodsPerYear: 12 },
                { code: 'Quarterly', name: 'Quarterly', periodsPerYear: 4 },
                { code: 'SemiAnnual', name: 'Semi-Annual', periodsPerYear: 2 },
                { code: 'Annual', name: 'Annual', periodsPerYear: 1 }
            ],
            dayCounter: [
                { code: 'Actual360', name: 'ACT/360' },
                { code: 'Actual365Fixed', name: 'ACT/365' },
                { code: 'Thirty360Bond', name: '30/360' }
            ],
            quoteType: ['Bid', 'Ask', 'Mid', 'Last'],
            greekType: [
                { code: 'delta', isSecondOrder: false },
                { code: 'gamma', isSecondOrder: true },
                { code: 'vega', isSecondOrder: false },
                { code: 'theta', isSecondOrder: false },
                { code: 'rho', isSecondOrder: false }
            ],
            assetClass: ['rates', 'fx', 'equity', 'credit', 'commodity'],
            instrumentType: ['equity_vanilla_option', 'fx_option', 'irs'],
            optionType: ['call', 'put']
        },
        defaults: {
            pricing: { curveRate: 0.05, volatility: 0.20 },
            monteCarlo: { numPaths: 10000, numSteps: 252 },
            bumpSizes: { rate: 0.0001, spot: 0.01, vol: 0.01 },
            pricer: {
                equity: { spot: 100, strike: 100, expiryYears: 1, volatility: 0.20, rate: 0.05, optionType: 'call' },
                fx: { spot: 1.10, strike: 1.10, expiryYears: 1, volatility: 0.10, domesticRate: 0.05, foreignRate: 0.02, optionType: 'call' },
                irs: { notional: 1000000, fixedRate: 0.025, tenorYears: 5 }
            },
            curve: { notional: 10000000, fixedRate: 0.03, tenorYears: 5, interpolation: 'linear_on_log_df' },
            expansion: {
                rates: { currency: 'USD', tenor: '1Y', rate: 0.035, notional: 10000000 },
                swap: { currency: 'USD', tenor: '5Y', fixedRate: 0.03, spread: 0, notional: 10000000, paymentFrequency: 'SemiAnnual', dayCount: 'Actual365Fixed' },
                fx: { baseCurrency: 'EUR', quoteCurrency: 'USD', spotRate: 1.085, forwardRate: 1.09, notional: 1000000, optionType: 'call', volatility: 0.10 },
                equity: { underlying: 'AAPL', spotPrice: 180, strike: 185, volatility: 0.25, riskFreeRate: 0.05, optionType: 'call', direction: 'long' }
            }
        },
        rateIndexByCurrency: {
            USD: 'SOFR',
            EUR: 'EURIBOR3M',
            GBP: 'SONIA',
            JPY: 'TONAR',
            CHF: 'SARON'
        }
    };

    // ==========================================================================
    // Public API
    // ==========================================================================

    /**
     * Load configuration from the API.
     * Multiple concurrent calls will share the same promise.
     *
     * @returns {Promise<Object>} The loaded configuration
     */
    async function load() {
        if (_loaded) {
            return _config;
        }

        if (_loading) {
            return _loading;
        }

        _loading = (async () => {
            try {
                const response = await fetch('/api/config');
                if (!response.ok) {
                    throw new Error(`Config fetch failed: ${response.status}`);
                }
                _config = await response.json();
                _loaded = true;
                console.log('[ConfigLoader] Configuration loaded successfully');
                return _config;
            } catch (error) {
                console.warn('[ConfigLoader] Failed to load config, using fallback:', error.message);
                _config = FALLBACK_CONFIG;
                _loaded = true;
                return _config;
            } finally {
                _loading = null;
            }
        })();

        return _loading;
    }

    /**
     * Check if configuration has been loaded.
     *
     * @returns {boolean} True if configuration is loaded
     */
    function isLoaded() {
        return _loaded;
    }

    /**
     * Get an Enum array by name.
     *
     * @param {string} name - Enum name (e.g., 'currency', 'tenor', 'frequency')
     * @returns {Array} Array of Enum values, or empty array if not found
     */
    function getEnum(name) {
        return _config?.enums?.[name] ?? [];
    }

    /**
     * Get Enum codes only (for enums with code/name structure).
     *
     * @param {string} name - Enum name
     * @returns {Array<string>} Array of codes
     */
    function getEnumCodes(name) {
        const enums = getEnum(name);
        if (enums.length === 0) return [];

        // If it's an array of strings, return as-is
        if (typeof enums[0] === 'string') {
            return enums;
        }

        // If it's an array of objects with 'code', extract codes
        return enums.map(e => e.code);
    }

    /**
     * Get a default value by path.
     *
     * @param {string} path - Dot-separated path (e.g., 'pricer.equity.spot')
     * @returns {*} The default value, or undefined if not found
     */
    function getDefault(path) {
        const parts = path.split('.');
        let value = _config?.defaults;

        for (const part of parts) {
            if (value === undefined || value === null) {
                return undefined;
            }
            value = value[part];
        }

        return value;
    }

    /**
     * Get the rate index for a currency.
     *
     * @param {string} currency - Currency code (e.g., 'USD')
     * @returns {string|undefined} Rate index (e.g., 'SOFR')
     */
    function getRateIndex(currency) {
        return _config?.rateIndexByCurrency?.[currency];
    }

    /**
     * Get all rate index mappings.
     *
     * @returns {Object} Currency to rate index mapping
     */
    function getRateIndexMap() {
        return _config?.rateIndexByCurrency ?? {};
    }

    /**
     * Get the complete configuration object.
     *
     * @returns {Object|null} Configuration object or null if not loaded
     */
    function getConfig() {
        return _config;
    }

    /**
     * Populate a select element with Enum values.
     *
     * @param {HTMLSelectElement} selectElement - The select element to populate
     * @param {string} enumName - Enum name (e.g., 'currency')
     * @param {Object} options - Options
     * @param {string} options.valueField - Field to use as value (default: auto-detect)
     * @param {string} options.labelField - Field to use as label (default: auto-detect)
     * @param {string} options.selected - Value to pre-select
     * @param {boolean} options.includeEmpty - Include empty option (default: false)
     */
    function populateSelect(selectElement, enumName, options = {}) {
        const enums = getEnum(enumName);
        if (!selectElement || enums.length === 0) return;

        const { selected, includeEmpty = false } = options;

        // Clear existing options
        selectElement.innerHTML = '';

        // Add empty option if requested
        if (includeEmpty) {
            const emptyOption = document.createElement('option');
            emptyOption.value = '';
            emptyOption.textContent = '-- Select --';
            selectElement.appendChild(emptyOption);
        }

        // Determine if enums are strings or objects
        const isSimple = typeof enums[0] === 'string';

        enums.forEach(item => {
            const option = document.createElement('option');

            if (isSimple) {
                option.value = item;
                option.textContent = item;
            } else {
                // Object with code/name structure
                option.value = item.code;
                option.textContent = item.name || item.code;
            }

            if (selected !== undefined && option.value === selected) {
                option.selected = true;
            }

            selectElement.appendChild(option);
        });
    }

    /**
     * Create tenor order mapping for sorting.
     *
     * @returns {Object} Tenor to sort order mapping
     */
    function getTenorOrder() {
        const tenors = getEnum('tenor');
        const order = {};
        tenors.forEach((tenor, index) => {
            order[tenor] = index;
        });
        return order;
    }

    /**
     * Reload configuration (force refresh).
     *
     * @returns {Promise<Object>} The reloaded configuration
     */
    async function reload() {
        _loaded = false;
        _config = null;
        _loading = null;
        return load();
    }

    // ==========================================================================
    // Expose Public API
    // ==========================================================================

    return {
        load,
        reload,
        isLoaded,
        getEnum,
        getEnumCodes,
        getDefault,
        getRateIndex,
        getRateIndexMap,
        getConfig,
        populateSelect,
        getTenorOrder,

        // Expose config getter for compatibility
        get config() { return _config; }
    };
})();

// Auto-export for module systems
if (typeof module !== 'undefined' && module.exports) {
    module.exports = ConfigLoader;
}
