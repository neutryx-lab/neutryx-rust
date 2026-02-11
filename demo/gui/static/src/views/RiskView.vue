<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick } from 'vue';
import { Chart, registerables, type TooltipItem, type ChartConfiguration } from 'chart.js';
import { getChartColors } from '@/composables/useChartTheme';

Chart.register(...registerables);

// Types
interface GreekRow {
  asset: string;
  delta: number;
  gamma: number;
  vega: number;
  theta: number;
  rho: number;
}

interface RiskLimit {
  name: string;
  current: number;
  limit: number;
  unit: string;
  icon: string;
}

interface SensitivityBucket {
  tenor: string;
  dv01: number;
  cs01: number;
  vega01: number;
}

interface VaRBreakdown {
  category: string;
  standalone: number;
  diversified: number;
  color: string;
}

type RiskTab = 'overview' | 'greeks' | 'limits';
type VaRConfidence = '95' | '99';
type VaRHorizon = '1d' | '10d';

// State
const activeTab = ref<RiskTab>('overview');
const varConfidence = ref<VaRConfidence>('99');
const varHorizon = ref<VaRHorizon>('10d');
const valuationDate = ref('2025-01-15');

// Chart refs
const varHistChart = ref<Chart | null>(null);
const varHistContainer = ref<HTMLDivElement | null>(null);
const riskContribChart = ref<Chart | null>(null);
const riskContribContainer = ref<HTMLDivElement | null>(null);
const sensChart = ref<Chart | null>(null);
const sensContainer = ref<HTMLDivElement | null>(null);

// Mock data
const greeksData = computed<GreekRow[]>(() => [
  { asset: 'IR Swaps', delta: -2340000, gamma: 185000, vega: 892000, theta: -45200, rho: -1230000 },
  { asset: 'IR Options', delta: 580000, gamma: 1420000, vega: 3150000, theta: -128000, rho: -340000 },
  { asset: 'FX Forwards', delta: 4120000, gamma: 32000, vega: 0, theta: -18500, rho: 210000 },
  { asset: 'FX Options', delta: -890000, gamma: 670000, vega: 1850000, theta: -67000, rho: -95000 },
  { asset: 'Credit CDS', delta: -1560000, gamma: 0, vega: 0, theta: -32000, rho: -890000 },
  { asset: 'Equity Options', delta: 1230000, gamma: 980000, vega: 2100000, theta: -89000, rho: 150000 },
]);

const greeksTotals = computed(() => {
  const totals = { delta: 0, gamma: 0, vega: 0, theta: 0, rho: 0 };
  for (const row of greeksData.value) {
    totals.delta += row.delta;
    totals.gamma += row.gamma;
    totals.vega += row.vega;
    totals.theta += row.theta;
    totals.rho += row.rho;
  }
  return totals;
});

const riskLimits = computed<RiskLimit[]>(() => [
  { name: 'Total VaR (99%, 10d)', current: 18.7, limit: 25.0, unit: '$M', icon: 'fa-shield-alt' },
  { name: 'Delta Limit', current: 11.4, limit: 15.0, unit: '$M', icon: 'fa-arrows-alt-h' },
  { name: 'Gamma Limit', current: 3.29, limit: 5.0, unit: '$M', icon: 'fa-wave-square' },
  { name: 'Vega Limit', current: 7.99, limit: 10.0, unit: '$M', icon: 'fa-cloud' },
  { name: 'CS01 Limit', current: 1.8, limit: 3.0, unit: '$M', icon: 'fa-credit-card' },
  { name: 'Concentration (single name)', current: 4.2, limit: 5.0, unit: '$M', icon: 'fa-bullseye' },
]);

