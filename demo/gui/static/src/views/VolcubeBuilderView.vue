<script setup lang="ts">
import { ref, computed, watch } from 'vue';

// Types
interface SwaptionInstrument {
  expiry: string;
  tenor: string;
  atmVol: number;
  smile?: Array<{ strikeOffsetBp: number; vol: number }>;
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

// Computed
const summaryStats = computed(() => {
  if (activeTab.value === 'swaption') {
    return [
      { label: 'Valuation Date', value: referenceDate.value || '-', icon: 'fa-calendar', color: '#8b5cf6' },
      { label: 'Instruments', value: swaptionInstruments.value.length, icon: 'fa-list', color: '#3b82f6' },
      { label: 'Model', value: selectedModel.value || '-', icon: 'fa-cogs', color: '#8b5cf6' },
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

// API calls
async function loadSwaptionIndices() {
  try {
    const response = await fetch('/api/volcube/indices');
    if (!response.ok) throw new Error('Failed to load indices');
    const data = await response.json();
    swaptionIndices.value = data.indices || [];
    // Default to first USD index
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
    // Extract reference date from API response
    referenceDate.value = data.referenceDate || data.reference_date || data.metadata?.lastUpdated?.split('T')[0] || '';
    calibrationResult.value = null;
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

// Initialize
loadSwaptionIndices();
loadSwaptionModels();
loadFxPairs();
</script>

<template>
  <div class="volcube-builder-view">
    <!-- Summary Stats -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
      <div
        v-for="stat in summaryStats"
        :key="stat.label"
        class="glass-card p-4"
      >
        <div class="flex items-start justify-between">
          <div>
            <p class="text-sm text-[var(--text-muted)] mb-1">{{ stat.label }}</p>
            <p class="text-2xl font-semibold text-[var(--text-primary)] truncate">{{ stat.value }}</p>
          </div>
          <div
            class="w-10 h-10 rounded-lg flex items-center justify-center flex-shrink-0"
            :style="{ backgroundColor: `${stat.color}1a` }"
          >
            <i :class="['fas', stat.icon]" :style="{ color: stat.color }"></i>
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
      <div class="space-y-6">
        <!-- Swaption Settings -->
        <template v-if="activeTab === 'swaption'">
          <div class="glass-card p-6">
            <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Index Selection</h3>
            <select
              v-model="selectedSwaptionIndex"
              class="w-full px-4 py-2.5 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--primary)]"
            >
              <option value="">Select index...</option>
              <option v-for="idx in swaptionIndices" :key="idx" :value="idx">{{ idx }}</option>
            </select>
          </div>

          <div class="glass-card p-6">
            <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Calibration Settings</h3>
            <div class="space-y-4">
              <div>
                <label class="block text-sm text-[var(--text-muted)] mb-2">Model</label>
                <select
                  v-model="selectedModel"
                  class="w-full px-4 py-2.5 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--primary)]"
                >
                  <option v-for="model in swaptionModels" :key="model" :value="model">{{ model }}</option>
                </select>
              </div>
            </div>
          </div>
        </template>

        <!-- FX Settings -->
        <template v-else>
          <div class="glass-card p-6">
            <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Currency Pair</h3>
            <select
              v-model="selectedFxPair"
              class="w-full px-4 py-2.5 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--primary)]"
            >
              <option value="">Select pair...</option>
              <option v-for="pair in fxPairs" :key="pair" :value="pair">{{ pair }}</option>
            </select>
          </div>

          <div class="glass-card p-6">
            <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Market Data</h3>
            <div class="space-y-4">
              <div>
                <label class="block text-sm text-[var(--text-muted)] mb-2">Spot Rate</label>
                <input
                  v-model="fxSpot"
                  type="number"
                  step="0.0001"
                  class="w-full px-4 py-2.5 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--primary)]"
                >
              </div>
              <div>
                <label class="block text-sm text-[var(--text-muted)] mb-2">Domestic Rate (%)</label>
                <input
                  v-model="fxDomesticRate"
                  type="number"
                  step="0.01"
                  class="w-full px-4 py-2.5 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--primary)]"
                >
              </div>
              <div>
                <label class="block text-sm text-[var(--text-muted)] mb-2">Foreign Rate (%)</label>
                <input
                  v-model="fxForeignRate"
                  type="number"
                  step="0.01"
                  class="w-full px-4 py-2.5 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--primary)]"
                >
              </div>
            </div>
          </div>
        </template>

        <!-- Actions -->
        <div class="glass-card p-6">
          <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Actions</h3>
          <div class="space-y-3">
            <button
              :disabled="(activeTab === 'swaption' && !selectedSwaptionIndex) || (activeTab === 'fx' && !selectedFxPair) || isCalibrating"
              class="w-full px-4 py-2.5 rounded-lg bg-[var(--primary)] text-white font-medium transition-all duration-200 hover:opacity-90 disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
              @click="calibrate"
            >
              <i :class="['fas', isCalibrating ? 'fa-spinner fa-spin' : 'fa-cogs']"></i>
              {{ isCalibrating ? 'Calibrating...' : 'Calibrate' }}
            </button>
            <div class="grid grid-cols-2 gap-3">
              <button
                :disabled="!calibrationResult"
                class="px-4 py-2 rounded-lg bg-[var(--surface)] text-[var(--text-secondary)] font-medium transition-all duration-200 hover:bg-[var(--surface-hover)] disabled:opacity-50 disabled:cursor-not-allowed"
                @click="exportCsv"
              >
                <i class="fas fa-file-csv mr-2"></i>CSV
              </button>
              <button
                :disabled="!calibrationResult"
                class="px-4 py-2 rounded-lg bg-[var(--surface)] text-[var(--text-secondary)] font-medium transition-all duration-200 hover:bg-[var(--surface-hover)] disabled:opacity-50 disabled:cursor-not-allowed"
                @click="exportJson"
              >
                <i class="fas fa-file-code mr-2"></i>JSON
              </button>
            </div>
          </div>
        </div>

        <!-- Calibration Result -->
        <div v-if="calibrationResult" class="glass-card p-6">
          <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4 flex items-center gap-2">
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

          <!-- Swaption Table -->
          <template v-if="activeTab === 'swaption'">
            <div v-if="swaptionInstruments.length === 0" class="text-center py-12">
              <i class="fas fa-cube text-4xl text-[var(--text-muted)] mb-4"></i>
              <p class="text-[var(--text-muted)]">Select an index to load instruments</p>
            </div>
            <div v-else class="overflow-x-auto">
              <table class="w-full">
                <thead>
                  <tr class="border-b border-[var(--glass-border)]">
                    <th class="text-left py-3 px-4 text-sm font-medium text-[var(--text-muted)]">Expiry</th>
                    <th class="text-left py-3 px-4 text-sm font-medium text-[var(--text-muted)]">Tenor</th>
                    <th class="text-right py-3 px-4 text-sm font-medium text-[var(--text-muted)]">ATM Vol</th>
                  </tr>
                </thead>
                <tbody>
                  <tr
                    v-for="(inst, idx) in swaptionInstruments"
                    :key="idx"
                    class="border-b border-[var(--glass-border)] hover:bg-[var(--surface-hover)] transition-colors"
                  >
                    <td class="py-3 px-4 text-sm text-[var(--text-primary)]">{{ inst.expiry }}</td>
                    <td class="py-3 px-4 text-sm text-[var(--text-secondary)]">{{ inst.tenor }}</td>
                    <td class="py-3 px-4 text-sm text-right text-[var(--text-primary)] font-mono">{{ formatVol(inst.atmVol) }}</td>
                  </tr>
                </tbody>
              </table>
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
</style>
