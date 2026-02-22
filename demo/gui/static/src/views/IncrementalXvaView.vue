<template>
  <div class="incremental-xva-view">
    <div class="page-header">
      <h1>Incremental XVA Engine</h1>
      <p class="subtitle">
        HW1F Outer MC + Analytical Swap Pricing + MFM Grid Cache Lookup
      </p>
    </div>

    <!-- Configuration Section -->
    <div class="config-grid">
      <!-- HW1F Model Parameters -->
      <div class="config-card">
        <h3>HW1F Model</h3>
        <div class="form-group">
          <label>Mean Reversion (a)</label>
          <input type="number" v-model.number="config.hwMeanReversion" step="0.01" min="0.001" />
        </div>
        <div class="form-group">
          <label>Volatility (σ)</label>
          <input type="number" v-model.number="config.hwVolatility" step="0.001" min="0.001" />
        </div>
        <div class="form-group">
          <label>Initial Rate r(0)</label>
          <input type="number" v-model.number="config.hwInitialRate" step="0.005" />
        </div>
      </div>

      <!-- Simulation Settings -->
      <div class="config-card">
        <h3>Simulation</h3>
        <div class="form-group">
          <label>Paths</label>
          <select v-model.number="config.nPaths">
            <option :value="1000">1,000</option>
            <option :value="5000">5,000</option>
            <option :value="10000">10,000</option>
            <option :value="50000">50,000</option>
            <option :value="100000">100,000</option>
          </select>
        </div>
        <div class="form-group">
          <label>Horizon (years)</label>
          <input type="number" v-model.number="config.horizonYears" step="1" min="1" max="30" />
        </div>
        <div class="form-group">
          <label>Time Step</label>
          <select v-model="config.timeStep">
            <option value="monthly">Monthly</option>
            <option value="quarterly">Quarterly</option>
            <option value="semi-annual">Semi-Annual</option>
          </select>
        </div>
        <div class="form-group">
          <label>Seed</label>
          <input type="number" v-model.number="config.seed" />
        </div>
        <div class="toggle-group">
          <label><input type="checkbox" v-model="config.antithetic" /> Antithetic</label>
          <label><input type="checkbox" v-model="config.bilateral" /> Bilateral</label>
          <label><input type="checkbox" v-model="config.computeFva" /> FVA</label>
        </div>
      </div>

      <!-- Coupling Method -->
      <div class="config-card">
        <h3>Model Coupling</h3>
        <div class="form-group">
          <label>
            <input type="radio" v-model="config.couplingMethod" value="swap_rate" />
            Approach A: Swap Rate Mapping
          </label>
          <label>
            <input type="radio" v-model="config.couplingMethod" value="zscore" />
            Approach B: Z-Score Matching
          </label>
        </div>
        <div v-if="config.couplingMethod === 'swap_rate'" class="form-group">
          <label>Benchmark Swap Tenor</label>
          <input type="number" v-model.number="config.couplingSwapTenor" step="1" />
        </div>
      </div>

      <!-- Credit Parameters -->
      <div class="config-card">
        <h3>Counterparty Credit</h3>
        <div class="form-group">
          <label>Hazard Rate</label>
          <input type="number" v-model.number="config.hazardRate" step="0.005" min="0" />
        </div>
        <div class="form-group">
          <label>LGD</label>
          <input type="number" v-model.number="config.lgd" step="0.1" min="0" max="1" />
        </div>
        <div class="form-group">
          <label>Funding Spread (bps)</label>
          <input type="number" v-model.number="fundingSpreadBps" step="5" min="0" />
        </div>
      </div>
    </div>

    <!-- Portfolio Section -->
    <div class="portfolio-section">
      <div class="portfolio-card">
        <h3>Base Portfolio</h3>
        <div class="trade-list">
          <div v-for="swap in config.baseSwaps" :key="swap.tradeId" class="trade-item">
            <span class="trade-badge swap">{{ swap.isPayer ? 'Pay' : 'Rcv' }}</span>
            <span>{{ swap.tradeId }}</span>
            <span>{{ formatNotional(swap.notional) }}</span>
            <span>{{ (swap.fixedRate * 100).toFixed(2) }}%</span>
            <span>{{ swap.tenorYears }}Y</span>
          </div>
          <div v-for="exotic in config.baseExotics" :key="exotic.tradeId" class="trade-item">
            <span class="trade-badge exotic">{{ exotic.productType.toUpperCase() }}</span>
            <span>{{ exotic.tradeId }}</span>
            <span>{{ formatNotional(exotic.notional) }}</span>
          </div>
        </div>
      </div>

      <div class="portfolio-card incremental">
        <h3>Incremental Trade</h3>
        <div class="trade-list">
          <template v-if="config.incrementalTrade.type === 'swap'">
            <div class="trade-item highlight">
              <span class="trade-badge swap">{{ config.incrementalTrade.isPayer ? 'Pay' : 'Rcv' }}</span>
              <span>{{ config.incrementalTrade.tradeId }}</span>
              <span>{{ formatNotional(config.incrementalTrade.notional) }}</span>
              <span>{{ ((config.incrementalTrade.fixedRate || 0) * 100).toFixed(2) }}%</span>
            </div>
          </template>
          <template v-else>
            <div class="trade-item highlight">
              <span class="trade-badge exotic">{{ (config.incrementalTrade.productType || 'EXOTIC').toUpperCase() }}</span>
              <span>{{ config.incrementalTrade.tradeId }}</span>
              <span>{{ formatNotional(config.incrementalTrade.notional) }}</span>
            </div>
          </template>
        </div>
      </div>
    </div>

    <!-- Action Buttons -->
    <div class="actions">
      <button class="btn btn-secondary" @click="loadDemo" :disabled="loading">
        Load Demo
      </button>
      <button class="btn btn-primary" @click="runSimulation" :disabled="loading">
        <span v-if="loading" class="spinner"></span>
        {{ loading ? 'Computing...' : 'Run Incremental XVA' }}
      </button>
    </div>

    <!-- Results Section -->
    <div v-if="result" class="results-section">
      <!-- Summary Cards -->
      <div class="summary-cards">
        <div class="summary-card">
          <h4>Base XVA</h4>
          <div class="value">{{ formatCurrency(result.baseXva.total) }}</div>
          <div class="detail">BCVA: {{ formatCurrency(result.baseXva.bcva) }}</div>
          <div class="detail">BDVA: {{ formatCurrency(result.baseXva.bdva) }}</div>
          <div class="detail">FVA: {{ formatCurrency(result.baseXva.fva) }}</div>
        </div>
        <div class="summary-card">
          <h4>Full XVA</h4>
          <div class="value">{{ formatCurrency(result.fullXva.total) }}</div>
          <div class="detail">BCVA: {{ formatCurrency(result.fullXva.bcva) }}</div>
          <div class="detail">BDVA: {{ formatCurrency(result.fullXva.bdva) }}</div>
          <div class="detail">FVA: {{ formatCurrency(result.fullXva.fva) }}</div>
        </div>
        <div class="summary-card highlight">
          <h4>Incremental XVA</h4>
          <div class="value large" :class="result.incrementalXva.total >= 0 ? 'positive' : 'negative'">
            {{ formatCurrency(result.incrementalXva.total) }}
          </div>
          <div class="detail">BCVA: {{ formatCurrency(result.incrementalXva.bcva) }}</div>
          <div class="detail">BDVA: {{ formatCurrency(result.incrementalXva.bdva) }}</div>
          <div class="detail">FVA: {{ formatCurrency(result.incrementalXva.fva) }}</div>
        </div>
      </div>

      <!-- Computation Info -->
      <div class="computation-info">
        <span>{{ result.nPaths.toLocaleString() }} paths</span>
        <span>{{ result.timeGrid.length }} time steps</span>
        <span>Coupling: {{ result.couplingMethod === 'swap_rate' ? 'Swap Rate (A)' : 'Z-Score (B)' }}</span>
        <span>{{ result.computationTimeMs.toFixed(1) }} ms</span>
      </div>

      <!-- Exposure Chart -->
      <div class="chart-section">
        <h3>Exposure Profiles</h3>
        <canvas ref="exposureChartRef"></canvas>
      </div>

      <!-- XVA Waterfall -->
      <div class="chart-section">
        <h3>XVA Waterfall</h3>
        <canvas ref="waterfallChartRef"></canvas>
      </div>

      <!-- Breakdown Table -->
      <div class="table-section">
        <h3>XVA Breakdown</h3>
        <table>
          <thead>
            <tr>
              <th>Metric</th>
              <th>Base Portfolio</th>
              <th>Full Portfolio</th>
              <th>Incremental</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="metric in metricsRows" :key="metric.label">
              <td>{{ metric.label }}</td>
              <td>{{ formatCurrency(metric.base) }}</td>
              <td>{{ formatCurrency(metric.full) }}</td>
              <td :class="metric.incremental >= 0 ? 'positive' : 'negative'">
                {{ formatCurrency(metric.incremental) }}
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, nextTick, watch } from 'vue'
import Chart from 'chart.js/auto'
import {
  fetchIncrementalXvaConfig,
  runIncrementalXva,
} from '../services/api'
import type {
  IncrementalXvaRequest,
  IncrementalXvaResponse,
} from '../types/api'

