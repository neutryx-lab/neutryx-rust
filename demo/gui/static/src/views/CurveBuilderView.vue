<script setup lang="ts">
import { ref, watch, nextTick, onUnmounted } from 'vue';
import { Chart, registerables, type ChartConfiguration } from 'chart.js';
import { useCurveBuilder, calibrationMethods } from '@/composables/useCurveBuilder';
import { useCurveCharts } from '@/composables/useCurveCharts';
import { useMarketEnvStore } from '@/stores/marketEnv';
import { getChartColors } from '@/composables/useChartTheme';
import CurveInstrumentTable from '@/components/curve/CurveInstrumentTable.vue';
import CurveJacobianHeatmap from '@/components/curve/CurveJacobianHeatmap.vue';
import AssetTabBar from '@/components/common/AssetTabBar.vue';
import { useJyInflationStore } from '@/stores/jyInflation';
import { useJYInflation } from '@/composables/useJYInflation';

Chart.register(...registerables);

const jyStore = useJyInflationStore();
const { buildCurves: jyBuildCurvesRaw } = useJYInflation();

const marketEnv = useMarketEnvStore();
const publishFeedback = ref(false);
const assetTab = ref<'rate' | 'credit' | 'fx' | 'inflation'>('rate');

function publishToEnvironment() {
  if (!buildResult.value || !selectedCurveName.value) return;
  const currency = selectedCurve.value?.rateIndex?.split('-')[0] ?? '';
  marketEnv.publishCurve(
    selectedCurveName.value,
    currency,
    buildResult.value,
    interpolation.value,
    calibrationMethod.value,
  );
  publishFeedback.value = true;
  setTimeout(() => { publishFeedback.value = false; }, 2000);
}

// Initialise charts composable
const {
  shortTermChartCanvas,
  longTermChartCanvas,
  chartType,
  updateCharts,
} = useCurveCharts();

// Template refs – bound via ref="..." in <template>; mark as read for TS
void shortTermChartCanvas;
void longTermChartCanvas;

// Inflation chart refs
const inflationCurveCanvas = ref<HTMLCanvasElement | null>(null);
const inflationDfCanvas = ref<HTMLCanvasElement | null>(null);
let inflationCurveChart: Chart | null = null;
let inflationDfChart: Chart | null = null;

onUnmounted(() => {
  inflationCurveChart?.destroy();
  inflationDfChart?.destroy();
});

function renderInflationCurveChart() {
  if (!inflationCurveCanvas.value || !jyStore.curveResult) return;
  inflationCurveChart?.destroy();

  const cc = getChartColors();
  const labels = jyStore.curveResult.nominalCurve.map(p => `${p.tenor}Y`);

  const config: ChartConfiguration<'line'> = {
    type: 'line',
    data: {
      labels,
      datasets: [
        {
          label: 'Nominal',
          data: jyStore.curveResult.nominalCurve.map(p => p.value * 100),
          borderColor: '#3b82f6',
          backgroundColor: '#3b82f61a',
          tension: 0.4,
          fill: false,
        },
        {
          label: 'Real',
          data: jyStore.curveResult.realCurve.map(p => p.value * 100),
          borderColor: '#10b981',
          backgroundColor: '#10b9811a',
          tension: 0.4,
          fill: false,
        },
        {
          label: 'Breakeven',
          data: jyStore.curveResult.breakevenCurve.map(p => p.value * 100),
          borderColor: '#f59e0b',
          backgroundColor: '#f59e0b1a',
          tension: 0.4,
          borderDash: [5, 5],
          fill: false,
        },
      ],
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      plugins: {
        legend: { display: true, position: 'top', labels: { color: cc.tick, usePointStyle: true } },
        tooltip: { backgroundColor: cc.tooltipBg, titleColor: cc.tooltipTitle, bodyColor: cc.tooltipBody },
      },
      scales: {
        x: { grid: { color: cc.grid }, ticks: { color: cc.tick }, title: { display: true, text: 'Tenor', color: cc.tick } },
        y: { grid: { color: cc.grid }, ticks: { color: cc.tick, callback: v => `${(v as number).toFixed(2)}%` }, title: { display: true, text: 'Rate (%)', color: cc.tick } },
      },
    },
  };

  inflationCurveChart = new Chart(inflationCurveCanvas.value, config);
}