const sensitivityBuckets = computed<SensitivityBucket[]>(() => [
  { tenor: 'ON', dv01: -12, cs01: -2, vega01: 0 },
  { tenor: '1M', dv01: -45, cs01: -8, vega01: 15 },
  { tenor: '3M', dv01: -120, cs01: -25, vega01: 42 },
  { tenor: '6M', dv01: -85, cs01: -18, vega01: 68 },
  { tenor: '1Y', dv01: 230, cs01: 45, vega01: 185 },
  { tenor: '2Y', dv01: 380, cs01: 82, vega01: 320 },
  { tenor: '5Y', dv01: -520, cs01: 120, vega01: 450 },
  { tenor: '10Y', dv01: -310, cs01: 65, vega01: 280 },
  { tenor: '15Y', dv01: 180, cs01: 35, vega01: 120 },
  { tenor: '20Y', dv01: 95, cs01: 18, vega01: 55 },
  { tenor: '30Y', dv01: -150, cs01: -30, vega01: 25 },
]);

const varBreakdown = computed<VaRBreakdown[]>(() => [
  { category: 'Interest Rates', standalone: 12.4, diversified: 8.9, color: '#3b82f6' },
  { category: 'FX', standalone: 8.2, diversified: 5.1, color: '#8b5cf6' },
  { category: 'Credit', standalone: 6.8, diversified: 4.3, color: '#f59e0b' },
  { category: 'Equity', standalone: 5.1, diversified: 3.2, color: '#10b981' },
  { category: 'Volatility', standalone: 4.5, diversified: 2.8, color: '#ef4444' },
]);

const totalVaR = computed(() => {
  const base = varConfidence.value === '99' ? 18.7 : 12.3;
  return varHorizon.value === '10d' ? base : base / Math.sqrt(10);
});

const totalCVaR = computed(() => totalVaR.value * 1.42);
const capitalReq = computed(() => totalVaR.value * 3 + 45.2);

// Summary stats
const summaryStats = computed(() => [
  { label: 'Valuation Date', value: valuationDate.value, icon: 'fa-calendar', color: '#8b5cf6' },
  { label: `VaR (${varConfidence.value}%, ${varHorizon.value})`, value: `$${totalVaR.value.toFixed(1)}M`, icon: 'fa-shield-alt', color: '#ef4444' },
  { label: `CVaR (${varConfidence.value}%)`, value: `$${totalCVaR.value.toFixed(1)}M`, icon: 'fa-exclamation-triangle', color: '#f59e0b' },
  { label: 'Capital Requirement', value: `$${capitalReq.value.toFixed(1)}M`, icon: 'fa-landmark', color: '#3b82f6' },
]);

// Formatting
function formatGreek(value: number): string {
  const absVal = Math.abs(value);
  if (absVal >= 1000000) {
    return `${value > 0 ? '+' : ''}${(value / 1000000).toFixed(2)}M`;
  }
  if (absVal >= 1000) {
    return `${value > 0 ? '+' : ''}${(value / 1000).toFixed(0)}K`;
  }
  return `${value > 0 ? '+' : ''}${value.toFixed(0)}`;
}

function greekClass(value: number): string {
  if (value > 0) return 'text-[#10b981]';
  if (value < 0) return 'text-[#ef4444]';
  return 'text-[var(--text-muted)]';
}

function limitPct(current: number, limit: number): number {
  return Math.min(100, (current / limit) * 100);
}

function limitColour(pct: number): string {
  if (pct >= 90) return '#ef4444';
  if (pct >= 70) return '#f59e0b';
  return '#10b981';
}

