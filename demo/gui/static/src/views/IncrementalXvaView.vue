<script setup lang="ts">
import { ref, computed, nextTick } from 'vue'
import { Chart, registerables, type ChartConfiguration, type TooltipItem } from 'chart.js'
import { getChartColors } from '@/composables/useChartTheme'
import {
  fetchIncrementalXvaConfig,
  runIncrementalXva,
} from '../services/api'
import type {
  IncrementalXvaRequest,
  IncrementalXvaResponse,
} from '../types/api'

Chart.register(...registerables)

// ── State ───────────────────────────────────────────────────────────────────

const loading = ref(false)
const result = ref<IncrementalXvaResponse | null>(null)
const exposureChartRef = ref<HTMLCanvasElement | null>(null)
const waterfallChartRef = ref<HTMLCanvasElement | null>(null)
const fundingSpreadBps = ref(50)
const configExpanded = ref(true)

let exposureChart: Chart | null = null
let waterfallChart: Chart | null = null

const config = ref<any>({
  nPaths: 10000,
  horizonYears: 10,
  timeStep: 'quarterly',
  seed: 42,
  antithetic: true,
  bilateral: true,
  computeFva: true,
  hwMeanReversion: 0.05,
  hwVolatility: 0.01,
  hwInitialRate: 0.03,
  couplingMethod: 'swap_rate',
  couplingSwapTenor: 10,
  hazardRate: 0.02,
  lgd: 0.6,
  baseSwaps: [] as any[],
  baseExotics: [] as any[],
  incrementalTrade: { type: 'swap', tradeId: 'NEW_IRS', notional: 1000000, fixedRate: 0.03, tenorYears: 5, paymentFrequency: 'semi-annual', isPayer: true },
})

// ── Computed ────────────────────────────────────────────────────────────────

const summaryStats = computed(() => {
  if (!result.value) return []
  const r = result.value
  return [
    { label: 'Base XVA', value: formatCurrency(r.baseXva.total), subtitle: `BCVA: ${formatCurrency(r.baseXva.bcva)}`, icon: 'fa-layer-group', color: '#3b82f6' },
    { label: 'Full XVA', value: formatCurrency(r.fullXva.total), subtitle: `BCVA: ${formatCurrency(r.fullXva.bcva)}`, icon: 'fa-chart-bar', color: '#8b5cf6' },
    { label: 'Incremental XVA', value: formatCurrency(r.incrementalXva.total), subtitle: `FVA: ${formatCurrency(r.incrementalXva.fva)}`, icon: 'fa-balance-scale', color: r.incrementalXva.total >= 0 ? '#10b981' : '#ef4444' },
  ]
})

const metricsRows = computed(() => {
  if (!result.value) return []
  const r = result.value
  return [
    { label: 'UCVA', base: r.baseXva.ucva, full: r.fullXva.ucva, incremental: r.incrementalXva.ucva },
    { label: 'BCVA', base: r.baseXva.bcva, full: r.fullXva.bcva, incremental: r.incrementalXva.bcva },
    { label: 'UDVA', base: r.baseXva.udva, full: r.fullXva.udva, incremental: r.incrementalXva.udva },
    { label: 'BDVA', base: r.baseXva.bdva, full: r.fullXva.bdva, incremental: r.incrementalXva.bdva },
    { label: 'FCA', base: r.baseXva.fca, full: r.fullXva.fca, incremental: r.incrementalXva.fca },
    { label: 'FBA', base: r.baseXva.fba, full: r.fullXva.fba, incremental: r.incrementalXva.fba },
    { label: 'FVA', base: r.baseXva.fva, full: r.fullXva.fva, incremental: r.incrementalXva.fva },
    { label: 'Total XVA', base: r.baseXva.total, full: r.fullXva.total, incremental: r.incrementalXva.total },
  ]
})

// ── Methods ─────────────────────────────────────────────────────────────────

function formatCurrency(val: number): string {
  const abs = Math.abs(val)
  if (abs >= 1e6) return `${val >= 0 ? '' : '-'}$${(abs / 1e6).toFixed(2)}M`
  if (abs >= 1e3) return `${val >= 0 ? '' : '-'}$${(abs / 1e3).toFixed(1)}K`
  return `$${val.toFixed(2)}`
}

