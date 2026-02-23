<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { Chart, registerables, type ChartConfiguration, type TooltipItem } from 'chart.js';
import { getChartColors } from '@/composables/useChartTheme';
import { useToast } from '@/composables/useToast';
import {
  fetchXvaConfig,
  runXvaSimulation,
  exportXvaCsv,
} from '@/services/api';
import type {
  XvaDefaultConfigResponse,
  XvaSimulationResponse,
  NettingSetResult,
} from '@/types';

Chart.register(...registerables);

const toast = useToast();

// State
const config = ref<XvaDefaultConfigResponse | null>(null);
const result = ref<XvaSimulationResponse | null>(null);
const loading = ref(false);
const exposureChart = ref<Chart | null>(null);
const waterfallChart = ref<Chart | null>(null);
const exposureCanvas = ref<HTMLCanvasElement | null>(null);
const waterfallCanvas = ref<HTMLCanvasElement | null>(null);
const selectedNettingSet = ref<string>('');

// Form state
const nPaths = ref(10000);
const horizonYears = ref(5);
const timeStep = ref('quarterly');
const antithetic = ref(true);
const bilateral = ref(true);
const computeFva = ref(true);
const seed = ref<number | undefined>(undefined);

// Series visibility
const visibleSeries = ref(new Set(['epe', 'ene', 'ecb', 'pfe95']));

const seriesConfig = [
  { key: 'epe', label: 'EPE', color: '#3b82f6', description: 'Expected Positive Exposure' },
  { key: 'ene', label: 'ENE', color: '#8b5cf6', description: 'Expected Negative Exposure' },
  { key: 'ecb', label: 'ECB', color: '#10b981', description: 'Expected Collateral Balance' },
  { key: 'pfe95', label: 'PFE 95%', color: '#ef4444', description: 'Potential Future Exposure (95%)' },
];

// Computed
const summaryStats = computed(() => {
  if (!result.value) return [];
  const cps = result.value.counterpartyResults;
  const totalCva = cps.reduce((s, c) => s + c.bcva, 0);
  const totalDva = cps.reduce((s, c) => s + c.bdva, 0);
  const totalFva = cps.reduce((s, c) => s + c.fva, 0);
  const netXva = cps.reduce((s, c) => s + c.totalXva, 0);
  return [
    { label: 'Total CVA', value: formatCurrency(totalCva), subtitle: `${cps.length} counterparties`, icon: 'fa-shield-alt', color: '#ef4444' },
    { label: 'Total DVA', value: formatCurrency(totalDva), subtitle: 'Bilateral', icon: 'fa-exchange-alt', color: '#3b82f6' },
    { label: 'Total FVA', value: formatCurrency(totalFva), subtitle: 'FCA + FBA', icon: 'fa-coins', color: '#f59e0b' },
    { label: 'Net XVA', value: formatCurrency(netXva), subtitle: `${result.value.config.nPaths.toLocaleString()} paths`, icon: 'fa-chart-bar', color: '#10b981' },
  ];
});

const selectedNsResult = computed<NettingSetResult | null>(() => {
  if (!result.value || !selectedNettingSet.value) return null;
  return result.value.nettingSets.find(ns => ns.nettingSetId === selectedNettingSet.value) ?? null;
});

// Methods
function formatCurrency(value: number): string {
  const abs = Math.abs(value);
  if (abs >= 1e6) return `${value >= 0 ? '' : '-'}$${(abs / 1e6).toFixed(2)}M`;
  if (abs >= 1e3) return `${value >= 0 ? '' : '-'}$${(abs / 1e3).toFixed(1)}K`;
  return `$${value.toFixed(2)}`;
}

async function loadConfig() {
  try {
    config.value = await fetchXvaConfig();
  } catch (e) {
    toast.error(`Failed to load XVA config: ${e instanceof Error ? e.message : 'Unknown error'}`);
  }
}

