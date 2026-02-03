<script setup lang="ts">
import { ref, computed, watch } from 'vue';

// Types
interface CurveInstrument {
  id: string;
  type: string;
  tenor: string;
  tenorYears: number;
  rate: number;
  enabled: boolean;
}

interface BuildResult {
  curve_id?: string;
  instrument_count?: number;
  interpolation?: string;
  calculation_time_ms?: number;
  pillars?: Array<{ time: number; discount_factor: number; zero_rate: number }>;
  converged?: boolean;
}

// State
const indices = ref<string[]>([]);
const selectedIndex = ref<string>('');
const instruments = ref<CurveInstrument[]>([]);
const originalInstruments = ref<CurveInstrument[]>([]);
const buildResult = ref<BuildResult | null>(null);
const isBuilding = ref(false);
const enableJumps = ref(false);

// Computed
const hasChanges = computed(() =>
  JSON.stringify(instruments.value) !== JSON.stringify(originalInstruments.value)
);

const enabledInstruments = computed(() =>
  instruments.value.filter(inst => inst.enabled)
);

const summaryStats = computed(() => [
  { label: 'Total Instruments', value: instruments.value.length, icon: 'fa-list-alt', color: '#3b82f6' },
  { label: 'Enabled', value: enabledInstruments.value.length, icon: 'fa-check-circle', color: '#10b981' },
  { label: 'Avg Rate', value: enabledInstruments.value.length > 0
      ? `${(enabledInstruments.value.reduce((sum, i) => sum + i.rate, 0) / enabledInstruments.value.length * 100).toFixed(2)}%`
      : '-', icon: 'fa-percent', color: '#8b5cf6' },
  { label: 'Status', value: buildResult.value ? 'Built' : 'Pending', icon: 'fa-info-circle', color: buildResult.value ? '#10b981' : '#f59e0b' },
]);

// API calls
async function loadIndices() {
  try {
    const response = await fetch('/api/curves/indices');
    if (!response.ok) throw new Error('Failed to load indices');
    const data = await response.json();
    indices.value = data.indices || [];
  } catch (error) {
    console.error('Failed to load indices:', error);
  }
}

async function loadInstruments(index: string) {
  try {
    const response = await fetch(`/api/curves/instruments/${index}`);
    if (!response.ok) throw new Error('Failed to load instruments');
    const data = await response.json();
    instruments.value = data.instruments || [];
    originalInstruments.value = JSON.parse(JSON.stringify(instruments.value));
    buildResult.value = null;
  } catch (error) {
    console.error('Failed to load instruments:', error);
  }
}

async function buildCurve() {
  if (!selectedIndex.value || enabledInstruments.value.length === 0) return;

  isBuilding.value = true;
  try {
    const response = await fetch('/api/curves/build', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        index: selectedIndex.value,
        instruments: enabledInstruments.value.map(inst => ({
          type: inst.type,
          tenor: inst.tenor,
          rate: inst.rate,
        })),
        enableJumps: enableJumps.value,
      }),
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.message || 'Build failed');
    }

    buildResult.value = await response.json();
    originalInstruments.value = JSON.parse(JSON.stringify(instruments.value));
  } catch (error) {
    console.error('Build failed:', error);
  } finally {
    isBuilding.value = false;
  }
}

function resetRates() {
  instruments.value = JSON.parse(JSON.stringify(originalInstruments.value));
}

