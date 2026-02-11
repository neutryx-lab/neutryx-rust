/**
 * Composable encapsulating all state management, types, API calls, and
 * business logic for the Curve Builder view.
 */

import { ref, computed, watch, onMounted, nextTick } from 'vue';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface CurveConfig {
  name: string;
  description?: string;
  rateIndex: string;
  instruments: string[];
  calibrationMethod: string;
  interpolation: string;
  allowExtrapolation: boolean;
}

export interface CurvesData {
  metadata: {
    description: string;
    version: string;
    sections: Record<string, string>;
  };
  curves: CurveConfig[];
}

export interface RateInstrument {
  type: string;
  tenor?: string;
  tenor_years?: number;
  rate?: number;
  frequency?: string;
  description?: string;
  // For event type instruments
  id?: string;
  event_date?: string;
  expected_rate_spike?: number;
  end_date?: string; // Turn events: spike reverts after this date
}

export interface RateData {
  index: string;
  currency: string;
  reference_date: string;
  instruments: RateInstrument[];
}

export interface DisplayInstrument {
  id: string;
  type: string;
  tenor: string;
  tenorYears: number;
  rate: number;
  enabled: boolean;
  originalRate: number;
  eventDate?: string; // For EVENT type instruments
  endDate?: string; // Turn events: spike reverts after this date
}

// instruments.json types
export interface InstrumentConfig {
  id: string;
  currency: string;
  convention: string;
  tenor: string;
  rateIndex: string;
  eventDate?: string;
  expectedRateSpike?: number; // Expected rate jump for CB events (e.g., -0.0025 = -25bp)
}

export interface InstrumentsData {
  metadata: Record<string, unknown>;
  templates: unknown[];
  instruments: InstrumentConfig[];
}

export interface CurvePillar {
  date: string;
  time: number;
  discount_factor: number;
  zero_rate: number;
  forward_rate: number;
}

export interface ForwardRatePoint {
  date: string;
  time: number;
  forward_rate: number;
}

export interface ChartGridPoint {
  date: string;
  time: number;
  discount_factor: number;
  forward_rate: number;
  label: string;
}

export interface JacobianData {
  row_labels: string[];
  col_labels: string[];
  matrix: number[][];
  size: number;
}