function formatNotional(val: number): string {
  if (val >= 1e6) return (val / 1e6).toFixed(0) + 'M'
  if (val >= 1e3) return (val / 1e3).toFixed(0) + 'K'
  return val.toString()
}

async function loadDemo() {
  try {
    const demoConfig = await fetchIncrementalXvaConfig()
    config.value.nPaths = demoConfig.nPaths
    config.value.horizonYears = demoConfig.horizonYears
    config.value.timeStep = demoConfig.timeStep
    config.value.antithetic = demoConfig.antithetic
    config.value.bilateral = demoConfig.bilateral
    config.value.computeFva = demoConfig.computeFva
    config.value.hwMeanReversion = demoConfig.hwMeanReversion
    config.value.hwVolatility = demoConfig.hwVolatility
    config.value.hwInitialRate = demoConfig.hwInitialRate
    config.value.couplingMethod = demoConfig.couplingMethod
    config.value.hazardRate = demoConfig.hazardRate
    config.value.lgd = demoConfig.lgd
    config.value.baseSwaps = demoConfig.baseSwaps
    config.value.baseExotics = demoConfig.baseExotics
    config.value.incrementalTrade = demoConfig.incrementalTrade
  } catch (e: any) {
    console.error('Failed to load demo config:', e)
  }
}

async function runSimulation() {
  loading.value = true
  result.value = null

  try {
    const request: IncrementalXvaRequest = {
      nPaths: config.value.nPaths,
      horizonYears: config.value.horizonYears,
      timeStep: config.value.timeStep,
      seed: config.value.seed,
      antithetic: config.value.antithetic,
      bilateral: config.value.bilateral,
      computeFva: config.value.computeFva,
      hwMeanReversion: config.value.hwMeanReversion,
      hwVolatility: config.value.hwVolatility,
      hwInitialRate: config.value.hwInitialRate,
      couplingMethod: config.value.couplingMethod,
      couplingSwapTenor: config.value.couplingSwapTenor,
      hazardRate: config.value.hazardRate,
      lgd: config.value.lgd,
      fundingSpread: fundingSpreadBps.value / 10000,
      baseSwaps: config.value.baseSwaps,
      baseExotics: config.value.baseExotics,
      incrementalTrade: config.value.incrementalTrade,
    }

    result.value = await runIncrementalXva(request)

    await nextTick()
    renderExposureChart()
    renderWaterfallChart()
  } catch (e: any) {
    console.error('Simulation failed:', e)
  } finally {
    loading.value = false
  }
}

function renderExposureChart() {
  if (!result.value || !exposureChartRef.value) return
  const r = result.value

  if (exposureChart) exposureChart.destroy()

  const labels = r.timeGrid.map((t: number) => t.toFixed(2) + 'Y')
  const cc = getChartColors()

  const chartConfig: ChartConfiguration<'line'> = {
    type: 'line',
    data: {
      labels,
      datasets: [
        { label: 'Base EPE', data: r.baseEpe, borderColor: '#3b82f6', backgroundColor: '#3b82f61a', borderDash: [5, 5], fill: false, tension: 0.4 },
        { label: 'Full EPE', data: r.fullEpe, borderColor: '#3b82f6', backgroundColor: '#3b82f61a', fill: true, tension: 0.4 },
        { label: 'Base ENE', data: r.baseEne.map((v: number) => -v), borderColor: '#ef4444', backgroundColor: '#ef44441a', borderDash: [5, 5], fill: false, tension: 0.4 },
        { label: 'Full ENE', data: r.fullEne.map((v: number) => -v), borderColor: '#ef4444', backgroundColor: '#ef44441a', fill: true, tension: 0.4 },
      ],
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      interaction: { intersect: false, mode: 'index' },
      plugins: {
        legend: { display: true, labels: { color: cc.tick } },
        tooltip: {
          backgroundColor: cc.tooltipBg,
          titleColor: cc.tooltipTitle,
          bodyColor: cc.tooltipBody,
          padding: 12,
          cornerRadius: 8,
          callbacks: {
            label: (ctx: TooltipItem<'line'>) => `${ctx.dataset.label}: ${formatCurrency(ctx.parsed.y)}`,
          },
        },
      },
      scales: {
        x: { grid: { color: cc.grid }, ticks: { color: cc.tick } },
        y: { grid: { color: cc.grid }, ticks: { color: cc.tick, callback: (v) => formatCurrency(v as number) } },
      },
    },
  }

  exposureChart = new Chart(exposureChartRef.value, chartConfig)
}

