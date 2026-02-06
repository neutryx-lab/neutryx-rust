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

import type { AppConfig, EnumValue } from '@/types';
import { fetchConfig } from './api';
import { createScopedLogger } from '@/utils/logger';

const log = createScopedLogger('ConfigLoader');

// =============================================================================
// Private State
// =============================================================================

let _config: AppConfig | null = null;
let _loaded = false;
let _loading: Promise<AppConfig> | null = null;
let _loadError: Error | null = null;

// =============================================================================
// Public API
// =============================================================================

/**
 * Load configuration from the API.
 * Multiple concurrent calls will share the same promise.
 */
export async function load(): Promise<AppConfig> {
  if (_loaded && _config) {
    return _config;
  }

  if (_loading) {
    return _loading;
  }

  _loading = (async () => {
    try {
      _config = await fetchConfig();
      _loaded = true;
      _loadError = null;
      log.info('Configuration loaded successfully');
      return _config;
    } catch (error) {
      _loadError = error instanceof Error ? error : new Error(String(error));
      log.error('Failed to load config', _loadError.message);
      throw _loadError;
    } finally {
      _loading = null;
    }
  })();

  return _loading;
}

/**
 * Check if configuration has been loaded.
 */
export function isLoaded(): boolean {
  return _loaded;
}

/**
 * Get the load error if configuration failed to load.
 */
export function getLoadError(): Error | null {
  return _loadError;
}

/**
 * Get an Enum array by name.
 */
export function getEnum(name: string): EnumValue[] {
  return _config?.enums?.[name] ?? [];
}

/**
 * Get Enum codes only (for enums with code/name structure).
 */
export function getEnumCodes(name: string): string[] {
  const enums = getEnum(name);
  if (enums.length === 0) return [];

  // If it's an array of strings, return as-is
  if (typeof enums[0] === 'string') {
    return enums as string[];
  }

  // If it's an array of objects with 'code', extract codes
  return (enums as { code: string }[]).map((e) => e.code);
}

/**
 * Get a default value by path.
 */
export function getDefault<T = unknown>(path: string): T | undefined {
  const parts = path.split('.');
  let value: unknown = _config?.defaults;

  for (const part of parts) {
    if (value === undefined || value === null) {
      return undefined;
    }
    value = (value as Record<string, unknown>)[part];
  }

  return value as T | undefined;
}

/**
 * Get the rate index for a currency.
 */
export function getRateIndex(currency: string): string | undefined {
  return _config?.rateIndexByCurrency?.[currency];
}

/**
 * Get all rate index mappings.
 */
export function getRateIndexMap(): Record<string, string> {
  return _config?.rateIndexByCurrency ?? {};
}

/**
 * Get the complete configuration object.
 */
export function getConfig(): AppConfig | null {
  return _config;
}

/**
 * Populate a select element with Enum values.
 */
export interface PopulateSelectOptions {
  valueField?: string;
  labelField?: string;
  selected?: string;
  includeEmpty?: boolean;
}

export function populateSelect(
  selectElement: HTMLSelectElement | null,
  enumName: string,
  options: PopulateSelectOptions = {}
): void {
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

  enums.forEach((item) => {
    const option = document.createElement('option');

    if (isSimple) {
      option.value = item as string;
      option.textContent = item as string;
    } else {
      // Object with code/name structure
      const obj = item as { code: string; name?: string };
      option.value = obj.code;
      option.textContent = obj.name ?? obj.code;
    }

    if (selected !== undefined && option.value === selected) {
      option.selected = true;
    }

    selectElement.appendChild(option);
  });
}

/**
 * Create tenor order mapping for sorting.
 */
export function getTenorOrder(): Record<string, number> {
  const tenors = getEnum('tenor') as string[];
  const order: Record<string, number> = {};
  tenors.forEach((tenor, index) => {
    order[tenor] = index;
  });
  return order;
}

/**
 * Reload configuration (force refresh).
 */
export async function reload(): Promise<AppConfig> {
  _loaded = false;
  _config = null;
  _loading = null;
  return load();
}

// =============================================================================
// ConfigLoader Object (for backward compatibility)
// =============================================================================

export const ConfigLoader = {
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
  get config() {
    return _config;
  },
};

export default ConfigLoader;