// Chart: VaR histogram
function renderVaRHistogram() {
  if (!varHistContainer.value) return;
  if (varHistChart.value) { varHistChart.value.destroy(); varHistChart.value = null; }

  varHistContainer.value.innerHTML = '';
  const canvas = document.createElement('canvas');
  varHistContainer.value.appendChild(canvas);

  // Generate P&L distribution data
  const bins = 40;
  const mean = 0.2;
  const std = totalVaR.value * 0.6;
  const binWidth = std * 6 / bins;
  const labels: string[] = [];
  const data: number[] = [];
  const colors: string[] = [];

  const varThreshold = -totalVaR.value;

  for (let i = 0; i < bins; i++) {
    const x = mean - std * 3 + i * binWidth;
    const freq = Math.exp(-0.5 * Math.pow((x - mean) / std, 2)) * (200 + Math.random() * 30);
    labels.push(`${x.toFixed(1)}`);
    data.push(Math.round(freq));
    colors.push(x < varThreshold ? 'rgba(239, 68, 68, 0.8)' : 'rgba(59, 130, 246, 0.6)');
  }

  const cc = getChartColors();
  const config: ChartConfiguration<'bar'> = {
    type: 'bar',
    data: {
      labels,
      datasets: [{
        label: 'Frequency',
        data,
        backgroundColor: colors,
        borderColor: colors.map(c => c.replace(/[\d.]+\)$/, '1)')),
        borderWidth: 1,
        borderRadius: 1,
        barPercentage: 1.0,
        categoryPercentage: 1.0,
      }],
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      plugins: {
        legend: { display: false },
        tooltip: {
          backgroundColor: cc.tooltipBg,
          titleColor: cc.tooltipTitle,
          bodyColor: cc.tooltipBody,
          padding: 12,
          cornerRadius: 8,
          callbacks: {
            title: (items) => `P&L: $${items[0].label}M`,
            label: (ctx: TooltipItem<'bar'>) => `Observations: ${ctx.parsed.y}`,
          },
        },
      },
      scales: {
        x: {
          grid: { display: false },
          ticks: {
            color: cc.tick,
            font: { size: 10 },
            maxTicksLimit: 8,
            callback: function(_, index) {
              return index % 5 === 0 ? `$${labels[index]}M` : '';
            },
          },
        },
        y: {
          grid: { color: cc.grid },
          ticks: { color: cc.tick, font: { size: 10 } },
        },
      },
    },
  };

  varHistChart.value = new Chart(canvas, config);
}

// Chart: Risk contribution doughnut
function renderRiskContribution() {
  if (!riskContribContainer.value) return;
  if (riskContribChart.value) { riskContribChart.value.destroy(); riskContribChart.value = null; }

  riskContribContainer.value.innerHTML = '';
  const canvas = document.createElement('canvas');
  riskContribContainer.value.appendChild(canvas);

  const bd = varBreakdown.value;

  const cc = getChartColors();
  const config: ChartConfiguration<'doughnut'> = {
    type: 'doughnut',
    data: {
      labels: bd.map(b => b.category),
      datasets: [{
        data: bd.map(b => b.diversified),
        backgroundColor: bd.map(b => `${b.color}cc`),
        borderColor: bd.map(b => b.color),
        borderWidth: 2,
        hoverOffset: 8,
      }],
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      cutout: '65%',
      plugins: {
        legend: { display: false },
        tooltip: {
          backgroundColor: cc.tooltipBg,
          titleColor: cc.tooltipTitle,
          bodyColor: cc.tooltipBody,
          padding: 12,
          cornerRadius: 8,
          callbacks: {
            label: (ctx: TooltipItem<'doughnut'>) => {
              const val = ctx.parsed;
              const total = bd.reduce((s, b) => s + b.diversified, 0);
              const pct = ((val / total) * 100).toFixed(1);
              return `$${val.toFixed(1)}M (${pct}%)`;
            },
          },
        },
      },
    },
  };

  riskContribChart.value = new Chart(canvas, config);
}

