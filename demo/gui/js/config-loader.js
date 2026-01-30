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
    let _loadError = null; // Store error if load fails

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
                _loadError = null;
                console.log('[ConfigLoader] Configuration loaded successfully');
                return _config;
            } catch (error) {
                _loadError = error;
                console.error('[ConfigLoader] Failed to load config:', error.message);
                throw error;
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
     * Get the load error if configuration failed to load.
     *
     * @returns {Error|null} The error or null if loaded successfully
     */
    function getLoadError() {
        return _loadError;
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
        getLoadError,
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