export interface BuildResult {
  curve_id?: string;
  instrument_count?: number;
  interpolation?: string;
  calculation_time_ms?: number;
  pillars?: CurvePillar[];
  forward_curve?: ForwardRatePoint[];
  short_term_grid?: ChartGridPoint[];
  long_term_grid?: ChartGridPoint[];
  converged?: boolean;
  bootstrap_method?: string;
  jacobian?: JacobianData;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

export const calibrationMethods = [
  { value: 'bootstrapping', label: 'Bootstrapping' },
  { value: 'global', label: 'Global' },
];

export const interpolationMethods = [
  { value: 'flat_forward', label: 'Flat Forward' },
  { value: 'log_linear_df', label: 'Log-Linear DF' },
  { value: 'linear_df', label: 'Linear DF' },
];

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/** Normalise interpolation values from curves.json (legacy) to backend snake_case format */
function normaliseInterpolation(value: string): string {
  const map: Record<string, string> = {
    'loglinear': 'log_linear_df',
    'log_linear': 'log_linear_df',
    'linear': 'linear_df',
    'monotone_cubic': 'log_linear_df',
    'cubic': 'log_linear_df',
    'cubic_spline': 'log_linear_df',
  };
  return map[value] || value;
}

/** Normalise calibration method values (legacy "sequential" -> "bootstrapping") */
function normaliseCalibrationMethod(value: string): string {
  if (value === 'sequential') return 'bootstrapping';
  return value;
}

function buildInstrumentId(type: string, tenor: string, currency: string): string {
  const typeMap: Record<string, string> = {
    'deposit': 'Depo',
    'ois': 'OIS',
    'fra': 'FRA',
    'future': 'Future',
    'swap': 'Swap',
  };
  const typeLabel = typeMap[type] || type.toUpperCase();
  // Normalize tenor: "O/N" -> "ON" to match curve config format
  const normalizedTenor = tenor === 'O/N' ? 'ON' : tenor;
  return `${currency}-${typeLabel}-${normalizedTenor}`;
}

// ---------------------------------------------------------------------------
// Composable
// ---------------------------------------------------------------------------

export function useCurveBuilder(updateChartsCallback: () => void) {
  // State
  const curvesConfig = ref<CurvesData | null>(null);
  const instrumentsConfig = ref<InstrumentsData | null>(null);
  const selectedCurveName = ref<string>('');
  const selectedCurve = ref<CurveConfig | null>(null);
  const rateData = ref<RateData | null>(null);
  const instruments = ref<DisplayInstrument[]>([]);
  const buildResult = ref<BuildResult | null>(null);
  const isLoading = ref(false);
  const isBuilding = ref(false);
  const loadError = ref<string | null>(null);
  const buildError = ref<string | null>(null);

  // Build settings (editable)
  const calibrationMethod = ref<string>('bootstrapping');
  const interpolation = ref<string>('log_linear_df');
  const allowExtrapolation = ref<boolean>(true);

  // Last-built settings -- used to detect "rebuild required"
  const lastBuiltSettings = ref<{
    calibrationMethod: string;
    interpolation: string;
    allowExtrapolation: boolean;
  } | null>(null);

  // ---------- Computed ----------

  const curveOptions = computed(() => {
    if (!curvesConfig.value) return [];
    return curvesConfig.value.curves.map(c => ({
      name: c.name,
      rateIndex: c.rateIndex,
    }));
  });

  const enabledInstruments = computed(() =>
    instruments.value.filter(inst => inst.enabled)
  );

  const hasChanges = computed(() => {
    if (!buildResult.value) return false; // never built yet -- nothing to rebuild

    // Check if any rate changed since last build
    const rateChanged = instruments.value.some(inst => inst.rate !== inst.originalRate);

    // Check if build settings changed since last build
    const lbs = lastBuiltSettings.value;
    const settingsChanged = lbs != null && (
      calibrationMethod.value !== lbs.calibrationMethod ||
      interpolation.value !== lbs.interpolation ||
      allowExtrapolation.value !== lbs.allowExtrapolation
    );

    return rateChanged || settingsChanged;
  });

  const summaryStats = computed(() => {
    const eventCount = enabledInstruments.value.filter(i => i.type === 'event').length;

    return [
      { label: 'Valuation Date', value: rateData.value?.reference_date || '-', icon: 'fa-calendar', color: '#8b5cf6' },
      { label: 'Instruments', value: `${enabledInstruments.value.length}/${instruments.value.length}${eventCount > 0 ? ` (${eventCount} events)` : ''}`, icon: 'fa-list-alt', color: '#3b82f6' },
      { label: 'Interpolation', value: interpolationMethods.find(m => m.value === interpolation.value)?.label ?? interpolation.value, icon: 'fa-wave-square', color: '#10b981' },
      { label: 'Status', value: buildResult.value ? 'Built' : 'Pending', icon: 'fa-info-circle', color: buildResult.value ? '#10b981' : '#f59e0b' },
    ];
  });

  // Curve data table -- merge short + long term grids, deduplicate by date
  const curveTableRows = computed(() => {
    if (!buildResult.value) return [];
    const shortGrid = buildResult.value.short_term_grid || [];
    const longGrid = buildResult.value.long_term_grid || [];

    const seen = new Set<string>();
    const rows: { date: string; time: number; df: number; fwd: number }[] = [];
    for (const pt of [...shortGrid, ...longGrid]) {
      if (!seen.has(pt.date)) {
        seen.add(pt.date);
        rows.push({ date: pt.date, time: pt.time, df: pt.discount_factor, fwd: pt.forward_rate });
      }
    }
    rows.sort((a, b) => a.time - b.time);
    return rows;
  });

  // ---------- API calls ----------

  async function loadCurvesConfig() {
    loadError.value = null;
    try {
      const response = await fetch('/data/config/curves.json');
      if (!response.ok) throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      curvesConfig.value = await response.json();
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      console.error('Failed to load curves config:', message);
      loadError.value = `Failed to load curves: ${message}`;
    }
  }

  async function loadInstrumentsConfig() {
    try {
      const response = await fetch('/data/config/instruments.json');
      if (!response.ok) throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      instrumentsConfig.value = await response.json();
    } catch (error) {
      console.error('Failed to load instruments config:', error);
    }
  }

  async function loadRateData(rateIndex: string) {
    try {
      // Convert rate index to file name (e.g., "USD-SOFR" -> "usd-sofr")
      const fileName = rateIndex.toLowerCase().replace('_', '-');
      const response = await fetch(`/data/input/rates/${fileName}.json`);
      if (!response.ok) throw new Error(`Failed to load rate data for ${rateIndex}`);
      rateData.value = await response.json();
    } catch (error) {
      console.error('Failed to load rate data:', error);
      rateData.value = null;
    }
  }

  // ---------- Instrument helpers ----------

  function loadInstrumentsForCurve() {
    if (!selectedCurve.value || !rateData.value) {
      instruments.value = [];
      return;
    }

    const currency = rateData.value.currency;
    const referenceDate = new Date(rateData.value.reference_date);

    // Get the set of instrument IDs that should be enabled by default (from curve config)
    const defaultEnabledIds = new Set(selectedCurve.value.instruments || []);

    // Build display instruments from rate data
    const displayInstruments: DisplayInstrument[] = [];

    for (const rateInst of rateData.value.instruments) {
      // Handle event type instruments from rate input file
      if (rateInst.type === 'event') {
        const eventDate = new Date(rateInst.event_date || '');

        // Skip past events
        if (eventDate < referenceDate) continue;

        // Approximate tenor for display sorting only (not used for pricing)
        const tenorYears = (eventDate.getTime() - referenceDate.getTime()) / (365.25 * 86_400_000);

        const id = rateInst.id || '';
        // Only include if in curve definition
        if (!defaultEnabledIds.has(id)) continue;

        displayInstruments.push({
          id,
          type: 'event',
          tenor: 'EVENT',
          tenorYears,
          rate: rateInst.expected_rate_spike || 0,
          originalRate: rateInst.expected_rate_spike || 0,
          enabled: true,
          eventDate: rateInst.event_date,
          endDate: rateInst.end_date,
        });
      } else {
        // Handle regular instruments (deposit, ois, fra, etc.)
        const tenor = rateInst.tenor || '';
        const id = buildInstrumentId(rateInst.type, tenor, currency);

        displayInstruments.push({
          id,
          type: rateInst.type,
          tenor,
          tenorYears: rateInst.tenor_years || 0,
          rate: rateInst.rate || 0,
          originalRate: rateInst.rate || 0,
          enabled: defaultEnabledIds.has(id),
        });
      }
    }

    // Sort by tenor years
    displayInstruments.sort((a, b) => a.tenorYears - b.tenorYears);

    instruments.value = displayInstruments;
  }

  // ---------- Curve selection ----------

  async function onCurveSelected() {
    if (!selectedCurveName.value || !curvesConfig.value) {
      selectedCurve.value = null;
      instruments.value = [];
      buildResult.value = null;
      return;
    }

    isLoading.value = true;

    try {
      // Find selected curve config
      const curve = curvesConfig.value.curves.find(c => c.name === selectedCurveName.value);
      if (!curve) return;

      selectedCurve.value = curve;

      // Set build settings from curve config
      calibrationMethod.value = normaliseCalibrationMethod(curve.calibrationMethod);
      interpolation.value = normaliseInterpolation(curve.interpolation);
      allowExtrapolation.value = curve.allowExtrapolation;

      // Load rate data for this curve's rate index
      await loadRateData(curve.rateIndex);

      // Build instruments list
      loadInstrumentsForCurve();

      // Clear previous build result
      buildResult.value = null;
    } finally {
      isLoading.value = false;
    }
  }

  // ---------- Build ----------

  async function buildCurve() {
    if (!selectedCurve.value || enabledInstruments.value.length === 0) return;

    isBuilding.value = true;
    buildError.value = null;
    try {
      // Build instrument payload including events
      const instrumentPayload = enabledInstruments.value.map(inst => {
        if (inst.type === 'event') {
          const payload: Record<string, unknown> = {
            instrument_type: 'event',
            tenor: '',
            rate: 0,
            event_date: inst.eventDate,
            expected_rate_spike: inst.rate, // rate field stores the spike for events
          };
          // Turn events: include end_date so the spike reverts
          if (inst.endDate) {
            payload.end_date = inst.endDate;
          }
          return payload;
        } else {
          return {
            instrument_type: inst.type.toLowerCase(),
            tenor: inst.tenor,
            rate: inst.rate,
          };
        }
      });

      const response = await fetch('/api/curves/build', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          index: selectedCurve.value.rateIndex,
          currency: rateData.value?.currency || 'USD',
          reference_date: rateData.value?.reference_date,
          instruments: instrumentPayload,
          bootstrap_method: calibrationMethod.value,
          interpolation: interpolation.value,
        }),
      });