// Chart: Sensitivity ladder
function renderSensitivityChart() {
  if (!sensContainer.value) return;
  if (sensChart.value) { sensChart.value.destroy(); sensChart.value = null; }

  sensContainer.value.innerHTML = '';
  const canvas = document.createElement('canvas');
  sensContainer.value.appendChild(canvas);

  const buckets = sensitivityBuckets.value;

  const cc = getChartColors();
  const config: ChartConfiguration<'bar'> = {
    type: 'bar',
    data: {
      labels: buckets.map(b => b.tenor),
      datasets: [
        {
          label: 'DV01',
          data: buckets.map(b => b.dv01),
          backgroundColor: 'rgba(59, 130, 246, 0.7)',
          borderColor: '#3b82f6',
          borderWidth: 1,
          borderRadius: 3,
        },
        {
          label: 'CS01',
          data: buckets.map(b => b.cs01),
          backgroundColor: 'rgba(245, 158, 11, 0.7)',
          borderColor: '#f59e0b',
          borderWidth: 1,
          borderRadius: 3,
        },
        {
          label: 'Vega01',
          data: buckets.map(b => b.vega01),
          backgroundColor: 'rgba(139, 92, 246, 0.7)',
          borderColor: '#8b5cf6',
          borderWidth: 1,
          borderRadius: 3,
        },
      ],
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      plugins: {
        legend: { display: false },
        tooltip: {
          backgroundColor: cc.tooltipBg,
          titleColor: cc.tooltipTitle,
          bodyColor: cc.tooltipBody,
          padding: 12,
          cornerRadius: 8,
          callbacks: {
            label: (ctx: TooltipItem<'bar'>) => {
              const val = ctx.parsed.y ?? 0;
              return `${ctx.dataset.label}: ${val > 0 ? '+' : ''}$${val}K/bp`;
            },
          },
        },
      },
      scales: {
        x: {
          grid: { display: false },
          ticks: { color: cc.tick, font: { size: 10 } },
        },
        y: {
          grid: { color: cc.grid },
          ticks: {
            color: cc.tick,
            font: { size: 10 },
            callback: (v) => `$${v}K`,
          },
        },
      },
    },
  };

  sensChart.value = new Chart(canvas, config);
}

function renderAllCharts() {
  renderVaRHistogram();
  nextTick(() => {
    renderRiskContribution();
    nextTick(() => renderSensitivityChart());
  });
}

// Tab change handler
function switchTab(tab: RiskTab) {
  activeTab.value = tab;
  if (tab === 'overview') {
    nextTick(() => renderAllCharts());
  } else if (tab === 'greeks') {
    nextTick(() => renderSensitivityChart());
  }
}

onMounted(() => {
  nextTick(() => renderAllCharts());
});

onUnmounted(() => {
  if (varHistChart.value) { varHistChart.value.destroy(); varHistChart.value = null; }
  if (riskContribChart.value) { riskContribChart.value.destroy(); riskContribChart.value = null; }
  if (sensChart.value) { sensChart.value.destroy(); sensChart.value = null; }
});
</script>