// ── State ───────────────────────────────────────────────────────────────────

const loading = ref(false)
const result = ref<IncrementalXvaResponse | null>(null)
const exposureChartRef = ref<HTMLCanvasElement | null>(null)
const waterfallChartRef = ref<HTMLCanvasElement | null>(null)
const fundingSpreadBps = ref(50)

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
  if (Math.abs(val) >= 1e6) return (val / 1e6).toFixed(2) + 'M'
  if (Math.abs(val) >= 1e3) return (val / 1e3).toFixed(1) + 'K'
  return val.toFixed(2)
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

  exposureChart = new Chart(exposureChartRef.value, {
    type: 'line',
    data: {
      labels,
      datasets: [
        {
          label: 'Base EPE',
          data: r.baseEpe,
          borderColor: '#3b82f6',
          backgroundColor: 'rgba(59,130,246,0.1)',
          borderDash: [5, 5],
          fill: false,
          tension: 0.3,
        },
        {
          label: 'Full EPE',
          data: r.fullEpe,
          borderColor: '#3b82f6',
          backgroundColor: 'rgba(59,130,246,0.1)',
          fill: false,
          tension: 0.3,
        },
        {
          label: 'Base ENE',
          data: r.baseEne.map((v: number) => -v),
          borderColor: '#ef4444',
          borderDash: [5, 5],
          fill: false,
          tension: 0.3,
        },
        {
          label: 'Full ENE',
          data: r.fullEne.map((v: number) => -v),
          borderColor: '#ef4444',
          fill: false,
          tension: 0.3,
        },
      ],
    },
    options: {
      responsive: true,
      plugins: {
        legend: { position: 'top' },
        tooltip: {
          callbacks: {
            label: (ctx: any) => `${ctx.dataset.label}: ${formatCurrency(ctx.raw)}`,
          },
        },
      },
      scales: {
        y: {
          title: { display: true, text: 'Exposure' },
        },
        x: {
          title: { display: true, text: 'Time' },
        },
      },
    },
  })
}

