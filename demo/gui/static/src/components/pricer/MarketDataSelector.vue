<script setup lang="ts">
import { usePricerStore } from '@/stores/pricer';
import { CURVE_OPTIONS } from '@/constants/pricer';

const store = usePricerStore();
</script>

<template>
  <div class="glass-card p-6">
    <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Market Data</h3>
    <div class="space-y-4">
      <div>
        <label class="block text-sm text-[var(--text-muted)] mb-2">Discount Curve</label>
        <select
          v-model="store.selectedCurveIndex"
          class="w-full px-4 py-2.5 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--primary)]"
        >
          <option v-for="c in CURVE_OPTIONS" :key="c.index" :value="c.index">
            {{ c.label }}
          </option>
        </select>
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
