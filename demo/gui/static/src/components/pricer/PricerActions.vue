<script setup lang="ts">
import { usePricerStore } from '@/stores/pricer';
import { usePricer } from '@/composables/usePricer';

const store = usePricerStore();
const { expandCashflows, calculateAll, resetAll } = usePricer();
</script>

<template>
  <div class="glass-card p-6">
    <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Actions</h3>
    <div class="space-y-3">
      <!-- Expand Cashflows -->
      <button
        :disabled="!store.selectedInstrumentId || store.isExpanding"
        class="w-full px-4 py-2.5 rounded-lg bg-[var(--surface)] text-[var(--text-primary)] font-medium transition-all duration-200 hover:bg-[var(--surface-hover)] disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
        @click="expandCashflows"
      >
        <i :class="['fas', store.isExpanding ? 'fa-spinner fa-spin' : 'fa-expand']"></i>
        {{ store.isExpanding ? 'Expanding...' : 'Expand Cashflows' }}
      </button>

      <!-- Price & Risks -->
      <button
        :disabled="!store.expandedTrade || store.isCalculating"
        class="w-full px-4 py-2.5 rounded-lg bg-[var(--primary)] text-white font-medium transition-all duration-200 hover:opacity-90 disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
        @click="calculateAll"
      >
        <i :class="['fas', store.isCalculating ? 'fa-spinner fa-spin' : 'fa-play']"></i>
        {{ store.isCalculating ? 'Calculating...' : 'Price & Risks' }}
      </button>

      <!-- Reset -->
      <button
        :disabled="!store.expandedTrade"
        class="w-full px-4 py-2 rounded-lg bg-[var(--surface)] text-[var(--text-secondary)] font-medium transition-all duration-200 hover:bg-[var(--surface-hover)] disabled:opacity-50 disabled:cursor-not-allowed"
        @click="resetAll"
      >
        <i class="fas fa-undo mr-2"></i>Reset
      </button>
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
