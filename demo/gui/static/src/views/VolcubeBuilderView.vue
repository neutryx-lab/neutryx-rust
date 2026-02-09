<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue';

// Types
interface SmilePoint {
  strikeOffsetBp: number;
  vol: number;
}

interface SwaptionInstrument {
  expiry: string;
  tenor: string;
  atmVol: number;
  smile: SmilePoint[];
  enabled: boolean;
}

interface FxQuote {
  expiry: number;
  atmVol: number;
  rr25d: number;
  bf25d: number;
  rr10d?: number;
  bf10d?: number;
}

interface CalibrationResult {
  surfaceId: string;
  model: string;
  parameters: Record<string, number>;
  errors: Array<{ expiry: string; tenor?: string; error: number }>;
  metadata: {
    instrumentCount: number;
    processingTimeMs: number;
  };
}

type AssetTab = 'swaption' | 'fx';

// Canonical sort orders
const EXPIRY_ORDER = ['1M', '3M', '6M', '1Y', '2Y', '5Y', '10Y', '15Y', '20Y', '30Y'];
const TENOR_ORDER = ['1Y', '2Y', '5Y', '10Y', '15Y', '20Y', '30Y'];

// State
const activeTab = ref<AssetTab>('swaption');
const swaptionIndices = ref<string[]>([]);
const selectedSwaptionIndex = ref('');
const swaptionInstruments = ref<SwaptionInstrument[]>([]);
const swaptionModels = ref<string[]>([]);
const selectedModel = ref('');
const referenceDate = ref('');

const fxPairs = ref<string[]>([]);
const selectedFxPair = ref('');
const fxQuotes = ref<FxQuote[]>([]);
const fxSpot = ref('');
const fxDomesticRate = ref('0');
const fxForeignRate = ref('0');

const calibrationResult = ref<CalibrationResult | null>(null);
const isCalibrating = ref(false);

// Popover state
const popoverCell = ref<{ expiry: string; tenor: string } | null>(null);
const popoverPosition = ref<{ top: number; left: number }>({ top: 0, left: 0 });

// Matrix computed properties
const instrumentMap = computed(() => {
  const map = new Map<string, SwaptionInstrument>();
  for (const inst of swaptionInstruments.value) {
    map.set(`${inst.expiry}|${inst.tenor}`, inst);
  }
  return map;
});

function sortByOrder(labels: string[], order: string[]): string[] {
  return [...labels].sort((a, b) => {
    const idxA = order.indexOf(a);
    const idxB = order.indexOf(b);
    return (idxA === -1 ? 999 : idxA) - (idxB === -1 ? 999 : idxB);
  });
}

const matrixExpiries = computed(() => {
  const expiries = [...new Set(swaptionInstruments.value.map(i => i.expiry))];
  return sortByOrder(expiries, EXPIRY_ORDER);
});

const matrixTenors = computed(() => {
  const tenors = [...new Set(swaptionInstruments.value.map(i => i.tenor))];
  return sortByOrder(tenors, TENOR_ORDER);
});

const volRange = computed(() => {
  const vols = swaptionInstruments.value.map(i => i.atmVol);
  if (vols.length === 0) return { min: 0, max: 1 };
  return { min: Math.min(...vols), max: Math.max(...vols) };
});

function getCell(expiry: string, tenor: string): SwaptionInstrument | undefined {
  return instrumentMap.value.get(`${expiry}|${tenor}`);
}

const popoverInstrument = computed(() => {
  if (!popoverCell.value) return null;
  return getCell(popoverCell.value.expiry, popoverCell.value.tenor) ?? null;
});

// Heatmap colour functions
function heatmapColour(vol: number): string {
  const { min, max } = volRange.value;
  if (max === min) return 'rgba(99, 102, 241, 0.15)';
  const t = Math.max(0, Math.min(1, (vol - min) / (max - min)));
  const hue = 220 - t * 205;
  const saturation = 60 + t * 20;
  const lightness = 45 + (1 - Math.abs(t - 0.5) * 2) * 10;
  return `hsla(${hue}, ${saturation}%, ${lightness}%, 0.25)`;
}