function renderWaterfallChart() {
  if (!result.value || !waterfallChartRef.value) return
  const r = result.value

  if (waterfallChart) waterfallChart.destroy()

  waterfallChart = new Chart(waterfallChartRef.value, {
    type: 'bar',
    data: {
      labels: ['BCVA', 'BDVA', 'FVA', 'Total XVA'],
      datasets: [
        {
          label: 'Base',
          data: [r.baseXva.bcva, -r.baseXva.bdva, r.baseXva.fva, r.baseXva.total],
          backgroundColor: 'rgba(59,130,246,0.6)',
        },
        {
          label: 'Full',
          data: [r.fullXva.bcva, -r.fullXva.bdva, r.fullXva.fva, r.fullXva.total],
          backgroundColor: 'rgba(16,185,129,0.6)',
        },
        {
          label: 'Incremental',
          data: [r.incrementalXva.bcva, -r.incrementalXva.bdva, r.incrementalXva.fva, r.incrementalXva.total],
          backgroundColor: 'rgba(245,158,11,0.8)',
        },
      ],
    },
    options: {
      responsive: true,
      plugins: {
        legend: { position: 'top' },
        tooltip: {
          callbacks: {
            label: (ctx: any) => `${ctx.dataset.label}: ${formatCurrency(ctx.raw)}`,
          },
        },
      },
      scales: {
        y: {
          title: { display: true, text: 'XVA Value' },
        },
      },
    },
  })
}

// Auto-load demo on mount
loadDemo()
</script>

<style scoped>
.incremental-xva-view {
  padding: 1.5rem;
  max-width: 1400px;
  margin: 0 auto;
}

.page-header {
  margin-bottom: 1.5rem;
}

.page-header h1 {
  font-size: 1.5rem;
  font-weight: 700;
  margin: 0;
}

.subtitle {
  color: #6b7280;
  font-size: 0.875rem;
  margin-top: 0.25rem;
}

.config-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 1rem;
  margin-bottom: 1.5rem;
}