function renderWaterfallChart() {
  if (!result.value || !waterfallChartRef.value) return
  const r = result.value

  if (waterfallChart) waterfallChart.destroy()

  const cc = getChartColors()

  const chartConfig: ChartConfiguration<'bar'> = {
    type: 'bar',
    data: {
      labels: ['BCVA', 'BDVA', 'FVA', 'Total XVA'],
      datasets: [
        { label: 'Base', data: [r.baseXva.bcva, -r.baseXva.bdva, r.baseXva.fva, r.baseXva.total], backgroundColor: '#3b82f6cc' },
        { label: 'Full', data: [r.fullXva.bcva, -r.fullXva.bdva, r.fullXva.fva, r.fullXva.total], backgroundColor: '#10b981cc' },
        { label: 'Incremental', data: [r.incrementalXva.bcva, -r.incrementalXva.bdva, r.incrementalXva.fva, r.incrementalXva.total], backgroundColor: '#f59e0bcc' },
      ],
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      interaction: { intersect: false, mode: 'index' },
      plugins: {
        legend: { display: true, labels: { color: cc.tick } },
        tooltip: {
          backgroundColor: cc.tooltipBg,
          titleColor: cc.tooltipTitle,
          bodyColor: cc.tooltipBody,
          padding: 12,
          cornerRadius: 8,
          callbacks: {
            label: (ctx: TooltipItem<'bar'>) => `${ctx.dataset.label}: ${formatCurrency(ctx.parsed.y)}`,
          },
        },
      },
      scales: {
        x: { grid: { color: cc.grid }, ticks: { color: cc.tick } },
        y: { grid: { color: cc.grid }, ticks: { color: cc.tick, callback: (v) => formatCurrency(v as number) } },
      },
    },
  }

  waterfallChart = new Chart(waterfallChartRef.value, chartConfig)
}

// Auto-load demo on mount
loadDemo()
</script>

