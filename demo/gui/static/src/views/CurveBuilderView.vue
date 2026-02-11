<script setup lang="ts">
import { watch } from 'vue';
import { useCurveBuilder, calibrationMethods, interpolationMethods } from '@/composables/useCurveBuilder';
import { useCurveCharts } from '@/composables/useCurveCharts';
import CurveInstrumentTable from '@/components/curve/CurveInstrumentTable.vue';
import CurveJacobianHeatmap from '@/components/curve/CurveJacobianHeatmap.vue';

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
} = useCurveBuilder(() => {
  if (buildResult.value) {
    updateCharts(buildResult.value, interpolation.value);
  }
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
          <h3 class="text-base font-semibold text-[var(--text-primary)] mb-3">Curve Selection</h3>

          <!-- Error Message -->
          <div v-if="loadError" class="mb-3 p-2 rounded bg-red-500/20 border border-red-500/50">
            <p class="text-xs text-red-400">{{ loadError }}</p>
          </div>

          <select
            v-model="selectedCurveName"
            :disabled="!curvesConfig"
            class="w-full px-3 py-2 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-[var(--primary)] disabled:opacity-50"
          >
            <option value="">{{ curvesConfig ? 'Select curve...' : 'Loading...' }}</option>
            <option v-for="curve in curveOptions" :key="curve.name" :value="curve.name">
              {{ curve.name }}
            </option>
          </select>

        </div>

        <!-- Instruments Table -->
        <CurveInstrumentTable
          :instruments="instruments"
          :is-loading="isLoading"
          @toggle="toggleEnabled"
          @toggle-all="toggleAll"
          @update-rate="updateRate"
          @update-spike="updateSpike"
        />

        <!-- Build Settings -->
        <div class="glass-card p-5">
          <h3 class="text-base font-semibold text-[var(--text-primary)] mb-3">Build Settings</h3>
          <div class="space-y-3">
            <div>
              <label class="block text-xs text-[var(--text-muted)] mb-1">Calibration</label>
              <select
                v-model="calibrationMethod"
                class="w-full px-2 py-1.5 rounded bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm"
              >
                <option v-for="m in calibrationMethods" :key="m.value" :value="m.value">{{ m.label }}</option>
              </select>
            </div>
            <div>
              <label class="block text-xs text-[var(--text-muted)] mb-1">Interpolation</label>
              <select
                v-model="interpolation"
                class="w-full px-2 py-1.5 rounded bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm"
              >
                <option v-for="m in interpolationMethods" :key="m.value" :value="m.value">{{ m.label }}</option>
              </select>
            </div>
            <label class="flex items-center gap-2 cursor-pointer">
              <input v-model="allowExtrapolation" type="checkbox" class="w-4 h-4 rounded">
              <span class="text-sm text-[var(--text-secondary)]">Extrapolation</span>
            </label>
          </div>
        </div>

        <!-- Actions -->
        <div class="glass-card p-5">
          <button
            :disabled="!selectedCurve || enabledInstruments.length === 0 || isBuilding"
            class="w-full px-4 py-2.5 rounded-lg bg-[var(--primary)] text-white font-medium hover:opacity-90 disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
            @click="buildCurve"
          >
            <i :class="['fas', isBuilding ? 'fa-spinner fa-spin' : 'fa-hammer']"></i>
            {{ isBuilding ? 'Building...' : 'Build Curve' }}
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
            <h3 class="text-lg font-semibold text-[var(--text-primary)]">Yield Curve</h3>
            <div v-if="buildResult?.short_term_grid" class="flex gap-2">
              <button
                :class="[
                  'px-3 py-1.5 text-xs rounded-lg transition-colors',
                  chartType === 'forward_rate' ? 'bg-emerald-500 text-white' : 'bg-[var(--surface)] text-[var(--text-secondary)]'
                ]"
                @click="chartType = 'forward_rate'"
              >
                Forward Rate
              </button>
              <button
                :class="[
                  'px-3 py-1.5 text-xs rounded-lg transition-colors',
                  chartType === 'discount_factor' ? 'bg-[var(--primary)] text-white' : 'bg-[var(--surface)] text-[var(--text-secondary)]'
                ]"
                @click="chartType = 'discount_factor'"
              >
                Discount Factor
              </button>
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
                <span class="text-[var(--text-muted)]">Instruments:</span>
                <span class="ml-2 text-[var(--text-primary)] font-medium">{{ buildResult.instrument_count }}</span>
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
                    <th class="text-right py-2 px-2 text-xs font-medium text-[var(--text-muted)]">Fwd Rate (%)</th>
                    <th class="text-right py-2 px-2 text-xs font-medium text-[var(--text-muted)]">DF</th>
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
                    <td class="py-1.5 px-2 text-xs text-right font-mono text-emerald-400">{{ (row.fwd * 100).toFixed(4) }}</td>
                    <td class="py-1.5 px-2 text-xs text-right font-mono text-[var(--text-primary)]">{{ row.df.toFixed(8) }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>

        <!-- Jacobian Card (below Yield Curve, same width) -->
        <CurveJacobianHeatmap
          v-if="buildResult?.jacobian"
          :jacobian="buildResult.jacobian"
        />
      </div>
    </div>
  </div>
</template>

<style scoped>
.glass-card {
  background: var(--glass-bg);
  backdrop-filter: blur(20px);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-lg);
  box-shadow: var(--glass-shadow);
}

.curve-table-header {
  background: var(--surface);
  box-shadow: 0 1px 0 var(--glass-border);
}

.curve-table-header th {
  background: inherit;
}
</style>
