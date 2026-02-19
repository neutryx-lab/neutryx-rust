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
  curveType?: 'rate' | 'credit' | 'fx';
  discountCurve?: string;
  recoveryRate?: number;
  currencyPair?: string;
  fxCurveMethod?: 'flat' | 'irp_generic' | 'irp_basis';
  spot?: number;
  domesticCurve?: string;
  foreignCurve?: string;
  referenceCurve?: string;
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
  coupon_rate?: number; // For bond instruments
}

export interface RateData {
  index: string;
  currency: string;
  reference_date: string;
  instruments: RateInstrument[];
  recovery_rate?: number;
  currency_pair?: string;
  spot?: number;
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
  couponRate?: number; // For BOND type instruments
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
  survival_probability?: number;
  hazard_rate?: number;
  fx_forward?: number;
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
  fx_forward?: number;
  implied_overnight_rate?: number;
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
  curve_type?: string;
  spot?: number;
  currency_pair?: string;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

export const calibrationMethods = [
  { value: 'bootstrapping', label: 'Bootstrapping' },
  { value: 'global', label: 'Global' },
  { value: 'levenberg_marquardt', label: 'Levenberg-Marquardt' },
  { value: 'penalised', label: 'Penalised' },
  { value: 'best_fit', label: 'Best Fit' },
];

export const interpolationMethods = [
  { value: 'flat_forward', label: 'Flat Forward', spline: false },
  { value: 'log_linear_df', label: 'Log-Linear DF', spline: false },
  { value: 'linear_df', label: 'Linear DF', spline: false },
  { value: 'cubic_spline_fwd', label: 'Cubic Spline (Fwd)', spline: true },
  { value: 'monotone_convex', label: 'Monotone Convex', spline: true },
  { value: 'log_cubic_df', label: 'Log-Cubic DF', spline: true },
  { value: 'tension_spline', label: 'Tension Spline', spline: true },
];

/** Calibration methods that pair best with spline interpolation. */
const splinePreferredCalibrations = new Set(['global', 'levenberg_marquardt', 'penalised']);

export type HintLevel = 'good' | 'info' | 'warn';
export interface CompatibilityHint {
  level: HintLevel;
  message: string;
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/** Normalise interpolation values from curves.json (legacy) to backend snake_case format */
function normaliseInterpolation(value: string): string {
  const map: Record<string, string> = {
    'loglinear': 'log_linear_df',
    'log_linear': 'log_linear_df',
    'linear': 'linear_df',
    'monotone_cubic': 'monotone_convex',
    'cubic': 'cubic_spline_fwd',
    'cubic_spline': 'cubic_spline_fwd',
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
    'bond': 'Bond',
    'cds': 'CDS',
    'fx_forward': 'FxFwd',
    'xccy_basis': 'XCCYBasis',
  };
  const typeLabel = typeMap[type] || type.toUpperCase();
  return `${currency}-${typeLabel}-${tenor}`;
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

  // Map of curve name -> built curve_id (for discount curve references)
  const builtCurveIds = ref<Record<string, string>>({});

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
      curveType: c.curveType ?? 'rate',
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

  const isCreditCurve = computed(() =>
    selectedCurve.value?.curveType === 'credit'
  );

  const isFxCurve = computed(() =>
    selectedCurve.value?.curveType === 'fx'
  );

  const summaryStats = computed(() => {
    const eventCount = enabledInstruments.value.filter(i => i.type === 'event').length;

    const stats = [
      { label: 'Valuation Date', value: rateData.value?.reference_date || '-', icon: 'fa-calendar', color: '#8b5cf6' },
      { label: 'Instruments', value: `${enabledInstruments.value.length}/${instruments.value.length}${eventCount > 0 ? ` (${eventCount} events)` : ''}`, icon: 'fa-list-alt', color: '#3b82f6' },
      { label: 'Interpolation', value: interpolationMethods.find(m => m.value === interpolation.value)?.label ?? interpolation.value, icon: 'fa-wave-square', color: '#10b981' },
      { label: 'Status', value: buildResult.value ? 'Built' : 'Pending', icon: 'fa-info-circle', color: buildResult.value ? '#10b981' : '#f59e0b' },
    ];

    if (isCreditCurve.value) {
      const recovery = selectedCurve.value?.recoveryRate ?? 0.40;
      stats.push({ label: 'Recovery', value: `${(recovery * 100).toFixed(0)}%`, icon: 'fa-shield-alt', color: '#ef4444' });
    }

    if (isFxCurve.value && selectedCurve.value) {
      stats.push({ label: 'Spot', value: (selectedCurve.value.spot ?? 0).toFixed(4), icon: 'fa-exchange-alt', color: '#06b6d4' });
      stats.push({ label: 'Pair', value: selectedCurve.value.currencyPair ?? '-', icon: 'fa-coins', color: '#f97316' });
    }

    return stats;
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

      // Credit curves load from /data/input/credit/, FX from /data/input/fx/, rate curves from /data/input/rates/
      const isCredit = selectedCurve.value?.curveType === 'credit';
      const isFx = selectedCurve.value?.curveType === 'fx';
      let basePath: string;
      if (isCredit) basePath = '/data/input/credit';
      else if (isFx) basePath = '/data/input/fx';
      else basePath = '/data/input/rates';

      const response = await fetch(`${basePath}/${fileName}.json`);
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
      } else if (rateInst.type === 'fx_forward') {
        const tenor = rateInst.tenor || '';
        const pair = rateData.value.currency_pair || '';
        const id = `${pair}-FxFwd-${tenor}`;
        displayInstruments.push({
          id,
          type: 'fx_forward',
          tenor,
          tenorYears: rateInst.tenor_years || 0,
          rate: rateInst.rate || 0,
          originalRate: rateInst.rate || 0,
          enabled: defaultEnabledIds.has(id),
        });
      } else if (rateInst.type === 'xccy_basis') {
        const tenor = rateInst.tenor || '';
        const pair = rateData.value.currency_pair || '';
        const id = `${pair}-XCCYBasis-${tenor}`;
        displayInstruments.push({
          id,
          type: 'xccy_basis',
          tenor,
          tenorYears: rateInst.tenor_years || 0,
          rate: rateInst.rate || 0,
          originalRate: rateInst.rate || 0,
          enabled: defaultEnabledIds.has(id),
        });
      } else {
        // Handle regular instruments (deposit, ois, fra, bond, etc.)
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
          couponRate: rateInst.coupon_rate,
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
    if (!selectedCurve.value) return;
    // IRP FX curves have no instruments — allow empty for those
    if (enabledInstruments.value.length === 0 && selectedCurve.value.fxCurveMethod !== 'irp_generic') return;

    isBuilding.value = true;
    buildError.value = null;
    try {
      // Build instrument payload including events
      const instrumentPayload = enabledInstruments.value.map(inst => {
        if (inst.type === 'fx_forward' || inst.type === 'xccy_basis') {
          return {
            instrument_type: inst.type,
            tenor: inst.tenor,
            rate: inst.rate,
          };
        } else if (inst.type === 'event') {
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
        } else if (inst.type === 'bond') {
          return {
            instrument_type: 'bond',
            tenor: inst.tenor,
            rate: inst.rate,
            coupon_rate: inst.couponRate,
          };
        } else {
          return {
            instrument_type: inst.type.toLowerCase(),
            tenor: inst.tenor,
            rate: inst.rate,
          };
        }
      });

      // For credit curves, ensure the discount curve is built first.
      let discountCurveId: string | undefined;
      if (selectedCurve.value.curveType === 'credit' && selectedCurve.value.discountCurve) {
        const discountName = selectedCurve.value.discountCurve;
        discountCurveId = builtCurveIds.value[discountName];

        if (!discountCurveId) {
          // Auto-build the discount curve.
          discountCurveId = await autoBuildDiscountCurve(discountName);
          if (!discountCurveId) {
            throw new Error(`Failed to auto-build discount curve "${discountName}" — please build it first.`);
          }
        }
      }

      const isFx = selectedCurve.value.curveType === 'fx';
      const requestBody: Record<string, unknown> = {
        index: selectedCurve.value.rateIndex,
        currency: rateData.value?.currency || selectedCurve.value.currencyPair?.slice(0, 3) || 'USD',
        reference_date: rateData.value?.reference_date || new Date().toISOString().slice(0, 10),
        instruments: instrumentPayload,
      };
      // FX curves don't use bootstrap/interpolation — omit to avoid unknown variant error
      if (!isFx) {
        requestBody.bootstrap_method = calibrationMethod.value;
        requestBody.interpolation = interpolation.value;
      }

      // Tension spline parameter
      if (interpolation.value === 'tension_spline') {
        requestBody.tension = 1.0;
      }

      // Penalised calibration penalty weight
      if (calibrationMethod.value === 'penalised') {
        requestBody.penalty_weight = 1e-4;
      }

      if (selectedCurve.value.curveType === 'credit') {
        requestBody.curve_type = 'credit';
        requestBody.discount_curve_id = discountCurveId;
        requestBody.recovery_rate = selectedCurve.value.recoveryRate ?? 0.40;
      }

      if (selectedCurve.value.curveType === 'fx') {
        requestBody.curve_type = 'fx';
        requestBody.currency_pair = selectedCurve.value.currencyPair;
        requestBody.spot = selectedCurve.value.spot;
        requestBody.fx_curve_method = selectedCurve.value.fxCurveMethod;

        // For IRP method, auto-build domestic and foreign curves
        if (selectedCurve.value.fxCurveMethod === 'irp_generic') {
          if (selectedCurve.value.domesticCurve) {
            let domId = builtCurveIds.value[selectedCurve.value.domesticCurve];
            if (!domId) {
              domId = await autoBuildDiscountCurve(selectedCurve.value.domesticCurve);
              if (!domId) throw new Error(`Failed to auto-build domestic curve "${selectedCurve.value.domesticCurve}"`);
            }
            requestBody.domestic_curve_id = domId;
          }
          if (selectedCurve.value.foreignCurve) {
            let forId = builtCurveIds.value[selectedCurve.value.foreignCurve];
            if (!forId) {
              forId = await autoBuildDiscountCurve(selectedCurve.value.foreignCurve);
              if (!forId) throw new Error(`Failed to auto-build foreign curve "${selectedCurve.value.foreignCurve}"`);
            }
            requestBody.foreign_curve_id = forId;
          }
        }

        // For IRP Basis method, auto-build reference curve
        if (selectedCurve.value.fxCurveMethod === 'irp_basis' && selectedCurve.value.referenceCurve) {
          let refId = builtCurveIds.value[selectedCurve.value.referenceCurve];
          if (!refId) {
            refId = await autoBuildDiscountCurve(selectedCurve.value.referenceCurve);
            if (!refId) throw new Error(`Failed to auto-build reference curve "${selectedCurve.value.referenceCurve}"`);
          }
          requestBody.reference_curve_id = refId;
        }
      }

      const response = await fetch('/api/curves/build', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(requestBody),
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

      // Store curve_id for discount curve references
      if (buildResult.value?.curve_id && selectedCurve.value) {
        builtCurveIds.value[selectedCurve.value.name] = buildResult.value.curve_id;
      }

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

  // ---------- Auto-build discount curve ----------

  async function autoBuildDiscountCurve(curveName: string): Promise<string | undefined> {
    if (!curvesConfig.value) return undefined;

    const discountConfig = curvesConfig.value.curves.find(c => c.name === curveName);
    if (!discountConfig) return undefined;

    // Load rate data for the discount curve
    const fileName = discountConfig.rateIndex.toLowerCase().replace('_', '-');
    const rateResp = await fetch(`/data/input/rates/${fileName}.json`);
    if (!rateResp.ok) return undefined;

    const discountRateData: RateData = await rateResp.json();
    const defaultIds = new Set(discountConfig.instruments || []);

    // Build instrument payload from rate data (matching the curve definition)
    const instrumentPayload = discountRateData.instruments
      .filter(inst => {
        if (inst.type === 'event') {
          return defaultIds.has(inst.id || '');
        }
        const id = buildInstrumentId(inst.type, inst.tenor || '', discountRateData.currency);
        return defaultIds.has(id);
      })
      .map(inst => {
        if (inst.type === 'event') {
          const payload: Record<string, unknown> = {
            instrument_type: 'event',
            tenor: '',
            rate: 0,
            event_date: inst.event_date,
            expected_rate_spike: inst.expected_rate_spike,
          };
          if (inst.end_date) payload.end_date = inst.end_date;
          return payload;
        }
        return {
          instrument_type: inst.type,
          tenor: inst.tenor,
          rate: inst.rate,
          coupon_rate: inst.coupon_rate,
        };
      });

    const buildResp = await fetch('/api/curves/build', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        index: discountConfig.rateIndex,
        currency: discountRateData.currency,
        reference_date: discountRateData.reference_date,
        instruments: instrumentPayload,
        bootstrap_method: normaliseCalibrationMethod(discountConfig.calibrationMethod),
        interpolation: normaliseInterpolation(discountConfig.interpolation),
      }),
    });

    if (!buildResp.ok) return undefined;

    const result = await buildResp.json();
    if (result.curve_id) {
      builtCurveIds.value[curveName] = result.curve_id;
      return result.curve_id;
    }
    return undefined;
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

  function updatePips(index: number, value: string) {
    instruments.value[index].rate = parseFloat(value);
  }

  function updateCoupon(index: number, value: string) {
    // Convert percentage to decimal (e.g., 4.5% = 0.045)
    instruments.value[index].couponRate = parseFloat(value) / 100;
  }

  function toggleEnabled(index: number) {
    instruments.value[index].enabled = !instruments.value[index].enabled;
  }

  function toggleAll(enabled: boolean) {
    instruments.value.forEach(inst => inst.enabled = enabled);
  }

  // ---------- Compatibility hints ----------

  /** Interpolation list annotated with recommendation badge for current calibration. */
  const annotatedInterpolationMethods = computed(() =>
    interpolationMethods.map(m => {
      const cal = calibrationMethod.value;
      let badge = '';
      if (m.spline && cal === 'bootstrapping') {
        badge = ' \u26A0'; // ⚠
      } else if (m.spline && splinePreferredCalibrations.has(cal)) {
        badge = ' \u2605'; // ★
      } else if (!m.spline && cal === 'penalised') {
        badge = ' \u26A0'; // ⚠
      }
      return { ...m, displayLabel: m.label + badge };
    }),
  );

  /** Contextual hint about the current calibration + interpolation combination. */
  const compatibilityHint = computed<CompatibilityHint | null>(() => {
    const cal = calibrationMethod.value;
    const isSpline = interpolationMethods.find(m => m.value === interpolation.value)?.spline ?? false;

    if (cal === 'bootstrapping' && isSpline) {
      return {
        level: 'warn',
        message: 'Sequential bootstrapping with spline interpolation may converge slowly. Consider Global or LM.',
      };
    }
    if (cal === 'penalised' && !isSpline) {
      return {
        level: 'info',
        message: 'Penalised calibration is most effective with spline interpolation.',
      };
    }
    if (splinePreferredCalibrations.has(cal) && isSpline) {
      return {
        level: 'good',
        message: 'Good combination \u2014 global solver pairs well with spline interpolation.',
      };
    }
    return null;
  });

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
    isCreditCurve,
    isFxCurve,
    summaryStats,
    curveTableRows,
    builtCurveIds,
    annotatedInterpolationMethods,
    compatibilityHint,

    // Actions
    buildCurve,
    resetSettings,
    exportRates,
    updateRate,
    updateSpike,
    updatePips,
    updateCoupon,
    toggleEnabled,
    toggleAll,
  };
}
