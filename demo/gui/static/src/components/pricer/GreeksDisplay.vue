<script setup lang="ts">
import { usePricerStore } from '@/stores/pricer';
import { formatCurrency } from '@/utils/format';

const store = usePricerStore();
</script>

<template>
  <div v-if="store.greeksResult" class="glass-card p-6">
    <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Greeks</h3>
    <div class="grid grid-cols-2 gap-4">
      <!-- DV01 -->
      <div class="text-center p-3 rounded-lg bg-[var(--surface)]">
        <p class="text-xs text-[var(--text-muted)] mb-1">DV01</p>
        <p
          :class="[
            'text-lg font-semibold',
            store.greeksResult.delta >= 0 ? 'text-[var(--success)]' : 'text-[var(--danger)]',
          ]"
        >
          {{ formatCurrency(store.greeksResult.delta) }}
        </p>
      </div>

      <!-- Gamma -->
      <div v-if="store.greeksResult.gamma !== null" class="text-center p-3 rounded-lg bg-[var(--surface)]">
        <p class="text-xs text-[var(--text-muted)] mb-1">Gamma</p>
        <p class="text-lg font-semibold text-[var(--text-primary)]">
          {{ formatCurrency(store.greeksResult.gamma!) }}
        </p>
      </div>

      <!-- Theta -->
      <div v-if="store.greeksResult.theta !== null" class="text-center p-3 rounded-lg bg-[var(--surface)]">
        <p class="text-xs text-[var(--text-muted)] mb-1">Theta</p>
        <p
          :class="[
            'text-lg font-semibold',
            store.greeksResult.theta! >= 0 ? 'text-[var(--success)]' : 'text-[var(--danger)]',
          ]"
        >
          {{ formatCurrency(store.greeksResult.theta!) }}
        </p>
      </div>

      <!-- Vega -->
      <div v-if="store.greeksResult.vega !== null" class="text-center p-3 rounded-lg bg-[var(--surface)]">
        <p class="text-xs text-[var(--text-muted)] mb-1">Vega</p>
        <p class="text-lg font-semibold text-[var(--text-primary)]">
          {{ formatCurrency(store.greeksResult.vega!) }}
        </p>
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