function heatmapTextColour(vol: number): string {
  const { min, max } = volRange.value;
  if (max === min) return 'var(--text-primary)';
  const t = Math.max(0, Math.min(1, (vol - min) / (max - min)));
  if (t > 0.75) return '#f97316';
  if (t > 0.5) return '#22c55e';
  if (t > 0.25) return '#3b82f6';
  return 'var(--text-secondary)';
}

// Summary stats
const summaryStats = computed(() => {
  if (activeTab.value === 'swaption') {
    const instruments = swaptionInstruments.value;
    return [
      { label: 'Valuation Date', value: referenceDate.value || '-', icon: 'fa-calendar', color: '#8b5cf6' },
      { label: 'Instruments', value: instruments.length, icon: 'fa-th', color: '#3b82f6' },
      { label: 'Matrix', value: instruments.length > 0 ? `${matrixExpiries.value.length} x ${matrixTenors.value.length}` : '-', icon: 'fa-border-all', color: '#10b981' },
      { label: 'Status', value: calibrationResult.value ? 'Calibrated' : 'Pending', icon: 'fa-info-circle', color: calibrationResult.value ? '#10b981' : '#f59e0b' },
    ];
  }
  return [
    { label: 'Quotes', value: fxQuotes.value.length, icon: 'fa-list', color: '#3b82f6' },
    { label: 'Selected Pair', value: selectedFxPair.value || '-', icon: 'fa-exchange-alt', color: '#10b981' },
    { label: 'Spot Rate', value: fxSpot.value || '-', icon: 'fa-dollar-sign', color: '#8b5cf6' },
    { label: 'Status', value: calibrationResult.value ? 'Calibrated' : 'Pending', icon: 'fa-info-circle', color: calibrationResult.value ? '#10b981' : '#f59e0b' },
  ];
});

// Utility functions
function formatVol(vol: number): string {
  return `${(vol * 100).toFixed(2)}%`;
}

function expiryToLabel(expiry: number): string {
  if (expiry < 0.05) return '1W';
  if (expiry < 0.125) return '1M';
  if (expiry < 0.33) return '3M';
  if (expiry < 0.54) return '6M';
  if (expiry < 1.5) return '1Y';
  if (expiry < 2.5) return '2Y';
  return `${Math.round(expiry)}Y`;
}

// Popover functions
function togglePopover(event: MouseEvent, expiry: string, tenor: string) {
  const cell = getCell(expiry, tenor);
  if (!cell || !cell.smile || cell.smile.length === 0) return;

  if (popoverCell.value?.expiry === expiry && popoverCell.value?.tenor === tenor) {
    popoverCell.value = null;
    return;
  }

  const target = event.currentTarget as HTMLElement;
  const container = target.closest('.matrix-container') as HTMLElement;
  if (!container) return;

  const targetRect = target.getBoundingClientRect();
  const containerRect = container.getBoundingClientRect();

  popoverPosition.value = {
    top: targetRect.bottom - containerRect.top + 4,
    left: targetRect.left - containerRect.left + targetRect.width / 2,
  };

  popoverCell.value = { expiry, tenor };
}

function closePopover() {
  popoverCell.value = null;
}

function onDocumentClick(event: MouseEvent) {
  const target = event.target as HTMLElement;
  if (!target.closest('.popover-trigger') && !target.closest('.smile-popover')) {
    popoverCell.value = null;
  }
}

// API calls
async function loadSwaptionIndices() {
  try {
    const response = await fetch('/api/volcube/indices');
    if (!response.ok) throw new Error('Failed to load indices');
    const data = await response.json();
    swaptionIndices.value = data.indices || [];
    const usdIndex = swaptionIndices.value.find(idx => idx.startsWith('USD'));
    if (usdIndex && !selectedSwaptionIndex.value) {
      selectedSwaptionIndex.value = usdIndex;
    }
  } catch (error) {
    console.error('Failed to load swaption indices:', error);
  }
}

async function loadSwaptionModels() {
  try {
    const response = await fetch('/api/volcube/models');
    if (!response.ok) throw new Error('Failed to load models');
    const data = await response.json();
    swaptionModels.value = data.models || [];
    if (swaptionModels.value.length > 0) {
      selectedModel.value = swaptionModels.value[0];
    }
  } catch (error) {
    console.error('Failed to load calibration models:', error);
  }
}

