<script setup lang="ts">
import { ref, watch } from 'vue';
import { useCurveBuilder, calibrationMethods } from '@/composables/useCurveBuilder';
import { useCurveCharts } from '@/composables/useCurveCharts';
import { useMarketEnvStore } from '@/stores/marketEnv';
import CurveInstrumentTable from '@/components/curve/CurveInstrumentTable.vue';
import CurveJacobianHeatmap from '@/components/curve/CurveJacobianHeatmap.vue';
import { useJyInflationStore } from '@/stores/jyInflation';
import { useJYInflation } from '@/composables/useJYInflation';
import JyCurvePanel from '@/components/jy/JyCurvePanel.vue';

const jyStore = useJyInflationStore();
const { buildCurves: jyBuildCurves } = useJYInflation();

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

  // Computed
  curveOptions,
  enabledInstruments,
  hasChanges,
  isCreditCurve,
  isFxCurve,
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
  if (c) assetTab.value = (c.curveType ?? 'rate') as 'rate' | 'credit' | 'fx';
});

// Auto-select the first curve when switching tabs
watch(assetTab, (tab) => {
  const first = curveOptions.value.find(c => c.curveType === tab);
  selectedCurveName.value = first?.name ?? '';
  // Reset chart type when switching asset tabs
  chartType.value = 'forward_rate';
});

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
        v-for="stat in summaryStats"
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

    <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <!-- Left Panel: Settings -->
      <div class="space-y-4">
        <!-- Curve Selector -->
        <div class="glass-card p-5">
          <div class="section-header" style="margin-top: 0">Curve Selection</div>

          <!-- Asset Type Tabs -->
          <div class="flex gap-1 mb-3 p-0.5 rounded-lg bg-[var(--surface)]">
            <button
              v-for="tab in [
                { key: 'rate', label: 'Rate', icon: 'fa-chart-line' },
                { key: 'credit', label: 'Credit', icon: 'fa-shield-halved' },
                { key: 'fx', label: 'FX', icon: 'fa-exchange-alt' },
                { key: 'inflation', label: 'Inflation', icon: 'fa-chart-bar' },
              ]"
              :key="tab.key"
              class="flex-1 px-3 py-1.5 rounded-md text-xs font-medium transition-all flex items-center justify-center gap-1.5"
              :class="assetTab === tab.key
                ? 'bg-[var(--primary)] text-white shadow-sm'
                : 'text-[var(--text-muted)] hover:text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]'"
              @click="assetTab = tab.key as 'rate' | 'credit' | 'fx' | 'inflation'"
            >
              <i :class="['fas', tab.icon]" style="font-size: 10px"></i>
              {{ tab.label }}
            </button>
          </div>

          <!-- Error Message -->
          <div v-if="loadError" class="mb-3 p-2 rounded bg-red-500/20 border border-red-500/50">
            <p class="text-xs text-red-400">{{ loadError }}</p>
          </div>

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

        <!-- Instruments Table -->
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
            <template v-if="isFxCurve">
              <div class="grid-label">Method</div>
              <div class="grid-input">
                <span class="text-sm text-[var(--text-primary)]">{{ selectedCurve?.fxCurveMethod === 'irp_generic' ? 'Interest Rate Parity' : selectedCurve?.fxCurveMethod === 'irp_basis' ? 'XCCY Basis + IR Curve' : 'Flat Forward Points' }}</span>
              </div>
              <template v-if="selectedCurve?.domesticCurve">
                <div class="grid-label">Domestic</div>
                <div class="grid-input">
                  <span class="text-sm text-[var(--text-primary)]">{{ selectedCurve.domesticCurve }}</span>
                </div>
              </template>
              <template v-if="selectedCurve?.foreignCurve">
                <div class="grid-label">Foreign</div>
                <div class="grid-input">
                  <span class="text-sm text-[var(--text-primary)]">{{ selectedCurve.foreignCurve }}</span>
                </div>
              </template>
              <template v-if="selectedCurve?.referenceCurve">
                <div class="grid-label">Reference</div>
                <div class="grid-input">
                  <span class="text-sm text-[var(--text-primary)]">{{ selectedCurve.referenceCurve }}</span>
                </div>
              </template>
            </template>
            <template v-else>
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
          <button
            :disabled="!selectedCurve || (!isFxCurve && enabledInstruments.length === 0) || isBuilding"
            class="w-full px-4 py-2.5 rounded-lg bg-[var(--primary)] text-white font-medium hover:opacity-90 disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
            @click="buildCurve"
          >
            <i :class="['fas', isBuilding ? 'fa-spinner fa-spin' : 'fa-hammer']"></i>
            {{ isBuilding ? 'Building...' : 'Build Curve' }}
          </button>
          <button
            :disabled="!buildResult || publishFeedback"
            class="w-full mt-2 px-4 py-2 rounded-lg bg-emerald-600 text-white text-sm font-medium hover:opacity-90 disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
            @click="publishToEnvironment"
          >
            <i :class="['fas', publishFeedback ? 'fa-check' : 'fa-cloud-upload-alt']"></i>
            {{ publishFeedback ? 'Published!' : 'Publish to Environment' }}
          </button>
          <div class="grid grid-cols-2 gap-2 mt-2">
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

          <div v-if="hasChanges" class="mt-3 p-2 rounded bg-[#f59e0b1a] border border-[var(--warning)]">
            <p class="text-xs text-[var(--warning)] flex items-center gap-1">
              <i class="fas fa-exclamation-triangle"></i>
              Rebuild required
            </p>
          </div>
        </div>
      </div>

      <!-- Right Panel: Curve Chart + Jacobian -->
      <div class="lg:col-span-2 space-y-6">
        <div class="glass-card p-6">
          <div class="flex items-center justify-between mb-4">
            <h3 class="text-lg font-semibold text-[var(--text-primary)]">
              {{ isFxCurve ? 'FX Forward Curve' : isCreditCurve ? 'Credit Curve' : 'Yield Curve' }}
            </h3>
            <div v-if="buildResult?.short_term_grid" class="flex gap-2">
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

          <!-- Build Error -->
          <div v-if="buildError" class="mb-4 p-3 rounded-lg bg-red-500/20 border border-red-500/50">
            <p class="text-sm text-red-400 flex items-center gap-2">
              <i class="fas fa-exclamation-circle"></i>
              {{ buildError }}
            </p>
          </div>

          <!-- Empty State -->
          <div v-if="!buildResult && !buildError" class="flex flex-col items-center justify-center h-[500px] text-[var(--text-muted)]">
            <i class="fas fa-chart-line text-5xl mb-4 opacity-30"></i>
            <p class="text-sm">Build a curve to see the chart</p>
          </div>

          <!-- Charts: Short-term (top) and Long-term (bottom) -->
          <div v-else class="space-y-4">
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

          <!-- Build Info -->
          <div v-if="buildResult" class="mt-4 pt-4 border-t border-[var(--glass-border)]">
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

          <!-- Pillar Data Table -->
          <div v-if="curveTableRows.length > 0" class="mt-4 pt-4 border-t border-[var(--glass-border)]">
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

        <!-- Jacobian Card (below Yield Curve, same width) -->
        <CurveJacobianHeatmap
          v-if="buildResult?.jacobian && !isFxCurve"
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