      if (!response.ok) {
        const text = await response.text();
        let message = 'Build failed';
        try {
          const errorData = JSON.parse(text);
          message = errorData.error || errorData.message || message;
        } catch {
          message = text || message;
        }
        throw new Error(message);
      }

      buildResult.value = await response.json();

      // Snapshot current state as "last built"
      instruments.value.forEach(inst => {
        inst.originalRate = inst.rate;
      });
      lastBuiltSettings.value = {
        calibrationMethod: calibrationMethod.value,
        interpolation: interpolation.value,
        allowExtrapolation: allowExtrapolation.value,
      };

      // Update charts
      await nextTick();
      updateChartsCallback();
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      console.error('Build failed:', message);
      buildError.value = message;
    } finally {
      isBuilding.value = false;
    }
  }

  // ---------- Actions ----------

  function resetSettings() {
    if (!selectedCurve.value) return;

    // Reset build settings
    calibrationMethod.value = normaliseCalibrationMethod(selectedCurve.value.calibrationMethod);
    interpolation.value = normaliseInterpolation(selectedCurve.value.interpolation);
    allowExtrapolation.value = selectedCurve.value.allowExtrapolation;

    // Reset rates
    instruments.value.forEach(inst => {
      inst.rate = inst.originalRate;
    });
  }

  function exportRates() {
    if (instruments.value.length === 0) return;

    const csv = [
      'ID,Type,Tenor,Rate(%),EventDate,Enabled',
      ...instruments.value.map(
        inst => `${inst.id},${inst.type},${inst.tenor},${inst.type === 'event' ? '' : (inst.rate * 100).toFixed(4)},${inst.eventDate || ''},${inst.enabled}`
      ),
    ].join('\n');

    const blob = new Blob([csv], { type: 'text/csv' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `curve_instruments_${selectedCurveName.value || 'unknown'}.csv`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  }

  function updateRate(index: number, value: string) {
    instruments.value[index].rate = parseFloat(value) / 100;
  }

  function updateSpike(index: number, value: string) {
    // Convert basis points to decimal (e.g., -25bp = -0.0025)
    instruments.value[index].rate = parseFloat(value) / 10000;
  }

  function toggleEnabled(index: number) {
    instruments.value[index].enabled = !instruments.value[index].enabled;
  }

  function toggleAll(enabled: boolean) {
    instruments.value.forEach(inst => inst.enabled = enabled);
  }

  // ---------- Watchers ----------

  watch(selectedCurveName, () => {
    onCurveSelected();
  });

  // ---------- Lifecycle ----------

  onMounted(async () => {
    await Promise.all([loadCurvesConfig(), loadInstrumentsConfig()]);
    // Set default selection to USD-SOFR
    if (curvesConfig.value?.curves.some(c => c.name === 'USD-SOFR')) {
      selectedCurveName.value = 'USD-SOFR';
    }
  });

  // ---------- Return ----------

  return {
    // State
    curvesConfig,
    instrumentsConfig,
    selectedCurveName,
    selectedCurve,
    rateData,
    instruments,
    buildResult,
    isLoading,
    isBuilding,
    loadError,
    buildError,
    calibrationMethod,
    interpolation,
    allowExtrapolation,
    lastBuiltSettings,

    // Computed
    curveOptions,
    enabledInstruments,
    hasChanges,
    summaryStats,
    curveTableRows,

    // Actions
    buildCurve,
    resetSettings,
    exportRates,
    updateRate,
    updateSpike,
    toggleEnabled,
    toggleAll,
  };
}