function renderInflationDfChart() {
  if (!inflationDfCanvas.value || !jyStore.curveResult) return;
  inflationDfChart?.destroy();

  const cc = getChartColors();
  const labels = jyStore.curveResult.nominalDf.map(p => `${p.tenor}Y`);

  const config: ChartConfiguration<'line'> = {
    type: 'line',
    data: {
      labels,
      datasets: [
        {
          label: 'Nominal DF',
          data: jyStore.curveResult.nominalDf.map(p => p.value),
          borderColor: '#3b82f6',
          tension: 0.4,
          fill: false,
        },
        {
          label: 'Real DF',
          data: jyStore.curveResult.realDf.map(p => p.value),
          borderColor: '#10b981',
          tension: 0.4,
          fill: false,
        },
      ],
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      plugins: {
        legend: { display: true, position: 'top', labels: { color: cc.tick, usePointStyle: true } },
        tooltip: { backgroundColor: cc.tooltipBg, titleColor: cc.tooltipTitle, bodyColor: cc.tooltipBody },
      },
      scales: {
        x: { grid: { color: cc.grid }, ticks: { color: cc.tick } },
        y: { grid: { color: cc.grid }, ticks: { color: cc.tick, callback: v => (v as number).toFixed(4) } },
      },
    },
  };

  inflationDfChart = new Chart(inflationDfCanvas.value, config);
}

// Re-render inflation charts when result changes or tab switches
watch([() => jyStore.curveResult, assetTab], async () => {
  if (assetTab.value === 'inflation' && jyStore.curveResult) {
    await nextTick();
    renderInflationCurveChart();
    renderInflationDfChart();
  }
});

// Initialise builder composable, wiring chart updates via callback
const {
  // State
  curvesConfig,
  selectedCurveName,
  selectedCurve,
  instruments,
  buildResult,
  isLoading,
  isBuilding,
  loadError,
  buildError,
  calibrationMethod,
  interpolation,
  allowExtrapolation,

  // Prerequisite curve overrides
  discountCurveOverride,
  domesticCurveOverride,
  foreignCurveOverride,
  referenceCurveOverride,
  nominalCurveOverride,

  // Computed
  curveOptions,
  availableRateCurves,
  enabledInstruments,
  hasChanges,
  isCreditCurve,
  isFxCurve,
  isInflationCurve,
  summaryStats,
  curveTableRows,
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
} = useCurveBuilder(() => {
  if (buildResult.value) {
    updateCharts(buildResult.value, interpolation.value);
  }
});

// Sync asset tab when a curve is selected (e.g. via URL or initial load)
watch(selectedCurve, (c) => {
  if (c) assetTab.value = (c.curveType ?? 'rate') as 'rate' | 'credit' | 'fx' | 'inflation';
});

// Auto-select the first curve when switching tabs
watch(assetTab, (tab) => {
  const first = curveOptions.value.find(c => c.curveType === tab);
  selectedCurveName.value = first?.name ?? '';
  if (tab !== 'inflation') {
    // Reset chart type when switching asset tabs
    chartType.value = 'forward_rate';
  }
});

// Build inflation curve: sync curve builder instruments → jyStore, then build
async function buildInflationCurve() {
  // Sync nominal curve from prerequisite dropdown
  jyStore.nominalCurveRef = nominalCurveOverride.value;
  // Sync enabled instruments → jyStore.realRates as CurveRatePoint[]
  jyStore.realRates = enabledInstruments.value.map(inst => ({
    instrumentType: inst.type === 'inflation_swap' ? 'ZCIS' : inst.id.split('-')[1] || 'TIPS',
    tenor: inst.tenor,
    rate: inst.rate,
  }));
  await jyBuildCurvesRaw();
}

// Watch chart type changes -- re-render when grid data available
watch(chartType, () => {
  if (buildResult.value?.short_term_grid) {
    updateCharts(buildResult.value, interpolation.value);
  }
});
</script>