<template>
  <div class="risk-view">
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

    <!-- Tab Navigation -->
    <div class="flex gap-2 mb-6">
      <button
        v-for="tab in ([
          { key: 'overview', label: 'Overview', icon: 'fa-chart-pie' },
          { key: 'greeks', label: 'Greeks & Sensitivities', icon: 'fa-th' },
          { key: 'limits', label: 'Risk Limits', icon: 'fa-tachometer-alt' },
        ] as const)"
        :key="tab.key"
        :class="[
          'px-4 py-2 rounded-lg text-sm font-medium transition-all duration-200 flex items-center gap-2',
          activeTab === tab.key
            ? 'bg-[var(--primary)] text-white'
            : 'text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]'
        ]"
        @click="switchTab(tab.key)"
      >
        <i :class="['fas', tab.icon]"></i>
        {{ tab.label }}
      </button>

      <!-- VaR Parameters (right-aligned) -->
      <div class="ml-auto flex items-center gap-3">
        <div class="flex items-center gap-2">
          <span class="text-xs text-[var(--text-muted)]">Confidence:</span>
          <div class="flex gap-1 p-0.5 bg-[var(--surface)] rounded-md">
            <button
              v-for="c in (['95', '99'] as const)"
              :key="c"
              :class="[
                'px-2 py-1 rounded text-xs font-medium transition-all',
                varConfidence === c ? 'bg-[var(--primary)] text-white' : 'text-[var(--text-secondary)]'
              ]"
              @click="varConfidence = c; nextTick(() => renderAllCharts())"
            >
              {{ c }}%
            </button>
          </div>
        </div>
        <div class="flex items-center gap-2">
          <span class="text-xs text-[var(--text-muted)]">Horizon:</span>
          <div class="flex gap-1 p-0.5 bg-[var(--surface)] rounded-md">
            <button
              v-for="h in (['1d', '10d'] as const)"
              :key="h"
              :class="[
                'px-2 py-1 rounded text-xs font-medium transition-all',
                varHorizon === h ? 'bg-[var(--primary)] text-white' : 'text-[var(--text-secondary)]'
              ]"
              @click="varHorizon = h; nextTick(() => renderAllCharts())"
            >
              {{ h }}
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- ==================== OVERVIEW TAB ==================== -->
    <template v-if="activeTab === 'overview'">
      <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <!-- VaR Distribution -->
        <div class="lg:col-span-2 glass-card p-5">
          <div class="flex items-center justify-between mb-3">
            <h3 class="text-base font-semibold text-[var(--text-primary)]">
              <i class="fas fa-chart-bar text-sm mr-2 text-[var(--text-muted)]"></i>
              P&L Distribution
            </h3>
            <span class="text-xs text-[var(--text-muted)]">
              <span class="inline-block w-2.5 h-2.5 rounded-sm mr-1" style="background: rgba(239, 68, 68, 0.8); vertical-align: middle;"></span>
              VaR tail region
            </span>
          </div>
          <div ref="varHistContainer" class="h-56"></div>
        </div>

        <!-- Risk Contribution Doughnut -->
        <div class="glass-card p-5">
          <h3 class="text-base font-semibold text-[var(--text-primary)] mb-3">
            <i class="fas fa-chart-pie text-sm mr-2 text-[var(--text-muted)]"></i>
            VaR Contribution
          </h3>
          <div ref="riskContribContainer" class="h-44"></div>
          <div class="mt-3 space-y-2">
            <div
              v-for="item in varBreakdown"
              :key="item.category"
              class="flex items-center justify-between text-xs"
            >
              <div class="flex items-center gap-2">
                <span class="w-2.5 h-2.5 rounded-sm" :style="{ backgroundColor: item.color }"></span>
                <span class="text-[var(--text-secondary)]">{{ item.category }}</span>
              </div>
              <div class="flex gap-3">
                <span class="text-[var(--text-muted)]">${{ item.standalone.toFixed(1) }}M</span>
                <span class="font-medium text-[var(--text-primary)]">${{ item.diversified.toFixed(1) }}M</span>
              </div>
            </div>
            <div class="flex items-center justify-between text-xs pt-2 border-t border-[var(--glass-border)]">
              <span class="text-[var(--text-muted)] font-medium">Diversification Benefit</span>
              <span class="text-[#10b981] font-medium">
                -${{ (varBreakdown.reduce((s, b) => s + b.standalone, 0) - totalVaR).toFixed(1) }}M
              </span>
            </div>
          </div>
        </div>
      </div>

      <!-- VaR Breakdown Table -->
      <div class="glass-card p-5 mt-6">
        <h3 class="text-base font-semibold text-[var(--text-primary)] mb-3">
          <i class="fas fa-table text-sm mr-2 text-[var(--text-muted)]"></i>
          VaR Breakdown
        </h3>
        <div class="overflow-x-auto">
          <table class="w-full text-sm">
            <thead>
              <tr class="border-b border-[var(--glass-border)]">
                <th class="text-left py-2 px-3 text-xs font-medium text-[var(--text-muted)]">Risk Factor</th>
                <th class="text-right py-2 px-3 text-xs font-medium text-[var(--text-muted)]">Standalone VaR</th>
                <th class="text-right py-2 px-3 text-xs font-medium text-[var(--text-muted)]">Diversified VaR</th>
                <th class="text-right py-2 px-3 text-xs font-medium text-[var(--text-muted)]">% of Total</th>
                <th class="py-2 px-3 text-xs font-medium text-[var(--text-muted)] w-40">Contribution</th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="item in varBreakdown"
                :key="item.category"
                class="border-b border-[var(--glass-border)] hover:bg-[var(--surface-hover)] transition-colors"
              >
                <td class="py-2.5 px-3">
                  <div class="flex items-center gap-2">
                    <span class="w-2 h-2 rounded-full" :style="{ backgroundColor: item.color }"></span>
                    <span class="text-[var(--text-primary)] text-xs font-medium">{{ item.category }}</span>
                  </div>
                </td>
                <td class="py-2.5 px-3 text-right text-xs text-[var(--text-secondary)] font-mono">${{ item.standalone.toFixed(1) }}M</td>
                <td class="py-2.5 px-3 text-right text-xs text-[var(--text-primary)] font-mono font-medium">${{ item.diversified.toFixed(1) }}M</td>
                <td class="py-2.5 px-3 text-right text-xs text-[var(--text-secondary)] font-mono">{{ ((item.diversified / totalVaR) * 100).toFixed(1) }}%</td>
                <td class="py-2.5 px-3">
                  <div class="h-2 rounded-full bg-[var(--surface)] overflow-hidden">
                    <div
                      class="h-2 rounded-full transition-all duration-500"
                      :style="{ width: `${(item.diversified / totalVaR) * 100}%`, backgroundColor: item.color }"
                    ></div>
                  </div>
                </td>
              </tr>
              <tr class="font-medium">
                <td class="py-2.5 px-3 text-xs text-[var(--text-primary)]">Total (Diversified)</td>
                <td class="py-2.5 px-3 text-right text-xs text-[var(--text-muted)] font-mono">${{ varBreakdown.reduce((s, b) => s + b.standalone, 0).toFixed(1) }}M</td>
                <td class="py-2.5 px-3 text-right text-xs text-[var(--text-primary)] font-mono">${{ totalVaR.toFixed(1) }}M</td>
                <td class="py-2.5 px-3 text-right text-xs text-[var(--text-primary)] font-mono">100.0%</td>
                <td class="py-2.5 px-3"></td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </template>

    <!-- ==================== GREEKS TAB ==================== -->
    <template v-if="activeTab === 'greeks'">
      <!-- Greeks Heatmap Table -->
      <div class="glass-card p-5 mb-6">
        <h3 class="text-base font-semibold text-[var(--text-primary)] mb-3">
          <i class="fas fa-th text-sm mr-2 text-[var(--text-muted)]"></i>
          Greeks by Asset Class
        </h3>
        <div class="overflow-x-auto">
          <table class="w-full text-sm">
            <thead>
              <tr class="border-b border-[var(--glass-border)]">
                <th class="text-left py-2 px-3 text-xs font-medium text-[var(--text-muted)]">Asset Class</th>
                <th class="text-right py-2 px-3 text-xs font-medium text-[var(--text-muted)]">
                  <span class="inline-flex items-center gap-1">Delta <span class="text-[10px] opacity-60">($/bp)</span></span>
                </th>
                <th class="text-right py-2 px-3 text-xs font-medium text-[var(--text-muted)]">
                  <span class="inline-flex items-center gap-1">Gamma <span class="text-[10px] opacity-60">($/bp²)</span></span>
                </th>
                <th class="text-right py-2 px-3 text-xs font-medium text-[var(--text-muted)]">
                  <span class="inline-flex items-center gap-1">Vega <span class="text-[10px] opacity-60">($/vol)</span></span>
                </th>
                <th class="text-right py-2 px-3 text-xs font-medium text-[var(--text-muted)]">
                  <span class="inline-flex items-center gap-1">Theta <span class="text-[10px] opacity-60">($/day)</span></span>
                </th>
                <th class="text-right py-2 px-3 text-xs font-medium text-[var(--text-muted)]">
                  <span class="inline-flex items-center gap-1">Rho <span class="text-[10px] opacity-60">($/bp)</span></span>
                </th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="row in greeksData"
                :key="row.asset"
                class="border-b border-[var(--glass-border)] hover:bg-[var(--surface-hover)] transition-colors"
              >
                <td class="py-2.5 px-3 text-xs font-medium text-[var(--text-primary)]">{{ row.asset }}</td>
                <td class="py-2.5 px-3 text-right text-xs font-mono" :class="greekClass(row.delta)">{{ formatGreek(row.delta) }}</td>
                <td class="py-2.5 px-3 text-right text-xs font-mono" :class="greekClass(row.gamma)">{{ formatGreek(row.gamma) }}</td>
                <td class="py-2.5 px-3 text-right text-xs font-mono" :class="greekClass(row.vega)">{{ formatGreek(row.vega) }}</td>
                <td class="py-2.5 px-3 text-right text-xs font-mono" :class="greekClass(row.theta)">{{ formatGreek(row.theta) }}</td>
                <td class="py-2.5 px-3 text-right text-xs font-mono" :class="greekClass(row.rho)">{{ formatGreek(row.rho) }}</td>
              </tr>
              <tr class="font-medium bg-[var(--surface)]">
                <td class="py-2.5 px-3 text-xs text-[var(--text-primary)]">Total</td>
                <td class="py-2.5 px-3 text-right text-xs font-mono" :class="greekClass(greeksTotals.delta)">{{ formatGreek(greeksTotals.delta) }}</td>
                <td class="py-2.5 px-3 text-right text-xs font-mono" :class="greekClass(greeksTotals.gamma)">{{ formatGreek(greeksTotals.gamma) }}</td>
                <td class="py-2.5 px-3 text-right text-xs font-mono" :class="greekClass(greeksTotals.vega)">{{ formatGreek(greeksTotals.vega) }}</td>
                <td class="py-2.5 px-3 text-right text-xs font-mono" :class="greekClass(greeksTotals.theta)">{{ formatGreek(greeksTotals.theta) }}</td>
                <td class="py-2.5 px-3 text-right text-xs font-mono" :class="greekClass(greeksTotals.rho)">{{ formatGreek(greeksTotals.rho) }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <!-- Sensitivity Ladder Chart -->
      <div class="glass-card p-5">
        <div class="flex items-center justify-between mb-3">
          <h3 class="text-base font-semibold text-[var(--text-primary)]">
            <i class="fas fa-chart-bar text-sm mr-2 text-[var(--text-muted)]"></i>
            Sensitivity Ladder (per bp)
          </h3>
          <div class="flex gap-3">
            <span class="flex items-center gap-1.5 text-xs text-[var(--text-secondary)]">
              <span class="w-2.5 h-2.5 rounded-sm" style="background: rgba(59, 130, 246, 0.7);"></span>
              DV01
            </span>
            <span class="flex items-center gap-1.5 text-xs text-[var(--text-secondary)]">
              <span class="w-2.5 h-2.5 rounded-sm" style="background: rgba(245, 158, 11, 0.7);"></span>
              CS01
            </span>
            <span class="flex items-center gap-1.5 text-xs text-[var(--text-secondary)]">
              <span class="w-2.5 h-2.5 rounded-sm" style="background: rgba(139, 92, 246, 0.7);"></span>
              Vega01
            </span>
          </div>
        </div>
        <div ref="sensContainer" class="h-64"></div>
      </div>
    </template>

    <!-- ==================== LIMITS TAB ==================== -->
    <template v-if="activeTab === 'limits'">
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <!-- Risk Limits Utilisation -->
        <div class="glass-card p-5">
          <h3 class="text-base font-semibold text-[var(--text-primary)] mb-4">
            <i class="fas fa-tachometer-alt text-sm mr-2 text-[var(--text-muted)]"></i>
            Limit Utilisation
          </h3>
          <div class="space-y-5">
            <div
              v-for="rl in riskLimits"
              :key="rl.name"
            >
              <div class="flex items-center justify-between mb-1.5">
                <div class="flex items-center gap-2">
                  <i :class="['fas', rl.icon, 'text-xs']" :style="{ color: limitColour(limitPct(rl.current, rl.limit)) }"></i>
                  <span class="text-xs text-[var(--text-secondary)]">{{ rl.name }}</span>
                </div>
                <div class="flex items-center gap-2 text-xs">
                  <span class="font-mono font-medium" :style="{ color: limitColour(limitPct(rl.current, rl.limit)) }">
                    {{ rl.unit }}{{ rl.current.toFixed(1) }}
                  </span>
                  <span class="text-[var(--text-muted)]">/</span>
                  <span class="text-[var(--text-muted)] font-mono">{{ rl.unit }}{{ rl.limit.toFixed(1) }}</span>
                </div>
              </div>
              <div class="h-3 rounded-full bg-[var(--surface)] overflow-hidden">
                <div
                  class="h-3 rounded-full transition-all duration-700"
                  :style="{
                    width: `${limitPct(rl.current, rl.limit)}%`,
                    backgroundColor: limitColour(limitPct(rl.current, rl.limit)),
                  }"
                ></div>
              </div>
              <div class="text-right mt-0.5">
                <span
                  class="text-[10px] font-medium"
                  :style="{ color: limitColour(limitPct(rl.current, rl.limit)) }"
                >
                  {{ limitPct(rl.current, rl.limit).toFixed(0) }}%
                </span>
              </div>
            </div>
          </div>
        </div>

        <!-- Limit Breach Summary -->
        <div class="space-y-6">
          <div class="glass-card p-5">
            <h3 class="text-base font-semibold text-[var(--text-primary)] mb-3">
              <i class="fas fa-bell text-sm mr-2 text-[var(--text-muted)]"></i>
              Alerts & Breaches
            </h3>
            <div class="space-y-3">
              <div class="flex items-start gap-3 p-3 rounded-lg bg-[#f59e0b1a] border border-[#f59e0b33]">
                <i class="fas fa-exclamation-triangle text-[#f59e0b] mt-0.5"></i>
                <div>
                  <p class="text-xs font-medium text-[var(--text-primary)]">Concentration Limit Warning</p>
                  <p class="text-[10px] text-[var(--text-muted)] mt-0.5">Single-name CDS exposure at 84% of limit. Consider reducing position in ACME Corp 5Y CDS.</p>
                </div>
              </div>
              <div class="flex items-start gap-3 p-3 rounded-lg bg-[#f59e0b1a] border border-[#f59e0b33]">
                <i class="fas fa-exclamation-triangle text-[#f59e0b] mt-0.5"></i>
                <div>
                  <p class="text-xs font-medium text-[var(--text-primary)]">Vega Limit Approaching</p>
                  <p class="text-[10px] text-[var(--text-muted)] mt-0.5">Portfolio vega at 80% utilisation. Trigger level: 85%.</p>
                </div>
              </div>
              <div class="flex items-start gap-3 p-3 rounded-lg bg-[#10b9811a] border border-[#10b98133]">
                <i class="fas fa-check-circle text-[#10b981] mt-0.5"></i>
                <div>
                  <p class="text-xs font-medium text-[var(--text-primary)]">Daily VaR Within Limits</p>
                  <p class="text-[10px] text-[var(--text-muted)] mt-0.5">All VaR metrics within approved trading limits. Last breach: 14 days ago.</p>
                </div>
              </div>
            </div>
          </div>

          <!-- Regulatory Capital -->
          <div class="glass-card p-5">
            <h3 class="text-base font-semibold text-[var(--text-primary)] mb-3">
              <i class="fas fa-landmark text-sm mr-2 text-[var(--text-muted)]"></i>
              Regulatory Capital (FRTB)
            </h3>
            <div class="space-y-3">
              <div class="flex items-center justify-between text-xs">
                <span class="text-[var(--text-secondary)]">IMA Capital Charge</span>
                <span class="text-[var(--text-primary)] font-mono font-medium">$101.3M</span>
              </div>
              <div class="flex items-center justify-between text-xs">
                <span class="text-[var(--text-secondary)]">SA Capital Charge</span>
                <span class="text-[var(--text-primary)] font-mono font-medium">$128.7M</span>
              </div>
              <div class="flex items-center justify-between text-xs">
                <span class="text-[var(--text-secondary)]">DRC (Default Risk)</span>
                <span class="text-[var(--text-primary)] font-mono font-medium">$22.4M</span>
              </div>
              <div class="flex items-center justify-between text-xs">
                <span class="text-[var(--text-secondary)]">RRAO (Residual)</span>
                <span class="text-[var(--text-primary)] font-mono font-medium">$4.6M</span>
              </div>
              <div class="flex items-center justify-between text-xs pt-2 border-t border-[var(--glass-border)]">
                <span class="text-[var(--text-primary)] font-medium">Total Capital</span>
                <span class="text-[var(--text-primary)] font-mono font-semibold">${{ capitalReq.toFixed(1) }}M</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </template>
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
</style>
