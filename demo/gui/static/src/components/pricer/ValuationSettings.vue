<script setup lang="ts">
import { usePricerStore } from '@/stores/pricer';

const store = usePricerStore();

const inputClass =
  'w-full px-4 py-2.5 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--primary)]';
const smallInputClass =
  'w-full px-3 py-2 text-sm rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)]';
</script>

<template>
  <div class="glass-card p-6">
    <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Valuation Settings</h3>
    <div class="space-y-4">
      <!-- Valuation Date -->
      <div>
        <label class="block text-sm text-[var(--text-muted)] mb-2">Valuation Date</label>
        <input type="date" v-model="store.valuationDate" :class="inputClass" />
      </div>

      <!-- Reporting Currency -->
      <div>
        <label class="block text-sm text-[var(--text-muted)] mb-2">Reporting Currency</label>
        <select v-model="store.reportingCcy" :class="inputClass">
          <option value="USD">USD</option>
          <option value="EUR">EUR</option>
          <option value="GBP">GBP</option>
          <option value="JPY">JPY</option>
        </select>
      </div>

      <!-- Use Default Model Config -->
      <label class="flex items-center gap-3 cursor-pointer">
        <input
          type="checkbox"
          v-model="store.useDefaults"
          class="w-5 h-5 rounded border-[var(--glass-border)] bg-[var(--surface)] text-[var(--primary)] focus:ring-[var(--primary)]"
        />
        <span class="text-sm text-[var(--text-secondary)]">Use Default Model Config</span>
      </label>

      <!-- Custom Model Config -->
      <template v-if="!store.useDefaults">
        <div class="grid grid-cols-2 gap-3">
          <div>
            <label class="block text-xs text-[var(--text-muted)] mb-1">Paths</label>
            <input type="number" v-model.number="store.numPaths" :class="smallInputClass" />
          </div>
          <div>
            <label class="block text-xs text-[var(--text-muted)] mb-1">Steps</label>
            <input type="number" v-model.number="store.numSteps" :class="smallInputClass" />
          </div>
        </div>
      </template>

      <!-- Bump Settings -->
      <div class="grid grid-cols-3 gap-3">
        <div>
          <label class="block text-xs text-[var(--text-muted)] mb-1">Rate Bump (bp)</label>
          <input
            type="number"
            v-model.number="store.rateBump"
            step="0.1"
            :class="smallInputClass"
          />
        </div>
        <div>
          <label class="block text-xs text-[var(--text-muted)] mb-1">FX Bump (%)</label>
          <input
            type="number"
            v-model.number="store.fxBump"
            step="0.1"
            :class="smallInputClass"
          />
        </div>
        <div>
          <label class="block text-xs text-[var(--text-muted)] mb-1">Vol Bump (%)</label>
          <input
            type="number"
            v-model.number="store.volBump"
            step="0.1"
            :class="smallInputClass"
          />
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