async function loadSwaptionInstruments(index: string) {
  try {
    const response = await fetch(`/api/volcube/instruments/${index}`);
    if (!response.ok) throw new Error('Failed to load instruments');
    const data = await response.json();
    swaptionInstruments.value = data.instruments || [];
    referenceDate.value = data.referenceDate || data.reference_date || data.metadata?.lastUpdated?.split('T')[0] || '';
    calibrationResult.value = null;
    popoverCell.value = null;
  } catch (error) {
    console.error('Failed to load instruments:', error);
  }
}

async function loadFxPairs() {
  try {
    const response = await fetch('/api/fxvol/pairs');
    if (!response.ok) throw new Error('Failed to load FX pairs');
    const data = await response.json();
    fxPairs.value = (data.pairs || []).map((p: { pair: string }) => p.pair);
  } catch (error) {
    console.error('Failed to load FX pairs:', error);
  }
}

async function loadFxQuotes(pair: string) {
  try {
    const response = await fetch(`/api/fxvol/quotes/${pair}`);
    if (!response.ok) throw new Error('Failed to load FX quotes');
    const data = await response.json();
    fxQuotes.value = data.quotes || [];
    if (data.spot) {
      fxSpot.value = data.spot.toFixed(4);
    }
    calibrationResult.value = null;
  } catch (error) {
    console.error('Failed to load FX quotes:', error);
  }
}

async function calibrate() {
  if (activeTab.value === 'swaption' && !selectedSwaptionIndex.value) return;
  if (activeTab.value === 'fx' && !selectedFxPair.value) return;

  isCalibrating.value = true;
  try {
    const endpoint = activeTab.value === 'swaption' ? '/api/volcube/calibrate' : '/api/fxvol/calibrate';

    const body = activeTab.value === 'swaption'
      ? {
          index: selectedSwaptionIndex.value,
          referenceDate: referenceDate.value,
          model: selectedModel.value,
        }
      : {
          pair: selectedFxPair.value,
          spot: parseFloat(fxSpot.value || '0'),
          domesticRate: parseFloat(fxDomesticRate.value || '0') / 100,
          foreignRate: parseFloat(fxForeignRate.value || '0') / 100,
        };

    const response = await fetch(endpoint, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.message || 'Calibration failed');
    }

    calibrationResult.value = await response.json();
  } catch (error) {
    console.error('Calibration failed:', error);
  } finally {
    isCalibrating.value = false;
  }
}

function exportCsv() {
  if (!calibrationResult.value) return;

  const csv = [
    'Parameter,Value',
    ...Object.entries(calibrationResult.value.parameters).map(
      ([key, value]) => `${key},${value}`
    ),
  ].join('\n');

  downloadFile(csv, 'volcube_calibration.csv', 'text/csv');
}

function exportJson() {
  if (!calibrationResult.value) return;

  const json = JSON.stringify(calibrationResult.value, null, 2);
  downloadFile(json, 'volcube_calibration.json', 'application/json');
}