async function runSimulation() {
  loading.value = true;
  try {
    const request = {
      nPaths: nPaths.value,
      horizonYears: horizonYears.value,
      timeStep: timeStep.value,
      antithetic: antithetic.value,
      bilateral: bilateral.value,
      computeFva: computeFva.value,
      seed: seed.value,
    };
    result.value = await runXvaSimulation(request);
    if (result.value.nettingSets.length > 0) {
      selectedNettingSet.value = result.value.nettingSets[0].nettingSetId;
    }
    toast.success(`XVA simulation completed in ${result.value.computationTimeMs.toFixed(0)}ms`);
    renderExposureChart();
    renderWaterfallChart();
  } catch (e) {
    toast.error(`Simulation failed: ${e instanceof Error ? e.message : 'Unknown error'}`);
  } finally {
    loading.value = false;
  }
}

async function handleExportCsv() {
  try {
    const data = await exportXvaCsv();
    const blob = new Blob([data.csvData], { type: 'text/csv' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `xva_risk_indicators_${data.nettingSetId}.csv`;
    a.click();
    URL.revokeObjectURL(url);
    toast.success(`Exported ${data.rowCount} rows for ${data.nettingSetId}`);
  } catch (e) {
    toast.error(`Export failed: ${e instanceof Error ? e.message : 'Unknown error'}`);
  }
}

function toggleSeries(key: string) {
  if (visibleSeries.value.has(key)) {
    if (visibleSeries.value.size > 1) visibleSeries.value.delete(key);
  } else {
    visibleSeries.value.add(key);
  }
  updateExposureVisibility();
}

function updateExposureVisibility() {
  if (!exposureChart.value) return;
  exposureChart.value.data.datasets.forEach((dataset, index) => {
    dataset.hidden = !visibleSeries.value.has(seriesConfig[index].key);
  });
  exposureChart.value.update();
}

function renderExposureChart() {
  if (!exposureCanvas.value || !result.value || !selectedNsResult.value) return;

  if (exposureChart.value) {
    exposureChart.value.destroy();
    exposureChart.value = null;
  }

  const ns = selectedNsResult.value;
  const labels = result.value.timeGrid.map(t => `${t.toFixed(2)}Y`);
  const pfe95 = ns.pfe.find(p => p.percentile === 0.95)?.values ?? ns.epe.map(() => 0);

  const ctx = exposureCanvas.value.getContext('2d');
  if (!ctx) return;

  const cc = getChartColors();
  const config: ChartConfiguration<'line'> = {
    type: 'line',
    data: {
      labels,
      datasets: [
        { label: 'EPE', data: ns.epe, borderColor: '#3b82f6', backgroundColor: '#3b82f61a', fill: true, tension: 0.4, hidden: !visibleSeries.value.has('epe') },
        { label: 'ENE', data: ns.ene.map(v => -v), borderColor: '#8b5cf6', backgroundColor: '#8b5cf61a', fill: true, tension: 0.4, hidden: !visibleSeries.value.has('ene') },
        { label: 'ECB', data: ns.ecb, borderColor: '#10b981', backgroundColor: '#10b9811a', fill: true, tension: 0.4, hidden: !visibleSeries.value.has('ecb') },
        { label: 'PFE 95%', data: pfe95, borderColor: '#ef4444', backgroundColor: '#ef44441a', fill: true, tension: 0.4, hidden: !visibleSeries.value.has('pfe95') },
      ],
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      interaction: { intersect: false, mode: 'index' },
      plugins: {
        legend: { display: false },
        tooltip: {
          backgroundColor: cc.tooltipBg,
          titleColor: cc.tooltipTitle,
          bodyColor: cc.tooltipBody,
          padding: 12,
          cornerRadius: 8,
          callbacks: {
            label: (context: TooltipItem<'line'>) => {
              const value = context.parsed.y ?? 0;
              return `${context.dataset.label ?? ''}: ${formatCurrency(value)}`;
            },
          },
        },
      },
      scales: {
        x: { grid: { color: cc.grid }, ticks: { color: cc.tick } },
        y: { grid: { color: cc.grid }, ticks: { color: cc.tick, callback: (v) => formatCurrency(v as number) } },
      },
    },
  };

  exposureChart.value = new Chart(ctx, config);
}

function renderWaterfallChart() {
  if (!waterfallCanvas.value || !result.value) return;

  if (waterfallChart.value) {
    waterfallChart.value.destroy();
    waterfallChart.value = null;
  }

  const cps = result.value.counterpartyResults;
  const labels = cps.map(c => c.counterpartyId);

  const ctx = waterfallCanvas.value.getContext('2d');
  if (!ctx) return;

  const cc = getChartColors();
  const config: ChartConfiguration<'bar'> = {
    type: 'bar',
    data: {
      labels,
      datasets: [
        { label: 'BCVA', data: cps.map(c => c.bcva), backgroundColor: '#ef4444cc' },
        { label: 'BDVA', data: cps.map(c => -c.bdva), backgroundColor: '#3b82f6cc' },
        { label: 'FVA', data: cps.map(c => c.fva), backgroundColor: '#f59e0bcc' },
      ],
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      interaction: { intersect: false, mode: 'index' },
      plugins: {
        legend: {
          display: true,
          labels: { color: cc.tick },
        },
        tooltip: {
          backgroundColor: cc.tooltipBg,
          titleColor: cc.tooltipTitle,
          bodyColor: cc.tooltipBody,
          padding: 12,
          cornerRadius: 8,
          callbacks: {
            label: (context: TooltipItem<'bar'>) => {
              const value = context.parsed.y ?? 0;
              return `${context.dataset.label ?? ''}: ${formatCurrency(value)}`;
            },
          },
        },
      },
      scales: {
        x: { grid: { color: cc.grid }, ticks: { color: cc.tick } },
        y: { grid: { color: cc.grid }, ticks: { color: cc.tick, callback: (v) => formatCurrency(v as number) } },
      },
    },
  };

  waterfallChart.value = new Chart(ctx, config);
}

function onNettingSetChange() {
  renderExposureChart();
}

onMounted(() => loadConfig());
onUnmounted(() => {
  exposureChart.value?.destroy();
  waterfallChart.value?.destroy();
});
</script>

<template>
  <div class="xva-view">
    <!-- Header -->
    <div class="flex items-center justify-between mb-6">
      <div>
        <h2 class="text-2xl font-semibold text-[var(--text-primary)]">XVA Engine</h2>
        <p class="text-sm text-[var(--text-muted)]">Monte Carlo Full-Valuation XVA with Bilateral CVA/DVA/FVA</p>
      </div>
      <div class="flex gap-3">
        <button
          v-if="result"
          class="px-4 py-2 rounded-lg text-sm font-medium bg-[var(--surface)] text-[var(--text-secondary)] hover:bg-[var(--surface-hover)] transition-all"
          @click="handleExportCsv"
        >
          <i class="fas fa-download mr-2"></i>Export CSV
        </button>
      </div>
    </div>

    <!-- Main 2-column layout -->
    <div class="grid grid-cols-1 lg:grid-cols-12 gap-6">
      <!-- Left: Configuration Sidebar (4 cols) -->
      <div class="lg:col-span-4">
        <div class="glass-card p-4 lg:sticky lg:top-4 space-y-4">
          <h3 class="text-sm font-semibold text-[var(--text-primary)] flex items-center gap-2">
            <i class="fas fa-cog text-[var(--primary)]"></i>
            Simulation Configuration
          </h3>

          <div class="space-y-3">
            <div>
              <label class="text-xs text-[var(--text-muted)] mb-1 block">MC Paths</label>
              <input
                v-model.number="nPaths"
                type="number"
                min="100"
                max="100000"
                step="1000"
                class="config-input"
              >
            </div>
            <div>
              <label class="text-xs text-[var(--text-muted)] mb-1 block">Horizon</label>
              <select v-model.number="horizonYears" class="config-input">
                <option :value="3">3 Years</option>
                <option :value="5">5 Years</option>
                <option :value="7">7 Years</option>
                <option :value="10">10 Years</option>
              </select>
            </div>
            <div>
              <label class="text-xs text-[var(--text-muted)] mb-1 block">Time Step</label>
              <select v-model="timeStep" class="config-input">
                <option value="monthly">Monthly</option>
                <option value="quarterly">Quarterly</option>
                <option value="semi-annual">Semi-Annual</option>
              </select>
            </div>
            <div>
              <label class="text-xs text-[var(--text-muted)] mb-1 block">Seed (optional)</label>
              <input
                v-model.number="seed"
                type="number"
                placeholder="Random"
                class="config-input"
              >
            </div>

            <div class="border-t border-[var(--glass-border)] pt-3 space-y-2">
              <label class="flex items-center gap-2 cursor-pointer">
                <input v-model="antithetic" type="checkbox" class="rounded">
                <span class="text-sm text-[var(--text-secondary)]">Antithetic Variates</span>
              </label>
              <label class="flex items-center gap-2 cursor-pointer">
                <input v-model="bilateral" type="checkbox" class="rounded">
                <span class="text-sm text-[var(--text-secondary)]">Bilateral CVA/DVA</span>
              </label>
              <label class="flex items-center gap-2 cursor-pointer">
                <input v-model="computeFva" type="checkbox" class="rounded">
                <span class="text-sm text-[var(--text-secondary)]">Compute FVA</span>
              </label>
            </div>
          </div>

          <button
            :class="[
              'w-full px-4 py-2 rounded-lg text-sm font-medium transition-all',
              loading ? 'bg-gray-500 cursor-not-allowed' : 'bg-[var(--primary)] hover:opacity-90'
            ]"
            class="text-white"
            :disabled="loading"
            @click="runSimulation"
          >
            <i :class="['fas mr-2', loading ? 'fa-spinner fa-spin' : 'fa-play']"></i>
            {{ loading ? 'Simulating...' : 'Run Simulation' }}
          </button>
        </div>
      </div>

      <!-- Right: Results (8 cols) -->
      <div class="lg:col-span-8 space-y-6">
        <!-- Summary Stats (only after simulation) -->
        <div v-if="result" class="grid grid-cols-2 lg:grid-cols-4 gap-4">
          <div
            v-for="stat in summaryStats"
            :key="stat.label"
            class="glass-card p-4"
          >
            <div class="flex items-start justify-between">
              <div>
                <p class="text-sm text-[var(--text-muted)] mb-1">{{ stat.label }}</p>
                <p class="text-xl font-semibold text-[var(--text-primary)]">{{ stat.value }}</p>
                <p class="text-xs text-[var(--text-muted)] mt-1">{{ stat.subtitle }}</p>
              </div>
              <div
                class="w-9 h-9 rounded-lg flex items-center justify-center flex-shrink-0"
                :style="{ backgroundColor: `${stat.color}1a` }"
              >
                <i :class="['fas', stat.icon, 'text-sm']" :style="{ color: stat.color }"></i>
              </div>
            </div>
          </div>
        </div>

        <!-- Exposure Profile Chart -->
        <div v-if="result" class="glass-card p-6">
          <div class="flex items-center justify-between mb-4">
            <div>
              <h3 class="text-lg font-semibold text-[var(--text-primary)]">Exposure Profiles</h3>
              <p class="text-sm text-[var(--text-muted)]">Risk indicator time series by netting set</p>
            </div>
            <select
              v-model="selectedNettingSet"
              class="px-3 py-1.5 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm"
              @change="onNettingSetChange"
            >
              <option v-for="ns in result.nettingSets" :key="ns.nettingSetId" :value="ns.nettingSetId">
                {{ ns.nettingSetId }}
              </option>
            </select>
          </div>

          <!-- Series Legend -->
          <div class="flex flex-wrap gap-4 mb-4">
            <button
              v-for="series in seriesConfig"
              :key="series.key"
              :class="[
                'flex items-center gap-2 px-3 py-1.5 rounded-lg transition-all duration-200',
                visibleSeries.has(series.key)
                  ? 'bg-[var(--surface)] text-[var(--text-primary)]'
                  : 'text-[var(--text-muted)] opacity-50'
              ]"
              :title="series.description"
              @click="toggleSeries(series.key)"
            >
              <span class="w-3 h-3 rounded-full" :style="{ backgroundColor: series.color }"></span>
              <span class="text-sm font-medium">{{ series.label }}</span>
            </button>
          </div>

          <div class="h-80">
            <canvas ref="exposureCanvas"></canvas>
          </div>

          <!-- NS Summary -->
          <div v-if="selectedNsResult" class="grid grid-cols-2 md:grid-cols-4 gap-4 mt-4 pt-4 border-t border-[var(--glass-border)]">
            <div>
              <p class="text-xs text-[var(--text-muted)]">Peak EPE</p>
              <p class="text-sm font-semibold text-[var(--text-primary)]">{{ formatCurrency(selectedNsResult.peakEpe) }}</p>
            </div>
            <div>
              <p class="text-xs text-[var(--text-muted)]">Peak ENE</p>
              <p class="text-sm font-semibold text-[var(--text-primary)]">{{ formatCurrency(selectedNsResult.peakEne) }}</p>
            </div>
            <div>
              <p class="text-xs text-[var(--text-muted)]">Avg EPE</p>
              <p class="text-sm font-semibold text-[var(--text-primary)]">{{ formatCurrency(selectedNsResult.avgEpe) }}</p>
            </div>
            <div>
              <p class="text-xs text-[var(--text-muted)]">Avg ENE</p>
              <p class="text-sm font-semibold text-[var(--text-primary)]">{{ formatCurrency(selectedNsResult.avgEne) }}</p>
            </div>
          </div>
        </div>

        <!-- XVA Waterfall Chart -->
        <div v-if="result" class="glass-card p-6">
          <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">XVA Waterfall by Counterparty</h3>
          <div class="h-72">
            <canvas ref="waterfallCanvas"></canvas>
          </div>
        </div>

        <!-- Counterparty XVA Table -->
        <div v-if="result" class="glass-card p-6">
          <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Counterparty XVA Results</h3>
          <div class="overflow-x-auto">
            <table class="w-full text-sm">
              <thead>
                <tr class="border-b border-[var(--glass-border)]">
                  <th class="text-left py-3 px-2 text-[var(--text-muted)] font-medium">Counterparty</th>
                  <th class="text-center py-3 px-2 text-[var(--text-muted)] font-medium">Rating</th>
                  <th class="text-right py-3 px-2 text-[var(--text-muted)] font-medium">Hazard Rate</th>
                  <th class="text-right py-3 px-2 text-[var(--text-muted)] font-medium">LGD</th>
                  <th class="text-right py-3 px-2 text-[var(--text-muted)] font-medium">UCVA</th>
                  <th class="text-right py-3 px-2 text-[var(--text-muted)] font-medium">BCVA</th>
                  <th class="text-right py-3 px-2 text-[var(--text-muted)] font-medium">UDVA</th>
                  <th class="text-right py-3 px-2 text-[var(--text-muted)] font-medium">BDVA</th>
                  <th class="text-right py-3 px-2 text-[var(--text-muted)] font-medium">FVA</th>
                  <th class="text-right py-3 px-2 text-[var(--text-primary)] font-semibold">Total XVA</th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="cp in result.counterpartyResults"
                  :key="cp.counterpartyId"
                  class="border-b border-[var(--glass-border)] border-opacity-50 hover:bg-[var(--surface-hover)] transition-colors"
                >
                  <td class="py-3 px-2 text-[var(--text-primary)] font-medium">{{ cp.counterpartyId }}</td>
                  <td class="py-3 px-2 text-center">
                    <span
                      :class="[
                        'px-2 py-0.5 rounded text-xs font-medium',
                        cp.creditRating === 'AA' ? 'bg-green-500/20 text-green-400' :
                        cp.creditRating === 'BBB' ? 'bg-yellow-500/20 text-yellow-400' :
                        'bg-red-500/20 text-red-400'
                      ]"
                    >{{ cp.creditRating }}</span>
                  </td>
                  <td class="py-3 px-2 text-right text-[var(--text-secondary)]">{{ (cp.hazardRate * 100).toFixed(1) }}%</td>
                  <td class="py-3 px-2 text-right text-[var(--text-secondary)]">{{ (cp.lgd * 100).toFixed(0) }}%</td>
                  <td class="py-3 px-2 text-right text-[var(--danger)]">{{ formatCurrency(cp.ucva) }}</td>
                  <td class="py-3 px-2 text-right text-[var(--danger)]">{{ formatCurrency(cp.bcva) }}</td>
                  <td class="py-3 px-2 text-right text-[var(--text-secondary)]">{{ formatCurrency(cp.udva) }}</td>
                  <td class="py-3 px-2 text-right text-[var(--text-secondary)]">{{ formatCurrency(cp.bdva) }}</td>
                  <td class="py-3 px-2 text-right text-[var(--warning)]">{{ formatCurrency(cp.fva) }}</td>
                  <td class="py-3 px-2 text-right font-semibold text-[var(--text-primary)]">{{ formatCurrency(cp.totalXva) }}</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>

        <!-- Hierarchy Tree -->
        <div v-if="result" class="glass-card p-6">
          <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Portfolio Hierarchy</h3>
          <p class="text-xs text-[var(--text-muted)] mb-4">
            {{ result.hierarchy.totalCounterparties }} Counterparties /
            {{ result.hierarchy.totalNettingSets }} Netting Sets /
            {{ result.hierarchy.totalTrades }} Trades
          </p>
          <div class="space-y-3">
            <div v-for="cp in result.hierarchy.counterparties" :key="cp.id" class="border border-[var(--glass-border)] rounded-lg p-3">
              <div class="flex items-center gap-2 mb-2">
                <i class="fas fa-building text-[var(--primary)]"></i>
                <span class="font-medium text-[var(--text-primary)]">{{ cp.id }}</span>
                <span class="text-xs px-2 py-0.5 rounded bg-[var(--surface)] text-[var(--text-muted)]">{{ cp.creditRating }}</span>
                <span v-if="cp.noDocTradeCount > 0" class="text-xs text-[var(--warning)]">+{{ cp.noDocTradeCount }} no-doc</span>
              </div>
              <div v-for="isda in cp.isdaAgreements" :key="isda.nettingSetId" class="ml-6 border-l-2 border-[var(--glass-border)] pl-4 mb-2">
                <div class="flex items-center gap-2 mb-1">
                  <i class="fas fa-file-contract text-[var(--text-muted)] text-xs"></i>
                  <span class="text-sm text-[var(--text-secondary)]">ISDA: {{ isda.nettingSetId }}</span>
                  <span v-if="isda.nonCsaTradeCount > 0" class="text-xs text-[var(--text-muted)]">(+{{ isda.nonCsaTradeCount }} non-CSA)</span>
                </div>
                <div v-for="csa in isda.vmCsas" :key="csa.csaId" class="ml-4 border-l-2 border-[var(--glass-border)] pl-4 py-1">
                  <div class="flex items-center gap-2">
                    <i class="fas fa-shield-alt text-green-400 text-xs"></i>
                    <span class="text-xs text-[var(--text-secondary)]">CSA: {{ csa.csaId }}</span>
                    <span class="text-xs text-[var(--text-muted)]">{{ csa.tradeCount }} trades</span>
                    <span class="text-xs text-[var(--text-muted)]">| MPOR: {{ csa.mporDays }}d</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Computation Info -->
        <div v-if="result" class="text-center text-xs text-[var(--text-muted)]">
          Computed in {{ result.computationTimeMs.toFixed(0) }}ms |
          {{ result.config.nPaths.toLocaleString() }} paths |
          {{ result.config.timePoints }} time points |
          {{ result.config.horizonYears }}Y horizon
          <span v-if="result.config.antithetic"> | Antithetic</span>
        </div>

        <!-- Empty State -->
        <div v-if="!result && !loading" class="glass-card p-12 text-center">
          <i class="fas fa-chart-area text-4xl text-[var(--text-muted)] mb-4"></i>
          <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-2">No Simulation Results</h3>
          <p class="text-sm text-[var(--text-muted)] mb-4">Configure parameters and click "Run Simulation" to compute XVA.</p>
          <p v-if="config" class="text-xs text-[var(--text-muted)]">
            Demo portfolio: {{ config.counterparties.length }} counterparties,
            {{ config.counterparties.reduce((s, c) => s + c.nettingSets.length, 0) }} netting sets
          </p>
        </div>
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

.config-input {
  width: 100%;
  padding: 0.4rem 0.6rem;
  border: 1px solid var(--glass-border);
  border-radius: 6px;
  background: var(--surface);
  color: var(--text-primary);
  font-size: 0.8rem;
}
.config-input:focus {
  outline: none;
  border-color: var(--primary);
}
</style>