<template>
  <div class="curve-builder-view">
    <!-- Summary Stats -->
    <div class="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
      <div
        v-for="stat in (isInflationCurve ? jyStore.summaryStats : summaryStats)"
        :key="stat.label"
        class="glass-card p-4"
      >
        <div class="flex items-start justify-between">
          <div>
            <p class="text-sm text-[var(--text-muted)] mb-1">{{ stat.label }}</p>
            <p class="text-xl font-semibold text-[var(--text-primary)]">{{ stat.value }}</p>
          </div>
          <div
            class="w-9 h-9 rounded-lg flex items-center justify-center"
            :style="{ backgroundColor: `${stat.color}1a` }"
          >
            <i :class="['fas', stat.icon, 'text-sm']" :style="{ color: stat.color }"></i>
          </div>
        </div>
      </div>
    </div>

    <!-- Asset Type Tabs -->
    <AssetTabBar
      v-model="assetTab"
      class="mb-6"
      :tabs="[
        { key: 'rate', label: 'Rate', icon: 'fa-chart-line' },
        { key: 'credit', label: 'Credit', icon: 'fa-shield-halved' },
        { key: 'fx', label: 'FX', icon: 'fa-exchange-alt' },
        { key: 'inflation', label: 'Inflation', icon: 'fa-chart-bar' },
      ]"
    />

    <!-- Unified Curve Builder (all asset types) -->
    <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <!-- Left Panel -->
      <div class="space-y-4">
        <!-- Curve Selector -->
        <div class="glass-card p-5">
          <div class="section-header" style="margin-top: 0">Curve Selection</div>

          <!-- Error Message -->
          <div v-if="loadError" class="mb-3 p-2 rounded bg-red-500/20 border border-red-500/50">
            <p class="text-xs text-red-400">{{ loadError }}</p>
          </div>

          <!-- Curve dropdown (all tabs) -->
          <div class="config-grid">
            <div class="grid-label">Curve</div>
            <div class="grid-input">
              <v-select
                v-model="selectedCurveName"
                :items="curveOptions.filter(c => c.curveType === assetTab).map(c => ({ title: c.name, value: c.name }))"
                :placeholder="curvesConfig ? 'Select curve...' : 'Loading...'"
                :disabled="!curvesConfig"
                density="compact"
                variant="outlined"
                hide-details
              />
            </div>
          </div>
        </div>

        <!-- Instruments Table (all curve types) -->
        <CurveInstrumentTable
          :instruments="instruments"
          :is-loading="isLoading"
          @toggle="toggleEnabled"
          @toggle-all="toggleAll"
          @update-rate="updateRate"
          @update-spike="updateSpike"
          @update-pips="updatePips"
          @update-coupon="updateCoupon"
        />

        <!-- Build Settings -->
        <div class="glass-card p-5">
          <div class="section-header" style="margin-top: 0">Build Settings</div>
          <div class="config-grid">
            <!-- Inflation: Nominal Curve + JY Model Parameters -->
            <template v-if="isInflationCurve">
              <div class="grid-label">Nominal</div>
              <div class="grid-input">
                <v-select
                  v-model="nominalCurveOverride"
                  :items="availableRateCurves"
                  placeholder="Select nominal curve..."
                  density="compact"
                  variant="outlined"
                  hide-details
                />
              </div>
              <div class="grid-label">Model</div>
              <div class="grid-input">
                <span class="text-sm text-[var(--text-primary)]">Jarrow-Yildirim</span>
              </div>
              <div class="grid-label">a<sub>N</sub> / &sigma;<sub>N</sub></div>
              <div class="grid-input">
                <span class="text-sm text-[var(--text-primary)] font-mono">{{ jyStore.modelParams.aN }} / {{ jyStore.modelParams.sigmaN }}</span>
              </div>
              <div class="grid-label">a<sub>R</sub> / &sigma;<sub>R</sub></div>
              <div class="grid-input">
                <span class="text-sm text-[var(--text-primary)] font-mono">{{ jyStore.modelParams.aR }} / {{ jyStore.modelParams.sigmaR }}</span>
              </div>
              <div class="grid-label">&sigma;<sub>I</sub></div>
              <div class="grid-input">
                <span class="text-sm text-[var(--text-primary)] font-mono">{{ jyStore.modelParams.sigmaI }}</span>
              </div>
              <div class="grid-label">&rho; (N-R / N-I / R-I)</div>
              <div class="grid-input">
                <span class="text-sm text-[var(--text-primary)] font-mono">{{ jyStore.correlation.rhoNr }} / {{ jyStore.correlation.rhoNi }} / {{ jyStore.correlation.rhoRi }}</span>
              </div>
              <div class="grid-span">
                <p class="text-xs text-[var(--text-muted)]">
                  <i class="fas fa-info-circle mr-1"></i>
                  Edit in Vol Surface &gt; Inflation tab
                </p>
              </div>
              <!-- Initial Conditions -->
              <div class="grid-span">
                <div class="section-header" style="margin-top: 0.5rem">Initial Conditions</div>
              </div>
              <div class="grid-label">Valuation Date</div>
              <div class="grid-input">
                <input v-model="jyStore.valuationDate" type="date"
                  class="w-full px-2 py-1.5 text-sm rounded border border-[var(--glass-border)] bg-[var(--surface)] text-[var(--text-primary)]" />
              </div>
              <div class="grid-label">r<sub>N</sub>(0)</div>
              <div class="grid-input">
                <input v-model.number="jyStore.initialNominalRate" type="number" step="0.001"
                  class="w-full px-2 py-1.5 text-sm rounded border border-[var(--glass-border)] bg-[var(--surface)] text-[var(--text-primary)]" />
              </div>
              <div class="grid-label">r<sub>R</sub>(0)</div>
              <div class="grid-input">
                <input v-model.number="jyStore.initialRealRate" type="number" step="0.001"
                  class="w-full px-2 py-1.5 text-sm rounded border border-[var(--glass-border)] bg-[var(--surface)] text-[var(--text-primary)]" />
              </div>
              <div class="grid-label">I(0)</div>
              <div class="grid-input">
                <input v-model.number="jyStore.initialIndex" type="number" step="1" min="1"
                  class="w-full px-2 py-1.5 text-sm rounded border border-[var(--glass-border)] bg-[var(--surface)] text-[var(--text-primary)]" />
              </div>
            </template>
            <!-- FX -->
            <template v-else-if="isFxCurve">
              <div class="grid-label">Method</div>
              <div class="grid-input">
                <span class="text-sm text-[var(--text-primary)]">{{ selectedCurve?.fxCurveMethod === 'irp_generic' ? 'Interest Rate Parity' : selectedCurve?.fxCurveMethod === 'irp_basis' ? 'XCCY Basis + IR Curve' : 'Flat Forward Points' }}</span>
              </div>
              <template v-if="selectedCurve?.fxCurveMethod === 'irp_generic'">
                <div class="grid-label">Domestic</div>
                <div class="grid-input">
                  <v-select
                    v-model="domesticCurveOverride"
                    :items="availableRateCurves"
                    placeholder="Select domestic curve..."
                    density="compact"
                    variant="outlined"
                    hide-details
                  />
                </div>
                <div class="grid-label">Foreign</div>
                <div class="grid-input">
                  <v-select
                    v-model="foreignCurveOverride"
                    :items="availableRateCurves"
                    placeholder="Select foreign curve..."
                    density="compact"
                    variant="outlined"
                    hide-details
                  />
                </div>
              </template>
              <template v-if="selectedCurve?.fxCurveMethod === 'irp_basis'">
                <div class="grid-label">Reference</div>
                <div class="grid-input">
                  <v-select
                    v-model="referenceCurveOverride"
                    :items="availableRateCurves"
                    placeholder="Select reference curve..."
                    density="compact"
                    variant="outlined"
                    hide-details
                  />
                </div>
              </template>
            </template>
            <!-- Rate / Credit -->
            <template v-else>
              <template v-if="isCreditCurve">
                <div class="grid-label">Discount</div>
                <div class="grid-input">
                  <v-select
                    v-model="discountCurveOverride"
                    :items="availableRateCurves"
                    placeholder="Select discount curve..."
                    density="compact"
                    variant="outlined"
                    hide-details
                  />
                </div>
              </template>
              <div class="grid-label">Calibration</div>
              <div class="grid-input">
                <v-select
                  v-model="calibrationMethod"
                  :items="calibrationMethods.map(m => ({ title: m.label, value: m.value }))"
                  density="compact"
                  variant="outlined"
                  hide-details
                />
              </div>
              <div class="grid-label">Interpolation</div>
              <div class="grid-input">
                <v-select
                  v-model="interpolation"
                  :items="annotatedInterpolationMethods.map(m => ({ title: m.displayLabel, value: m.value }))"
                  density="compact"
                  variant="outlined"
                  hide-details
                />
              </div>
              <div v-if="compatibilityHint" class="grid-span">
                <p
                  class="text-xs px-2 py-1.5 rounded"
                  :class="{
                    'text-emerald-400 bg-emerald-500/10': compatibilityHint.level === 'good',
                    'text-amber-400 bg-amber-500/10': compatibilityHint.level === 'warn',
                    'text-sky-400 bg-sky-500/10': compatibilityHint.level === 'info',
                  }"
                >
                  {{ compatibilityHint.message }}
                </p>
              </div>
              <div class="grid-label">Extrap.</div>
              <div class="grid-input">
                <v-switch v-model="allowExtrapolation" color="primary" density="compact" hide-details />
              </div>
            </template>
          </div>
        </div>

        <!-- Actions -->
        <div class="glass-card p-5">
          <!-- Build button: inflation -->
          <button
            v-if="isInflationCurve"
            class="w-full px-4 py-2.5 rounded-lg bg-[var(--primary)] text-white font-medium hover:opacity-90 disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
            :disabled="jyStore.loading || !nominalCurveOverride || enabledInstruments.length === 0"
            @click="buildInflationCurve"
          >
            <i :class="['fas', jyStore.loading ? 'fa-spinner fa-spin' : 'fa-hammer']"></i>
            {{ jyStore.loading ? 'Building...' : 'Build Curve' }}
          </button>
          <!-- Build button: standard -->
          <button
            v-else
            :disabled="!selectedCurve || (!isFxCurve && enabledInstruments.length === 0) || isBuilding"
            class="w-full px-4 py-2.5 rounded-lg bg-[var(--primary)] text-white font-medium hover:opacity-90 disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
            @click="buildCurve"
          >
            <i :class="['fas', isBuilding ? 'fa-spinner fa-spin' : 'fa-hammer']"></i>
            {{ isBuilding ? 'Building...' : 'Build Curve' }}
          </button>
          <button
            v-if="!isInflationCurve"
            :disabled="!buildResult || publishFeedback"
            class="w-full mt-2 px-4 py-2 rounded-lg bg-emerald-600 text-white text-sm font-medium hover:opacity-90 disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
            @click="publishToEnvironment"
          >
            <i :class="['fas', publishFeedback ? 'fa-check' : 'fa-cloud-upload-alt']"></i>
            {{ publishFeedback ? 'Published!' : 'Publish to Environment' }}
          </button>
          <div v-if="!isInflationCurve" class="grid grid-cols-2 gap-2 mt-2">
            <button
              :disabled="!hasChanges"
              class="px-3 py-1.5 rounded bg-[var(--surface)] text-[var(--text-secondary)] text-sm hover:bg-[var(--surface-hover)] disabled:opacity-50"
              @click="resetSettings"
            >
              <i class="fas fa-undo mr-1"></i>Reset
            </button>
            <button
              :disabled="instruments.length === 0"
              class="px-3 py-1.5 rounded bg-[var(--surface)] text-[var(--text-secondary)] text-sm hover:bg-[var(--surface-hover)] disabled:opacity-50"
              @click="exportRates"
            >
              <i class="fas fa-download mr-1"></i>Export
            </button>
          </div>

          <div v-if="hasChanges && !isInflationCurve" class="mt-3 p-2 rounded bg-[#f59e0b1a] border border-[var(--warning)]">
            <p class="text-xs text-[var(--warning)] flex items-center gap-1">
              <i class="fas fa-exclamation-triangle"></i>
              Rebuild required
            </p>
          </div>
        </div>
      </div>

      <!-- Right Panel: Charts + Data -->
      <div class="lg:col-span-2 space-y-6">
        <div class="glass-card p-6">
          <div class="flex items-center justify-between mb-4">
            <h3 class="text-lg font-semibold text-[var(--text-primary)]">
              {{ assetTab === 'inflation' ? 'Inflation Curve' : isFxCurve ? 'FX Forward Curve' : isCreditCurve ? 'Credit Curve' : 'Yield Curve' }}
            </h3>
            <!-- Chart type toggles (standard only) -->
            <div v-if="assetTab !== 'inflation' && buildResult?.short_term_grid" class="flex gap-2">
              <template v-if="isFxCurve">
                <button
                  :class="[
                    'px-3 py-1.5 text-xs rounded-lg transition-colors',
                    chartType === 'forward_rate' ? 'bg-cyan-500 text-white' : 'bg-[var(--surface)] text-[var(--text-secondary)]'
                  ]"
                  @click="chartType = 'forward_rate'"
                >FX Forward Rate</button>
                <button
                  :class="[
                    'px-3 py-1.5 text-xs rounded-lg transition-colors',
                    chartType === 'fx_basis' ? 'bg-emerald-500 text-white' : 'bg-[var(--surface)] text-[var(--text-secondary)]'
                  ]"
                  @click="chartType = 'fx_basis'"
                >Implied Yield</button>
                <button
                  :class="[
                    'px-3 py-1.5 text-xs rounded-lg transition-colors',
                    chartType === 'fx_overnight' ? 'bg-amber-500 text-white' : 'bg-[var(--surface)] text-[var(--text-secondary)]'
                  ]"
                  @click="chartType = 'fx_overnight'"
                >Implied Rate</button>
              </template>
              <template v-else>
                <button
                  :class="[
                    'px-3 py-1.5 text-xs rounded-lg transition-colors',
                    chartType === 'forward_rate' ? 'bg-emerald-500 text-white' : 'bg-[var(--surface)] text-[var(--text-secondary)]'
                  ]"
                  @click="chartType = 'forward_rate'"
                >
                  {{ isCreditCurve ? 'Hazard Rate' : 'Forward Rate' }}
                </button>
                <button
                  :class="[
                    'px-3 py-1.5 text-xs rounded-lg transition-colors',
                    chartType === 'discount_factor' ? 'bg-[var(--primary)] text-white' : 'bg-[var(--surface)] text-[var(--text-secondary)]'
                  ]"
                  @click="chartType = 'discount_factor'"
                >
                  {{ isCreditCurve ? 'Survival Prob' : 'Discount Factor' }}
                </button>
              </template>
            </div>
          </div>

          <!-- Build Error (standard) -->
          <div v-if="buildError && assetTab !== 'inflation'" class="mb-4 p-3 rounded-lg bg-red-500/20 border border-red-500/50">
            <p class="text-sm text-red-400 flex items-center gap-2">
              <i class="fas fa-exclamation-circle"></i>
              {{ buildError }}
            </p>
          </div>

          <!-- Empty State -->
          <div
            v-if="assetTab === 'inflation' ? !jyStore.curveResult : (!buildResult && !buildError)"
            class="flex flex-col items-center justify-center h-[500px] text-[var(--text-muted)]"
          >
            <i :class="['fas text-5xl mb-4 opacity-30', assetTab === 'inflation' ? 'fa-chart-bar' : 'fa-chart-line']"></i>
            <p class="text-sm">Build a curve to see the chart</p>
          </div>

          <!-- Inflation Charts -->
          <div v-else-if="assetTab === 'inflation' && jyStore.curveResult" class="space-y-4">
            <div>
              <h4 class="text-sm font-medium text-[var(--text-secondary)] mb-2">
                <i class="fas fa-chart-line text-xs mr-1"></i>Zero Rate Curves (Nominal / Real / Breakeven)
              </h4>
              <div class="h-48 bg-[var(--surface)] rounded-lg p-2">
                <canvas ref="inflationCurveCanvas"></canvas>
              </div>
            </div>
            <div>
              <h4 class="text-sm font-medium text-[var(--text-secondary)] mb-2">
                <i class="fas fa-chart-area text-xs mr-1"></i>Discount Factors (Nominal / Real)
              </h4>
              <div class="h-48 bg-[var(--surface)] rounded-lg p-2">
                <canvas ref="inflationDfCanvas"></canvas>
              </div>
            </div>
          </div>

          <!-- Standard Charts: Short-term (top) and Long-term (bottom) -->
          <div v-else-if="assetTab !== 'inflation'" class="space-y-4">
            <div>
              <h4 class="text-sm font-medium text-[var(--text-secondary)] mb-2">
                <i class="fas fa-clock text-xs mr-1"></i>Short Term (0-1Y)
              </h4>
              <div class="h-48 bg-[var(--surface)] rounded-lg p-2">
                <canvas ref="shortTermChartCanvas"></canvas>
              </div>
            </div>

            <div>
              <h4 class="text-sm font-medium text-[var(--text-secondary)] mb-2">
                <i class="fas fa-calendar-alt text-xs mr-1"></i>Long Term (0-30Y)
              </h4>
              <div class="h-48 bg-[var(--surface)] rounded-lg p-2">
                <canvas ref="longTermChartCanvas"></canvas>
              </div>
            </div>
          </div>

          <!-- Inflation Build Info -->
          <div v-if="assetTab === 'inflation' && jyStore.curveResult" class="mt-4 pt-4 border-t border-[var(--glass-border)]">
            <div class="grid grid-cols-3 gap-4 text-sm">
              <div>
                <span class="text-[var(--text-muted)]">Model:</span>
                <span class="ml-2 text-[var(--text-primary)] font-medium">Jarrow-Yildirim</span>
              </div>
              <div>
                <span class="text-[var(--text-muted)]">Nominal Points:</span>
                <span class="ml-2 text-[var(--text-primary)] font-medium">{{ jyStore.curveResult.nominalCurve.length }}</span>
              </div>
              <div>
                <span class="text-[var(--text-muted)]">Real Points:</span>
                <span class="ml-2 text-[var(--text-primary)] font-medium">{{ jyStore.curveResult.realCurve.length }}</span>
              </div>
            </div>
          </div>

          <!-- Standard Build Info -->
          <div v-if="assetTab !== 'inflation' && buildResult" class="mt-4 pt-4 border-t border-[var(--glass-border)]">
            <div class="grid grid-cols-4 gap-4 text-sm">
              <div>
                <span class="text-[var(--text-muted)]">{{ isCreditCurve ? 'Type:' : 'Instruments:' }}</span>
                <span class="ml-2 text-[var(--text-primary)] font-medium">{{ isCreditCurve ? 'Credit' : buildResult.instrument_count }}</span>
              </div>
              <div>
                <span class="text-[var(--text-muted)]">Method:</span>
                <span class="ml-2 text-[var(--text-primary)] font-medium">{{ calibrationMethods.find(m => m.value === (buildResult?.bootstrap_method ?? calibrationMethod))?.label ?? buildResult?.bootstrap_method }}</span>
              </div>
              <div>
                <span class="text-[var(--text-muted)]">Interpolation:</span>
                <span class="ml-2 text-[var(--text-primary)] font-medium">{{ buildResult.interpolation }}</span>
              </div>
              <div>
                <span class="text-[var(--text-muted)]">Time:</span>
                <span class="ml-2 text-[var(--text-primary)] font-medium">{{ buildResult.calculation_time_ms?.toFixed(2) }} ms</span>
              </div>
            </div>
          </div>

          <!-- Inflation Data Table -->
          <div v-if="assetTab === 'inflation' && jyStore.curveResult" class="mt-4 pt-4 border-t border-[var(--glass-border)]">
            <h4 class="text-sm font-medium text-[var(--text-secondary)] mb-3">
              <i class="fas fa-table text-xs mr-1"></i>
              Curve Data ({{ jyStore.curveResult.nominalCurve.length }} points)
            </h4>
            <div class="max-h-64 overflow-y-auto">
              <table class="w-full text-sm">
                <thead class="sticky top-0 z-10">
                  <tr class="border-b border-[var(--glass-border)] curve-table-header">
                    <th class="text-left py-2 px-2 text-xs font-medium text-[var(--text-muted)]">Tenor (Y)</th>
                    <th class="text-right py-2 px-2 text-xs font-medium text-[var(--text-muted)]">Nominal (%)</th>
                    <th class="text-right py-2 px-2 text-xs font-medium text-[var(--text-muted)]">Real (%)</th>
                    <th class="text-right py-2 px-2 text-xs font-medium text-[var(--text-muted)]">Breakeven (%)</th>
                    <th class="text-right py-2 px-2 text-xs font-medium text-[var(--text-muted)]">DF (Nom)</th>
                  </tr>
                </thead>
                <tbody>
                  <tr
                    v-for="(pt, idx) in jyStore.curveResult.nominalCurve"
                    :key="idx"
                    class="border-b border-[var(--glass-border)] hover:bg-[var(--surface-hover)] transition-colors"
                  >
                    <td class="py-1.5 px-2 text-xs text-[var(--text-primary)] font-mono">{{ pt.tenor.toFixed(1) }}</td>
                    <td class="py-1.5 px-2 text-xs text-right font-mono text-blue-400">{{ (pt.value * 100).toFixed(4) }}</td>
                    <td class="py-1.5 px-2 text-xs text-right font-mono text-green-400">{{ jyStore.curveResult!.realCurve[idx] ? (jyStore.curveResult!.realCurve[idx].value * 100).toFixed(4) : '-' }}</td>
                    <td class="py-1.5 px-2 text-xs text-right font-mono text-yellow-400">{{ jyStore.curveResult!.breakevenCurve[idx] ? (jyStore.curveResult!.breakevenCurve[idx].value * 100).toFixed(4) : '-' }}</td>
                    <td class="py-1.5 px-2 text-xs text-right font-mono text-[var(--text-primary)]">{{ jyStore.curveResult!.nominalDf[idx]?.value.toFixed(8) ?? '-' }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>

          <!-- Standard Pillar Data Table -->
          <div v-if="assetTab !== 'inflation' && curveTableRows.length > 0" class="mt-4 pt-4 border-t border-[var(--glass-border)]">
            <h4 class="text-sm font-medium text-[var(--text-secondary)] mb-3">
              <i class="fas fa-table text-xs mr-1"></i>
              Curve Data ({{ curveTableRows.length }} points)
            </h4>
            <div class="max-h-64 overflow-y-auto">
              <table class="w-full text-sm">
                <thead class="sticky top-0 z-10">
                  <tr class="border-b border-[var(--glass-border)] curve-table-header">
                    <th class="text-left py-2 px-2 text-xs font-medium text-[var(--text-muted)]">Date</th>
                    <th class="text-right py-2 px-2 text-xs font-medium text-[var(--text-muted)]">Time (Y)</th>
                    <th class="text-right py-2 px-2 text-xs font-medium text-[var(--text-muted)]">{{ isFxCurve ? 'FX Forward' : isCreditCurve ? 'Hazard Rate (%)' : 'Fwd Rate (%)' }}</th>
                    <th class="text-right py-2 px-2 text-xs font-medium text-[var(--text-muted)]">{{ isFxCurve ? 'Fwd Pts' : isCreditCurve ? 'SP' : 'DF' }}</th>
                  </tr>
                </thead>
                <tbody>
                  <tr
                    v-for="(row, idx) in curveTableRows"
                    :key="idx"
                    class="border-b border-[var(--glass-border)] hover:bg-[var(--surface-hover)] transition-colors"
                  >
                    <td class="py-1.5 px-2 text-xs text-[var(--text-primary)] font-mono">{{ row.date }}</td>
                    <td class="py-1.5 px-2 text-xs text-right text-[var(--text-secondary)] font-mono">{{ row.time.toFixed(4) }}</td>
                    <td class="py-1.5 px-2 text-xs text-right font-mono text-emerald-400">{{ isFxCurve ? row.fwd.toFixed(4) : (row.fwd * 100).toFixed(4) }}</td>
                    <td class="py-1.5 px-2 text-xs text-right font-mono text-[var(--text-primary)]">{{ isFxCurve ? (row.fwd - (buildResult?.spot ?? row.fwd)).toFixed(6) : row.df.toFixed(8) }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>

        <!-- Jacobian Card (below Yield Curve, same width; not for inflation or FX) -->
        <CurveJacobianHeatmap
          v-if="assetTab !== 'inflation' && buildResult?.jacobian && !isFxCurve"
          :jacobian="buildResult.jacobian"
        />
      </div>
    </div>
  </div>
</template>

<style scoped>
.curve-table-header {
  background: var(--surface);
  box-shadow: 0 1px 0 var(--glass-border);
}

.curve-table-header th {
  background: inherit;
}
</style>