function exportRates() {
  if (instruments.value.length === 0) return;

  const csv = [
    'Type,Tenor,Rate,Enabled',
    ...instruments.value.map(
      inst => `${inst.type},${inst.tenor},${(inst.rate * 100).toFixed(4)},${inst.enabled}`
    ),
  ].join('\n');

  const blob = new Blob([csv], { type: 'text/csv' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = `curve_instruments_${selectedIndex.value || 'unknown'}.csv`;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

function updateRate(index: number, value: string) {
  instruments.value[index].rate = parseFloat(value) / 100;
}

function toggleEnabled(index: number) {
  instruments.value[index].enabled = !instruments.value[index].enabled;
}

function toggleAll(enabled: boolean) {
  instruments.value.forEach(inst => inst.enabled = enabled);
}

// Watch for index selection change
watch(selectedIndex, (newIndex) => {
  if (newIndex) {
    loadInstruments(newIndex);
  }
});

// Initialize
loadIndices();
</script>

<template>
  <div class="curve-builder-view">
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
            <p class="text-2xl font-semibold text-[var(--text-primary)]">{{ stat.value }}</p>
          </div>
          <div
            class="w-10 h-10 rounded-lg flex items-center justify-center"
            :style="{ backgroundColor: `${stat.color}1a` }"
          >
            <i :class="['fas', stat.icon]" :style="{ color: stat.color }"></i>
          </div>
        </div>
      </div>
    </div>

    <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <!-- Left Panel: Index Selection & Settings -->
      <div class="space-y-6">
        <!-- Index Selector -->
        <div class="glass-card p-6">
          <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Index Selection</h3>
          <select
            v-model="selectedIndex"
            class="w-full px-4 py-2.5 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--primary)]"
          >
            <option value="">Select index...</option>
            <option v-for="idx in indices" :key="idx" :value="idx">{{ idx }}</option>
          </select>
        </div>

        <!-- Build Settings -->
        <div class="glass-card p-6">
          <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Build Settings</h3>
          <label class="flex items-center gap-3 cursor-pointer">
            <input
              v-model="enableJumps"
              type="checkbox"
              class="w-5 h-5 rounded border-[var(--glass-border)] bg-[var(--surface)] text-[var(--primary)] focus:ring-[var(--primary)]"
            >
            <span class="text-sm text-[var(--text-secondary)]">Enable CB Event Jumps</span>
          </label>
        </div>

        <!-- Actions -->
        <div class="glass-card p-6">
          <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Actions</h3>
          <div class="space-y-3">
            <button
              :disabled="!selectedIndex || enabledInstruments.length === 0 || isBuilding"
              class="w-full px-4 py-2.5 rounded-lg bg-[var(--primary)] text-white font-medium transition-all duration-200 hover:opacity-90 disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
              @click="buildCurve"
            >
              <i :class="['fas', isBuilding ? 'fa-spinner fa-spin' : 'fa-hammer']"></i>
              {{ isBuilding ? 'Building...' : 'Build Curve' }}
            </button>
            <div class="grid grid-cols-2 gap-3">
              <button
                :disabled="!hasChanges"
                class="px-4 py-2 rounded-lg bg-[var(--surface)] text-[var(--text-secondary)] font-medium transition-all duration-200 hover:bg-[var(--surface-hover)] disabled:opacity-50 disabled:cursor-not-allowed"
                @click="resetRates"
              >
                <i class="fas fa-undo mr-2"></i>Reset
              </button>
              <button
                :disabled="instruments.length === 0"
                class="px-4 py-2 rounded-lg bg-[var(--surface)] text-[var(--text-secondary)] font-medium transition-all duration-200 hover:bg-[var(--surface-hover)] disabled:opacity-50 disabled:cursor-not-allowed"
                @click="exportRates"
              >
                <i class="fas fa-download mr-2"></i>Export
              </button>
            </div>
          </div>

          <!-- Changes Indicator -->
          <div v-if="hasChanges" class="mt-4 p-3 rounded-lg bg-[#f59e0b1a] border border-[var(--warning)]">
            <p class="text-sm text-[var(--warning)] flex items-center gap-2">
              <i class="fas fa-exclamation-triangle"></i>
              Unsaved changes - rebuild required
            </p>
          </div>
        </div>

        <!-- Build Result -->
        <div v-if="buildResult" class="glass-card p-6">
          <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4 flex items-center gap-2">
            <i class="fas fa-check-circle text-[var(--success)]"></i>
            Build Complete
          </h3>
          <div class="space-y-2">
            <div class="flex justify-between text-sm">
              <span class="text-[var(--text-muted)]">Instruments:</span>
              <span class="text-[var(--text-primary)] font-medium">{{ buildResult.instrument_count }}</span>
            </div>
            <div class="flex justify-between text-sm">
              <span class="text-[var(--text-muted)]">Interpolation:</span>
              <span class="text-[var(--text-primary)] font-medium">{{ buildResult.interpolation }}</span>
            </div>
            <div class="flex justify-between text-sm">
              <span class="text-[var(--text-muted)]">Processing Time:</span>
              <span class="text-[var(--text-primary)] font-medium">{{ buildResult.calculation_time_ms?.toFixed(2) }} ms</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Right Panel: Instruments Table -->
      <div class="lg:col-span-2">
        <div class="glass-card p-6">
          <div class="flex items-center justify-between mb-4">
            <h3 class="text-lg font-semibold text-[var(--text-primary)]">Market Instruments</h3>
            <div v-if="instruments.length > 0" class="flex gap-2">
              <button
                class="px-3 py-1.5 text-xs rounded-lg bg-[var(--surface)] text-[var(--text-secondary)] hover:bg-[var(--surface-hover)] transition-colors"
                @click="toggleAll(true)"
              >
                Enable All
              </button>
              <button
                class="px-3 py-1.5 text-xs rounded-lg bg-[var(--surface)] text-[var(--text-secondary)] hover:bg-[var(--surface-hover)] transition-colors"
                @click="toggleAll(false)"
              >
                Disable All
              </button>
            </div>
          </div>

          <!-- Empty State -->
          <div v-if="instruments.length === 0" class="text-center py-12">
            <i class="fas fa-chart-line text-4xl text-[var(--text-muted)] mb-4"></i>
            <p class="text-[var(--text-muted)]">Select an index to load instruments</p>
          </div>

          <!-- Instruments Table -->
          <div v-else class="overflow-x-auto">
            <table class="w-full">
              <thead>
                <tr class="border-b border-[var(--glass-border)]">
                  <th class="text-left py-3 px-4 text-sm font-medium text-[var(--text-muted)]">Type</th>
                  <th class="text-left py-3 px-4 text-sm font-medium text-[var(--text-muted)]">Tenor</th>
                  <th class="text-right py-3 px-4 text-sm font-medium text-[var(--text-muted)]">Rate (%)</th>
                  <th class="text-center py-3 px-4 text-sm font-medium text-[var(--text-muted)]">Enabled</th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="(inst, idx) in instruments"
                  :key="inst.id"
                  :class="[
                    'border-b border-[var(--glass-border)] transition-colors',
                    inst.enabled ? 'hover:bg-[var(--surface-hover)]' : 'opacity-50'
                  ]"
                >
                  <td class="py-3 px-4 text-sm text-[var(--text-primary)]">{{ inst.type }}</td>
                  <td class="py-3 px-4 text-sm text-[var(--text-secondary)]">{{ inst.tenor }}</td>
                  <td class="py-3 px-4">
                    <input
                      type="number"
                      :value="(inst.rate * 100).toFixed(4)"
                      step="0.0001"
                      class="w-24 px-2 py-1 text-right text-sm rounded bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] focus:outline-none focus:ring-1 focus:ring-[var(--primary)]"
                      @change="updateRate(idx, ($event.target as HTMLInputElement).value)"
                    >
                  </td>
                  <td class="py-3 px-4 text-center">
                    <input
                      type="checkbox"
                      :checked="inst.enabled"
                      class="w-4 h-4 rounded border-[var(--glass-border)] bg-[var(--surface)] text-[var(--primary)] focus:ring-[var(--primary)]"
                      @change="toggleEnabled(idx)"
                    >
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
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
