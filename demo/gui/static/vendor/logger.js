/**
 * FrictionalBank Logger Utility
 * =============================
 * Centralised logging with debug mode control and structured output.
 *
 * Requirements:
 * - 1.1, 1.2, 1.3, 1.4, 1.5: Debug log production control
 * - 8.1, 8.2, 8.3, 8.4, 8.5: Structured logging
 */

(function(global) {
    'use strict';

    // ============================================
    // Task 1.1: ConfigManager Implementation
    // ============================================

    /**
     * ConfigManager - Manages environment configuration
     * Reads from window.__FB_CONFIG__ (injected by server)
     */
    const ConfigManager = {
        _config: null,

        /**
         * Initialise configuration from server-injected values
         * @returns {Object} Configuration object
         */
        init: function() {
            if (this._config) return this._config;

            const serverConfig = global.__FB_CONFIG__ || {};

            this._config = {
                debugMode: this._parseBoolean(serverConfig.debugMode, false),
                logLevel: this._parseLogLevel(serverConfig.logLevel, 'INFO'),
                initialized: true
            };

            return this._config;
        },

        /**
         * Get current configuration
         * @returns {Object} Current configuration
         */
        get: function() {
            if (!this._config) {
                this.init();
            }
            return this._config;
        },

        /**
         * Check if debug mode is enabled
         * @returns {boolean} True if debug mode is enabled
         */
        isDebugEnabled: function() {
            return this.get().debugMode;
        },

        /**
         * Get current log level
         * @returns {string} Current log level
         */
        getLogLevel: function() {
            return this.get().logLevel;
        },

        /**
         * Parse boolean value from various input types
         * @private
         */
        _parseBoolean: function(value, defaultValue) {
            if (value === undefined || value === null) return defaultValue;
            if (typeof value === 'boolean') return value;
            if (typeof value === 'string') {
                return value.toLowerCase() === 'true' || value === '1';
            }
            return Boolean(value);
        },

        /**
         * Parse and validate log level
         * @private
         */
        _parseLogLevel: function(value, defaultValue) {
            const validLevels = ['DEBUG', 'INFO', 'WARN', 'ERROR'];
            if (!value) return defaultValue;
            const upper = String(value).toUpperCase();
            return validLevels.includes(upper) ? upper : defaultValue;
        }
    };

    // ============================================
    // Task 1.2: Logger Utility Implementation
    // ============================================

    const LOG_LEVELS = {
        DEBUG: 0,
        INFO: 1,
        WARN: 2,
        ERROR: 3
    };

    /**
     * Logger - Structured logging utility
     * Provides DEBUG/INFO/WARN/ERROR levels with component prefixes
     */
    const Logger = {
        _level: null,

        /**
         * Initialise logger with current configuration
         */
        init: function() {
            ConfigManager.init();
            this._level = LOG_LEVELS[ConfigManager.getLogLevel()];

            // Log initialisation status once (Requirement 1.5)
            const config = ConfigManager.get();
            if (config.debugMode) {
                this._output('INFO', 'Logger', 'Logger initialised', {
                    debugMode: config.debugMode,
                    logLevel: config.logLevel
                });
            }
        },

        /**
         * Log debug message (suppressed in production)
         * @param {string} component - Component name (e.g., 'WebSocket', 'API', 'UI')
         * @param {string} message - Log message
         * @param {Object} [data] - Optional structured data
         */
        debug: function(component, message, data) {
            // Requirement 1.3: Suppress DEBUG in production
            if (!ConfigManager.isDebugEnabled()) return;
            this._log('DEBUG', component, message, data);
        },

        /**
         * Log info message
         * @param {string} component - Component name
         * @param {string} message - Log message
         * @param {Object} [data] - Optional structured data
         */
        info: function(component, message, data) {
            this._log('INFO', component, message, data);
        },

        /**
         * Log warning message
         * @param {string} component - Component name
         * @param {string} message - Log message
         * @param {Object} [data] - Optional structured data
         */
        warn: function(component, message, data) {
            this._log('WARN', component, message, data);
        },

        /**
         * Log error message
         * @param {string} component - Component name
         * @param {string} message - Log message
         * @param {Object} [data] - Optional structured data
         */
        error: function(component, message, data) {
            this._log('ERROR', component, message, data);
        },

        /**
         * Set log level dynamically
         * @param {string} level - New log level
         */
        setLevel: function(level) {
            const upper = String(level).toUpperCase();
            if (LOG_LEVELS.hasOwnProperty(upper)) {
                this._level = LOG_LEVELS[upper];
            }
        },

        /**
         * Check if debug mode is enabled
         * @returns {boolean} True if debug is enabled
         */
        isDebugEnabled: function() {
            return ConfigManager.isDebugEnabled();
        },

        /**
         * Get current log level
         * @returns {string} Current log level name
         */
        getLevel: function() {
            for (const [name, value] of Object.entries(LOG_LEVELS)) {
                if (value === this._level) return name;
            }
            return 'INFO';
        },

        /**
         * Internal log method with level filtering
         * @private
         */
        _log: function(level, component, message, data) {
            if (this._level === null) {
                this.init();
            }

            const levelValue = LOG_LEVELS[level];
            if (levelValue < this._level) return;

            this._output(level, component, message, data);
        },

        /**
         * Output formatted log message
         * @private
         */
        _output: function(level, component, message, data) {
            // Requirement 8.3: Include timestamp
            const timestamp = new Date().toISOString();

            // Requirement 8.4: Include component name
            const prefix = `[${timestamp}] [${level}] [${component}]`;

            const consoleMethod = this._getConsoleMethod(level);

            if (data !== undefined && data !== null) {
                // Requirement 1.4: Don't expose sensitive data in production
                if (!ConfigManager.isDebugEnabled() && this._containsSensitiveKeys(data)) {
                    consoleMethod(`${prefix} ${message}`, '[data redacted]');
                } else {
                    consoleMethod(`${prefix} ${message}`, data);
                }
            } else {
                consoleMethod(`${prefix} ${message}`);
            }
        },

        /**
         * Get appropriate console method for level
         * @private
         */
        _getConsoleMethod: function(level) {
            switch (level) {
                case 'DEBUG': return console.debug.bind(console);
                case 'INFO': return console.info.bind(console);
                case 'WARN': return console.warn.bind(console);
                case 'ERROR': return console.error.bind(console);
                default: return console.log.bind(console);
            }
        },

        /**
         * Check if data contains potentially sensitive keys
         * @private
         */
        _containsSensitiveKeys: function(data) {
            if (typeof data !== 'object' || data === null) return false;

            const sensitivePatterns = [
                /password/i, /token/i, /secret/i, /key/i, /auth/i,
                /credential/i, /apiresponse/i, /internalstate/i
            ];

            const checkKeys = (obj) => {
                for (const key of Object.keys(obj)) {
                    if (sensitivePatterns.some(p => p.test(key))) return true;
                    if (typeof obj[key] === 'object' && obj[key] !== null) {
                        if (checkKeys(obj[key])) return true;
                    }
                }
                return false;
            };

            return checkKeys(data);
        }
    };

    // ============================================
    // Unit Tests (only in debug mode)
    // ============================================

    /**
     * Run Logger unit tests
     * @returns {Object} Test results
     */
    function runLoggerTests() {
        const results = { passed: 0, failed: 0, tests: [] };

        function assert(condition, message) {
            if (condition) {
                results.passed++;
                results.tests.push({ pass: true, message });
            } else {
                results.failed++;
                results.tests.push({ pass: false, message });
            }
        }

        // Test ConfigManager
        assert(typeof ConfigManager.init === 'function', 'ConfigManager.init is a function');
        assert(typeof ConfigManager.get === 'function', 'ConfigManager.get is a function');
        assert(typeof ConfigManager.isDebugEnabled === 'function', 'ConfigManager.isDebugEnabled is a function');

        // Test ConfigManager defaults
        const config = ConfigManager.get();
        assert(typeof config.debugMode === 'boolean', 'debugMode is boolean');
        assert(typeof config.logLevel === 'string', 'logLevel is string');
        assert(['DEBUG', 'INFO', 'WARN', 'ERROR'].includes(config.logLevel), 'logLevel is valid');

        // Test Logger
        assert(typeof Logger.debug === 'function', 'Logger.debug is a function');
        assert(typeof Logger.info === 'function', 'Logger.info is a function');
        assert(typeof Logger.warn === 'function', 'Logger.warn is a function');
        assert(typeof Logger.error === 'function', 'Logger.error is a function');
        assert(typeof Logger.setLevel === 'function', 'Logger.setLevel is a function');
        assert(typeof Logger.isDebugEnabled === 'function', 'Logger.isDebugEnabled is a function');

        // Test level filtering
        Logger.setLevel('ERROR');
        assert(Logger.getLevel() === 'ERROR', 'setLevel changes level');
        Logger.setLevel(config.logLevel); // restore

        // Test boolean parsing
        assert(ConfigManager._parseBoolean(true, false) === true, 'parseBoolean handles true');
        assert(ConfigManager._parseBoolean(false, true) === false, 'parseBoolean handles false');
        assert(ConfigManager._parseBoolean('true', false) === true, 'parseBoolean handles "true"');
        assert(ConfigManager._parseBoolean('false', true) === false, 'parseBoolean handles "false"');
        assert(ConfigManager._parseBoolean(null, true) === true, 'parseBoolean handles null');

        // Test log level parsing
        assert(ConfigManager._parseLogLevel('debug', 'INFO') === 'DEBUG', 'parseLogLevel handles lowercase');
        assert(ConfigManager._parseLogLevel('WARN', 'INFO') === 'WARN', 'parseLogLevel handles uppercase');
        assert(ConfigManager._parseLogLevel('invalid', 'INFO') === 'INFO', 'parseLogLevel handles invalid');

        return results;
    }

    // ============================================
    // Global Exports
    // ============================================

    // Expose as globals for easy access
    global.FB_ConfigManager = ConfigManager;
    global.FB_Logger = Logger;

    // Auto-initialise
    Logger.init();

    // Run tests in debug mode
    if (ConfigManager.isDebugEnabled() && typeof global.FB_RUN_LOGGER_TESTS !== 'undefined') {
        const testResults = runLoggerTests();
        console.log('=== Logger Unit Tests ===');
        console.log(`${testResults.passed}/${testResults.passed + testResults.failed} tests passed`);
        testResults.tests.forEach(t => {
            console.log(`${t.pass ? 'PASS' : 'FAIL'}: ${t.message}`);
        });
    }

})(typeof window !== 'undefined' ? window : this);