<template>
  <div class="incremental-xva-view">
    <!-- Header -->
    <div class="flex items-center justify-between mb-6">
      <div>
        <h2 class="text-2xl font-semibold text-[var(--text-primary)]">Portfolio XVA Engine</h2>
      </div>
      <div class="flex items-center gap-3">
        <!-- Summary Stats in Header -->
        <template v-if="result">
          <div
            v-for="stat in summaryStats"
            :key="stat.label"
            class="glass-card px-4 py-2 flex items-center gap-3"
          >
            <div
              class="w-8 h-8 rounded-lg flex items-center justify-center flex-shrink-0"
              :style="{ backgroundColor: `${stat.color}1a` }"
            >
              <i :class="['fas text-xs', stat.icon]" :style="{ color: stat.color }"></i>
            </div>
            <div class="leading-tight">
              <p class="text-xs text-[var(--text-muted)]">{{ stat.label }}</p>
              <p class="text-lg font-semibold text-[var(--text-primary)]">{{ stat.value }}</p>
            </div>
          </div>
        </template>

        <button
          class="px-4 py-2 rounded-lg text-sm font-medium bg-[var(--surface)] text-[var(--text-secondary)] hover:bg-[var(--surface-hover)] transition-all"
          :disabled="loading"
          @click="loadDemo"
        >
          <i class="fas fa-download mr-2"></i>Load Demo
        </button>
        <button
          :class="[
            'px-4 py-2 rounded-lg text-sm font-medium transition-all',
            loading ? 'bg-gray-500 cursor-not-allowed' : 'bg-[var(--primary)] hover:opacity-90'
          ]"
          class="text-white"
          :disabled="loading"
          @click="runSimulation"
        >
          <i :class="['fas mr-2', loading ? 'fa-spinner fa-spin' : 'fa-play']"></i>
          {{ loading ? 'Computing...' : 'Run Simulation' }}
        </button>
      </div>
    </div>

    <!-- Configuration + Portfolio (side by side) -->
    <div class="glass-card p-6 mb-6">
      <button
        class="w-full flex items-center justify-between"
        @click="configExpanded = !configExpanded"
      >
        <h3 class="text-lg font-semibold text-[var(--text-primary)]">Simulation Configuration</h3>
        <i :class="['fas fa-chevron-down transition-transform duration-200', { 'rotate-180': !configExpanded }]"></i>
      </button>

      <Transition name="accordion">
        <div v-show="configExpanded" class="mt-4 grid grid-cols-1 lg:grid-cols-5 gap-6">
          <!-- Left: Parameters (3 cols) -->
          <div class="lg:col-span-3 space-y-4">
            <!-- HW1F Model Parameters -->
            <div>
              <h4 class="text-xs font-semibold uppercase tracking-wider text-[var(--text-muted)] mb-3">HW1F Model</h4>
              <div class="grid grid-cols-3 gap-3">
                <div>
                  <label class="text-xs text-[var(--text-muted)] mb-1 block">Mean Reversion (a)</label>
                  <input type="number" v-model.number="config.hwMeanReversion" step="0.01" min="0.001"
                    class="w-full px-3 py-2 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm">
                </div>
                <div>
                  <label class="text-xs text-[var(--text-muted)] mb-1 block">Volatility (σ)</label>
                  <input type="number" v-model.number="config.hwVolatility" step="0.001" min="0.001"
                    class="w-full px-3 py-2 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm">
                </div>
                <div>
                  <label class="text-xs text-[var(--text-muted)] mb-1 block">Initial Rate r(0)</label>
                  <input type="number" v-model.number="config.hwInitialRate" step="0.005"
                    class="w-full px-3 py-2 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm">
                </div>
              </div>
            </div>

            <!-- Simulation Settings -->
            <div>
              <h4 class="text-xs font-semibold uppercase tracking-wider text-[var(--text-muted)] mb-3">Simulation</h4>
              <div class="grid grid-cols-4 gap-3">
                <div>
                  <label class="text-xs text-[var(--text-muted)] mb-1 block">Paths</label>
                  <select v-model.number="config.nPaths"
                    class="w-full px-3 py-2 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm">
                    <option :value="1000">1,000</option>
                    <option :value="5000">5,000</option>
                    <option :value="10000">10,000</option>
                    <option :value="50000">50,000</option>
                    <option :value="100000">100,000</option>
                  </select>
                </div>
                <div>
                  <label class="text-xs text-[var(--text-muted)] mb-1 block">Horizon (yrs)</label>
                  <input type="number" v-model.number="config.horizonYears" step="1" min="1" max="30"
                    class="w-full px-3 py-2 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm">
                </div>
                <div>
                  <label class="text-xs text-[var(--text-muted)] mb-1 block">Time Step</label>
                  <select v-model="config.timeStep"
                    class="w-full px-3 py-2 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm">
                    <option value="monthly">Monthly</option>
                    <option value="quarterly">Quarterly</option>
                    <option value="semi-annual">Semi-Annual</option>
                  </select>
                </div>
                <div>
                  <label class="text-xs text-[var(--text-muted)] mb-1 block">Seed</label>
                  <input type="number" v-model.number="config.seed"
                    class="w-full px-3 py-2 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm">
                </div>
              </div>
              <div class="flex gap-6 mt-3">
                <div class="flex items-center gap-2">
                  <input v-model="config.antithetic" type="checkbox" id="antithetic" class="rounded">
                  <label for="antithetic" class="text-sm text-[var(--text-secondary)]">Antithetic</label>
                </div>
                <div class="flex items-center gap-2">
                  <input v-model="config.bilateral" type="checkbox" id="bilateral" class="rounded">
                  <label for="bilateral" class="text-sm text-[var(--text-secondary)]">Bilateral</label>
                </div>
                <div class="flex items-center gap-2">
                  <input v-model="config.computeFva" type="checkbox" id="computeFva" class="rounded">
                  <label for="computeFva" class="text-sm text-[var(--text-secondary)]">FVA</label>
                </div>
              </div>
            </div>

            <!-- Coupling & Credit -->
            <div class="grid grid-cols-2 gap-4">
              <div>
                <h4 class="text-xs font-semibold uppercase tracking-wider text-[var(--text-muted)] mb-3">Model Coupling</h4>
                <div class="space-y-2">
                  <label class="flex items-center gap-2 text-sm text-[var(--text-secondary)] cursor-pointer">
                    <input type="radio" v-model="config.couplingMethod" value="swap_rate" class="text-[var(--primary)]">
                    Swap Rate Mapping
                  </label>
                  <label class="flex items-center gap-2 text-sm text-[var(--text-secondary)] cursor-pointer">
                    <input type="radio" v-model="config.couplingMethod" value="zscore" class="text-[var(--primary)]">
                    Z-Score Matching
                  </label>
                </div>
                <div v-if="config.couplingMethod === 'swap_rate'" class="mt-3">
                  <label class="text-xs text-[var(--text-muted)] mb-1 block">Benchmark Swap Tenor</label>
                  <input type="number" v-model.number="config.couplingSwapTenor" step="1"
                    class="w-full px-3 py-2 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm">
                </div>
              </div>
              <div>
                <h4 class="text-xs font-semibold uppercase tracking-wider text-[var(--text-muted)] mb-3">Counterparty Credit</h4>
                <div class="grid grid-cols-3 gap-3">
                  <div>
                    <label class="text-xs text-[var(--text-muted)] mb-1 block">Hazard Rate</label>
                    <input type="number" v-model.number="config.hazardRate" step="0.005" min="0"
                      class="w-full px-3 py-2 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm">
                  </div>
                  <div>
                    <label class="text-xs text-[var(--text-muted)] mb-1 block">LGD</label>
                    <input type="number" v-model.number="config.lgd" step="0.1" min="0" max="1"
                      class="w-full px-3 py-2 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm">
                  </div>
                  <div>
                    <label class="text-xs text-[var(--text-muted)] mb-1 block">Spread (bps)</label>
                    <input type="number" v-model.number="fundingSpreadBps" step="5" min="0"
                      class="w-full px-3 py-2 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm">
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- Right: Portfolio (2 cols) -->
          <div class="lg:col-span-2 space-y-4">
            <!-- Base Portfolio -->
            <div class="rounded-lg border border-[var(--glass-border)] p-4">
              <h4 class="text-xs font-semibold uppercase tracking-wider text-[var(--text-muted)] mb-3">Base Portfolio</h4>
              <div class="space-y-1.5">
                <div v-for="swap in config.baseSwaps" :key="swap.tradeId"
                  class="flex items-center gap-2 px-3 py-2 rounded-lg bg-[var(--surface)] text-sm">
                  <span class="px-2 py-0.5 rounded text-xs font-medium bg-blue-500/20 text-blue-400">
                    {{ swap.isPayer ? 'Pay' : 'Rcv' }}
                  </span>
                  <span class="text-[var(--text-primary)] font-medium">{{ swap.tradeId }}</span>
                  <span class="text-[var(--text-secondary)]">{{ formatNotional(swap.notional) }}</span>
                  <span class="text-[var(--text-secondary)]">{{ (swap.fixedRate * 100).toFixed(2) }}%</span>
                  <span class="text-[var(--text-muted)]">{{ swap.tenorYears }}Y</span>
                </div>
                <div v-for="exotic in config.baseExotics" :key="exotic.tradeId"
                  class="flex items-center gap-2 px-3 py-2 rounded-lg bg-[var(--surface)] text-sm">
                  <span class="px-2 py-0.5 rounded text-xs font-medium bg-pink-500/20 text-pink-400">
                    {{ exotic.productType.toUpperCase() }}
                  </span>
                  <span class="text-[var(--text-primary)] font-medium">{{ exotic.tradeId }}</span>
                  <span class="text-[var(--text-secondary)]">{{ formatNotional(exotic.notional) }}</span>
                </div>
              </div>
            </div>

            <!-- Incremental Trade -->
            <div class="rounded-lg border border-amber-500/30 p-4">
              <h4 class="text-xs font-semibold uppercase tracking-wider text-amber-400 mb-3">Incremental Trade</h4>
              <div class="space-y-1.5">
                <template v-if="config.incrementalTrade.type === 'swap'">
                  <div class="flex items-center gap-2 px-3 py-2 rounded-lg bg-amber-500/10 border border-amber-500/20 text-sm">
                    <span class="px-2 py-0.5 rounded text-xs font-medium bg-blue-500/20 text-blue-400">
                      {{ config.incrementalTrade.isPayer ? 'Pay' : 'Rcv' }}
                    </span>
                    <span class="text-[var(--text-primary)] font-medium">{{ config.incrementalTrade.tradeId }}</span>
                    <span class="text-[var(--text-secondary)]">{{ formatNotional(config.incrementalTrade.notional) }}</span>
                    <span class="text-[var(--text-secondary)]">{{ ((config.incrementalTrade.fixedRate || 0) * 100).toFixed(2) }}%</span>
                  </div>
                </template>
                <template v-else>
                  <div class="flex items-center gap-2 px-3 py-2 rounded-lg bg-amber-500/10 border border-amber-500/20 text-sm">
                    <span class="px-2 py-0.5 rounded text-xs font-medium bg-pink-500/20 text-pink-400">
                      {{ (config.incrementalTrade.productType || 'EXOTIC').toUpperCase() }}
                    </span>
                    <span class="text-[var(--text-primary)] font-medium">{{ config.incrementalTrade.tradeId }}</span>
                    <span class="text-[var(--text-secondary)]">{{ formatNotional(config.incrementalTrade.notional) }}</span>
                  </div>
                </template>
              </div>
            </div>
          </div>
        </div>
      </Transition>
    </div>

    <!-- Exposure Chart -->
    <div v-if="result" class="glass-card p-6 mb-6">
      <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Exposure Profiles</h3>
      <div class="h-80">
        <canvas ref="exposureChartRef"></canvas>
      </div>
    </div>

    <!-- XVA Waterfall Chart -->
    <div v-if="result" class="glass-card p-6 mb-6">
      <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">XVA Waterfall</h3>
      <div class="h-72">
        <canvas ref="waterfallChartRef"></canvas>
      </div>
    </div>

    <!-- Breakdown Table -->
    <div v-if="result" class="glass-card p-6 mb-6">
      <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">XVA Breakdown</h3>
      <div class="overflow-x-auto">
        <table class="w-full text-sm">
          <thead>
            <tr class="border-b border-[var(--glass-border)]">
              <th class="text-left py-3 px-2 text-[var(--text-muted)] font-medium">Metric</th>
              <th class="text-right py-3 px-2 text-[var(--text-muted)] font-medium">Base Portfolio</th>
              <th class="text-right py-3 px-2 text-[var(--text-muted)] font-medium">Full Portfolio</th>
              <th class="text-right py-3 px-2 text-[var(--text-primary)] font-semibold">Incremental</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="metric in metricsRows"
              :key="metric.label"
              class="border-b border-[var(--glass-border)] border-opacity-50 hover:bg-[var(--surface-hover)] transition-colors"
            >
              <td class="py-3 px-2 text-[var(--text-primary)] font-medium">{{ metric.label }}</td>
              <td class="py-3 px-2 text-right text-[var(--text-secondary)]">{{ formatCurrency(metric.base) }}</td>
              <td class="py-3 px-2 text-right text-[var(--text-secondary)]">{{ formatCurrency(metric.full) }}</td>
              <td class="py-3 px-2 text-right font-semibold" :class="metric.incremental >= 0 ? 'text-green-400' : 'text-red-400'">
                {{ formatCurrency(metric.incremental) }}
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- Computation Info -->
    <div v-if="result" class="text-center text-xs text-[var(--text-muted)] pb-6">
      Computed in {{ result.computationTimeMs.toFixed(1) }}ms |
      {{ result.nPaths.toLocaleString() }} paths |
      {{ result.timeGrid.length }} time steps |
      Coupling: {{ result.couplingMethod === 'swap_rate' ? 'Swap Rate (A)' : 'Z-Score (B)' }}
    </div>

    <!-- Empty State -->
    <div v-if="!result && !loading" class="glass-card p-12 text-center">
      <i class="fas fa-balance-scale text-4xl text-[var(--text-muted)] mb-4"></i>
      <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-2">No Simulation Results</h3>
      <p class="text-sm text-[var(--text-muted)] mb-4">Configure parameters above and click "Run Simulation" to compute incremental XVA.</p>
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

.accordion-enter-active,
.accordion-leave-active {
  transition: all 0.2s ease;
  overflow: hidden;
}

.accordion-enter-from,
.accordion-leave-to {
  opacity: 0;
  max-height: 0;
}

.accordion-enter-to,
.accordion-leave-from {
  opacity: 1;
  max-height: 800px;
}
</style>