function downloadFile(content: string, filename: string, mimeType: string) {
  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

// Watch for selection changes
watch(selectedSwaptionIndex, (index) => {
  if (index) loadSwaptionInstruments(index);
});

watch(selectedFxPair, (pair) => {
  if (pair) loadFxQuotes(pair);
});

// Lifecycle
onMounted(() => {
  document.addEventListener('click', onDocumentClick);
});

onUnmounted(() => {
  document.removeEventListener('click', onDocumentClick);
});

// Initialize
loadSwaptionIndices();
loadSwaptionModels();
loadFxPairs();
</script>

<template>
  <div class="volcube-builder-view">
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

    <!-- Asset Tabs -->
    <div class="flex gap-2 mb-6">
      <button
        :class="[
          'px-4 py-2 rounded-lg font-medium transition-all duration-200 flex items-center gap-2',
          activeTab === 'swaption'
            ? 'bg-[var(--primary)] text-white'
            : 'bg-[var(--surface)] text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]'
        ]"
        @click="activeTab = 'swaption'"
      >
        <i class="fas fa-percentage"></i>
        Swaption
      </button>
      <button
        :class="[
          'px-4 py-2 rounded-lg font-medium transition-all duration-200 flex items-center gap-2',
          activeTab === 'fx'
            ? 'bg-[var(--primary)] text-white'
            : 'bg-[var(--surface)] text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]'
        ]"
        @click="activeTab = 'fx'"
      >
        <i class="fas fa-exchange-alt"></i>
        FX
      </button>
    </div>

    <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <!-- Left Panel: Settings -->
      <div class="space-y-4">
        <!-- Swaption Settings -->
        <template v-if="activeTab === 'swaption'">
          <div class="glass-card p-5">
            <h3 class="text-base font-semibold text-[var(--text-primary)] mb-3">Index Selection</h3>
            <select
              v-model="selectedSwaptionIndex"
              class="w-full px-3 py-2 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-[var(--primary)]"
            >
              <option value="">Select index...</option>
              <option v-for="idx in swaptionIndices" :key="idx" :value="idx">{{ idx }}</option>
            </select>
          </div>

          <div class="glass-card p-5">
            <h3 class="text-base font-semibold text-[var(--text-primary)] mb-3">Calibration Settings</h3>
            <div class="space-y-3">
              <div>
                <label class="block text-xs text-[var(--text-muted)] mb-1">Model</label>
                <select
                  v-model="selectedModel"
                  class="w-full px-2 py-1.5 rounded bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm"
                >
                  <option v-for="model in swaptionModels" :key="model" :value="model">{{ model }}</option>
                </select>
              </div>
            </div>
          </div>
        </template>

        <!-- FX Settings -->
        <template v-else>
          <div class="glass-card p-5">
            <h3 class="text-base font-semibold text-[var(--text-primary)] mb-3">Currency Pair</h3>
            <select
              v-model="selectedFxPair"
              class="w-full px-3 py-2 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-[var(--primary)]"
            >
              <option value="">Select pair...</option>
              <option v-for="pair in fxPairs" :key="pair" :value="pair">{{ pair }}</option>
            </select>
          </div>

          <div class="glass-card p-5">
            <h3 class="text-base font-semibold text-[var(--text-primary)] mb-3">Market Data</h3>
            <div class="space-y-3">
              <div>
                <label class="block text-xs text-[var(--text-muted)] mb-1">Spot Rate</label>
                <input
                  v-model="fxSpot"
                  type="number"
                  step="0.0001"
                  class="w-full px-2 py-1.5 rounded bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm"
                >
              </div>
              <div>
                <label class="block text-xs text-[var(--text-muted)] mb-1">Domestic Rate (%)</label>
                <input
                  v-model="fxDomesticRate"
                  type="number"
                  step="0.01"
                  class="w-full px-2 py-1.5 rounded bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm"
                >
              </div>
              <div>
                <label class="block text-xs text-[var(--text-muted)] mb-1">Foreign Rate (%)</label>
                <input
                  v-model="fxForeignRate"
                  type="number"
                  step="0.01"
                  class="w-full px-2 py-1.5 rounded bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm"
                >
              </div>
            </div>
          </div>
        </template>

        <!-- Actions -->
        <div class="glass-card p-5">
          <button
            :disabled="(activeTab === 'swaption' && !selectedSwaptionIndex) || (activeTab === 'fx' && !selectedFxPair) || isCalibrating"
            class="w-full px-4 py-2.5 rounded-lg bg-[var(--primary)] text-white font-medium hover:opacity-90 disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
            @click="calibrate"
          >
            <i :class="['fas', isCalibrating ? 'fa-spinner fa-spin' : 'fa-cogs']"></i>
            {{ isCalibrating ? 'Calibrating...' : 'Calibrate' }}
          </button>
          <div class="grid grid-cols-2 gap-2 mt-2">
            <button
              :disabled="!calibrationResult"
              class="px-3 py-1.5 rounded bg-[var(--surface)] text-[var(--text-secondary)] text-sm hover:bg-[var(--surface-hover)] disabled:opacity-50"
              @click="exportCsv"
            >
              <i class="fas fa-file-csv mr-1"></i>CSV
            </button>
            <button
              :disabled="!calibrationResult"
              class="px-3 py-1.5 rounded bg-[var(--surface)] text-[var(--text-secondary)] text-sm hover:bg-[var(--surface-hover)] disabled:opacity-50"
              @click="exportJson"
            >
              <i class="fas fa-file-code mr-1"></i>JSON
            </button>
          </div>
        </div>

        <!-- Calibration Result -->
        <div v-if="calibrationResult" class="glass-card p-5">
          <h3 class="text-base font-semibold text-[var(--text-primary)] mb-3 flex items-center gap-2">
            <i class="fas fa-check-circle text-[var(--success)]"></i>
            Calibration Result
          </h3>
          <div class="space-y-2 mb-4">
            <div class="flex justify-between text-sm">
              <span class="text-[var(--text-muted)]">Model:</span>
              <span class="text-[var(--text-primary)] font-medium">{{ calibrationResult.model }}</span>
            </div>
            <div class="flex justify-between text-sm">
              <span class="text-[var(--text-muted)]">Instruments:</span>
              <span class="text-[var(--text-primary)] font-medium">{{ calibrationResult.metadata.instrumentCount }}</span>
            </div>
            <div class="flex justify-between text-sm">
              <span class="text-[var(--text-muted)]">Processing Time:</span>
              <span class="text-[var(--text-primary)] font-medium">{{ calibrationResult.metadata.processingTimeMs.toFixed(2) }} ms</span>
            </div>
          </div>
          <h4 class="text-sm font-medium text-[var(--text-primary)] mb-2">Parameters</h4>
          <div class="space-y-1">
            <div
              v-for="(value, key) in calibrationResult.parameters"
              :key="key"
              class="flex justify-between text-sm"
            >
              <span class="text-[var(--text-muted)]">{{ key }}:</span>
              <span class="text-[var(--text-primary)] font-mono">{{ Number(value).toFixed(6) }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Right Panel: Data Table -->
      <div class="lg:col-span-2">
        <div class="glass-card p-6">
          <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">
            {{ activeTab === 'swaption' ? 'Swaption Instruments' : 'FX Quotes' }}
          </h3>

          <!-- Swaption Matrix -->
          <template v-if="activeTab === 'swaption'">
            <!-- Empty State -->
            <div v-if="swaptionInstruments.length === 0" class="text-center py-12">
              <i class="fas fa-cube text-4xl text-[var(--text-muted)] mb-4"></i>
              <p class="text-[var(--text-muted)]">Select an index to load instruments</p>
            </div>

            <!-- Matrix / Heatmap -->
            <div v-else class="matrix-container relative overflow-x-auto">
              <table class="w-full border-collapse">
                <thead>
                  <tr>
                    <th class="sticky left-0 z-10 py-2 px-3 text-xs font-medium text-[var(--text-muted)] bg-[var(--glass-bg)] border-b border-r border-[var(--glass-border)]">
                      Expiry \ Tenor
                    </th>
                    <th
                      v-for="tenor in matrixTenors"
                      :key="tenor"
                      class="py-2 px-3 text-xs font-medium text-[var(--text-muted)] text-center border-b border-[var(--glass-border)] min-w-[80px]"
                    >
                      {{ tenor }}
                    </th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="expiry in matrixExpiries" :key="expiry">
                    <td class="sticky left-0 z-10 py-2 px-3 text-xs font-medium text-[var(--text-muted)] bg-[var(--glass-bg)] border-r border-b border-[var(--glass-border)]">
                      {{ expiry }}
                    </td>
                    <td
                      v-for="tenor in matrixTenors"
                      :key="tenor"
                      class="py-2 px-2 text-center border-b border-[var(--glass-border)] transition-all duration-150 popover-trigger"
                      :class="[
                        getCell(expiry, tenor) ? 'cursor-pointer hover-cell' : '',
                        popoverCell?.expiry === expiry && popoverCell?.tenor === tenor ? 'ring-2 ring-[var(--primary)] ring-inset' : ''
                      ]"
                      :style="getCell(expiry, tenor)
                        ? { backgroundColor: heatmapColour(getCell(expiry, tenor)!.atmVol) }
                        : {}"
                      @click="getCell(expiry, tenor) ? togglePopover($event, expiry, tenor) : undefined"
                    >
                      <template v-if="getCell(expiry, tenor)">
                        <span
                          class="text-xs font-mono font-medium"
                          :style="{ color: heatmapTextColour(getCell(expiry, tenor)!.atmVol) }"
                        >
                          {{ formatVol(getCell(expiry, tenor)!.atmVol) }}
                        </span>
                      </template>
                      <span v-else class="text-xs text-[var(--text-muted)]">--</span>
                    </td>
                  </tr>
                </tbody>
              </table>

              <!-- Smile Popover -->
              <div
                v-if="popoverInstrument"
                class="smile-popover absolute z-50 w-64 glass-card p-4 shadow-lg"
                :style="{
                  top: `${popoverPosition.top}px`,
                  left: `${popoverPosition.left}px`,
                  transform: 'translateX(-50%)',
                }"
              >
                <div class="flex items-center justify-between mb-3">
                  <h4 class="text-sm font-semibold text-[var(--text-primary)]">
                    {{ popoverInstrument.expiry }} x {{ popoverInstrument.tenor }}
                  </h4>
                  <button
                    class="text-[var(--text-muted)] hover:text-[var(--text-primary)] text-xs"
                    @click="closePopover"
                  >
                    <i class="fas fa-times"></i>
                  </button>
                </div>

                <div class="text-xs space-y-1 mb-3">
                  <div class="flex justify-between">
                    <span class="text-[var(--text-muted)]">ATM Vol:</span>
                    <span class="text-[var(--text-primary)] font-mono">{{ formatVol(popoverInstrument.atmVol) }}</span>
                  </div>
                </div>

                <h5 class="text-xs font-medium text-[var(--text-muted)] mb-2">Smile</h5>
                <table class="w-full text-xs">
                  <thead>
                    <tr class="border-b border-[var(--glass-border)]">
                      <th class="text-left py-1 text-[var(--text-muted)]">Offset (bp)</th>
                      <th class="text-right py-1 text-[var(--text-muted)]">Vol</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr
                      v-for="pt in popoverInstrument.smile"
                      :key="pt.strikeOffsetBp"
                      class="border-b border-[var(--glass-border)]"
                    >
                      <td class="py-1 text-[var(--text-secondary)]">
                        {{ pt.strikeOffsetBp > 0 ? '+' : '' }}{{ pt.strikeOffsetBp }}
                      </td>
                      <td class="py-1 text-right font-mono text-[var(--text-primary)]">
                        {{ formatVol(pt.vol) }}
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>
          </template>

          <!-- FX Table -->
          <template v-else>
            <div v-if="fxQuotes.length === 0" class="text-center py-12">
              <i class="fas fa-exchange-alt text-4xl text-[var(--text-muted)] mb-4"></i>
              <p class="text-[var(--text-muted)]">Select a pair to load quotes</p>
            </div>
            <div v-else class="overflow-x-auto">
              <table class="w-full">
                <thead>
                  <tr class="border-b border-[var(--glass-border)]">
                    <th class="text-left py-3 px-4 text-sm font-medium text-[var(--text-muted)]">Expiry</th>
                    <th class="text-right py-3 px-4 text-sm font-medium text-[var(--text-muted)]">ATM Vol</th>
                    <th class="text-right py-3 px-4 text-sm font-medium text-[var(--text-muted)]">25D RR</th>
                    <th class="text-right py-3 px-4 text-sm font-medium text-[var(--text-muted)]">25D BF</th>
                  </tr>
                </thead>
                <tbody>
                  <tr
                    v-for="(quote, idx) in fxQuotes"
                    :key="idx"
                    class="border-b border-[var(--glass-border)] hover:bg-[var(--surface-hover)] transition-colors"
                  >
                    <td class="py-3 px-4 text-sm text-[var(--text-primary)]">{{ expiryToLabel(quote.expiry) }}</td>
                    <td class="py-3 px-4 text-sm text-right text-[var(--text-primary)] font-mono">{{ formatVol(quote.atmVol) }}</td>
                    <td class="py-3 px-4 text-sm text-right text-[var(--text-secondary)] font-mono">{{ (quote.rr25d * 10000).toFixed(1) }} bps</td>
                    <td class="py-3 px-4 text-sm text-right text-[var(--text-secondary)] font-mono">{{ (quote.bf25d * 10000).toFixed(1) }} bps</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </template>
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

.matrix-container {
  position: relative;
}

.hover-cell:hover {
  filter: brightness(1.3);
}

.smile-popover::before {
  content: '';
  position: absolute;
  top: -6px;
  left: 50%;
  transform: translateX(-50%);
  border-left: 6px solid transparent;
  border-right: 6px solid transparent;
  border-bottom: 6px solid var(--glass-border);
}
</style>