.config-card {
  background: var(--color-bg-secondary, #f9fafb);
  border: 1px solid var(--color-border, #e5e7eb);
  border-radius: 8px;
  padding: 1rem;
}

.config-card h3 {
  font-size: 0.875rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: #6b7280;
  margin: 0 0 0.75rem;
}

.form-group {
  margin-bottom: 0.5rem;
}

.form-group label {
  display: block;
  font-size: 0.8rem;
  color: #374151;
  margin-bottom: 0.25rem;
}

.form-group input[type="number"],
.form-group select {
  width: 100%;
  padding: 0.375rem 0.5rem;
  border: 1px solid #d1d5db;
  border-radius: 4px;
  font-size: 0.8rem;
  background: white;
}

.form-group input[type="radio"] {
  margin-right: 0.5rem;
}

.toggle-group {
  display: flex;
  gap: 0.75rem;
  margin-top: 0.5rem;
}

.toggle-group label {
  font-size: 0.8rem;
  display: flex;
  align-items: center;
  gap: 0.25rem;
}

.portfolio-section {
  display: grid;
  grid-template-columns: 2fr 1fr;
  gap: 1rem;
  margin-bottom: 1.5rem;
}

.portfolio-card {
  background: var(--color-bg-secondary, #f9fafb);
  border: 1px solid var(--color-border, #e5e7eb);
  border-radius: 8px;
  padding: 1rem;
}

.portfolio-card.incremental {
  border-color: #f59e0b;
  background: #fffbeb;
}

.portfolio-card h3 {
  font-size: 0.875rem;
  font-weight: 600;
  margin: 0 0 0.75rem;
}

.trade-list {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
}

.trade-item {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.375rem 0.5rem;
  background: white;
  border-radius: 4px;
  font-size: 0.8rem;
  border: 1px solid #e5e7eb;
}

.trade-item.highlight {
  border-color: #f59e0b;
  background: #fef3c7;
}

.trade-badge {
  padding: 0.125rem 0.375rem;
  border-radius: 3px;
  font-size: 0.7rem;
  font-weight: 600;
  text-transform: uppercase;
}

.trade-badge.swap {
  background: #dbeafe;
  color: #1d4ed8;
}

.trade-badge.exotic {
  background: #fce7f3;
  color: #be185d;
}

.actions {
  display: flex;
  gap: 0.75rem;
  margin-bottom: 1.5rem;
}

.btn {
  padding: 0.5rem 1.25rem;
  border-radius: 6px;
  font-weight: 600;
  font-size: 0.875rem;
  cursor: pointer;
  border: none;
  transition: all 0.15s;
}

.btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.btn-primary {
  background: #3b82f6;
  color: white;
}

.btn-primary:hover:not(:disabled) {
  background: #2563eb;
}

.btn-secondary {
  background: #e5e7eb;
  color: #374151;
}

.btn-secondary:hover:not(:disabled) {
  background: #d1d5db;
}

.spinner {
  display: inline-block;
  width: 14px;
  height: 14px;
  border: 2px solid rgba(255,255,255,0.3);
  border-top-color: white;
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
  margin-right: 0.5rem;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.results-section {
  margin-top: 1.5rem;
}

.summary-cards {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 1rem;
  margin-bottom: 1.5rem;
}

.summary-card {
  background: var(--color-bg-secondary, #f9fafb);
  border: 1px solid var(--color-border, #e5e7eb);
  border-radius: 8px;
  padding: 1rem;
}

.summary-card.highlight {
  border-color: #f59e0b;
  background: #fffbeb;
}

.summary-card h4 {
  font-size: 0.8rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: #6b7280;
  margin: 0 0 0.5rem;
}

.summary-card .value {
  font-size: 1.25rem;
  font-weight: 700;
  margin-bottom: 0.5rem;
}

.summary-card .value.large {
  font-size: 1.5rem;
}

.summary-card .value.positive { color: #059669; }
.summary-card .value.negative { color: #dc2626; }

.summary-card .detail {
  font-size: 0.75rem;
  color: #6b7280;
}

.computation-info {
  display: flex;
  gap: 1.5rem;
  margin-bottom: 1.5rem;
  font-size: 0.8rem;
  color: #6b7280;
}

.computation-info span {
  padding: 0.25rem 0.5rem;
  background: #f3f4f6;
  border-radius: 4px;
}

.chart-section {
  background: var(--color-bg-secondary, #f9fafb);
  border: 1px solid var(--color-border, #e5e7eb);
  border-radius: 8px;
  padding: 1rem;
  margin-bottom: 1.5rem;
}

.chart-section h3 {
  font-size: 0.875rem;
  font-weight: 600;
  margin: 0 0 0.75rem;
}

.table-section {
  background: var(--color-bg-secondary, #f9fafb);
  border: 1px solid var(--color-border, #e5e7eb);
  border-radius: 8px;
  padding: 1rem;
}

.table-section h3 {
  font-size: 0.875rem;
  font-weight: 600;
  margin: 0 0 0.75rem;
}

table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.8rem;
}

thead th {
  background: #f3f4f6;
  padding: 0.5rem;
  text-align: right;
  font-weight: 600;
  border-bottom: 2px solid #e5e7eb;
}

thead th:first-child {
  text-align: left;
}

tbody td {
  padding: 0.5rem;
  text-align: right;
  border-bottom: 1px solid #e5e7eb;
}

tbody td:first-child {
  text-align: left;
  font-weight: 500;
}

td.positive { color: #059669; }
td.negative { color: #dc2626; }
</style>
