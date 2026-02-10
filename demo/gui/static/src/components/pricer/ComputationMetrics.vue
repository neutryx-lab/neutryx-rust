<script setup lang="ts">
import { usePricerStore } from '@/stores/pricer';

const store = usePricerStore();
</script>

<template>
  <div v-if="store.computationMetrics" class="glass-card p-4">
    <div class="flex items-center justify-between text-sm">
      <span class="text-[var(--text-muted)]">
        <i class="fas fa-clock mr-1"></i>{{ store.computationMetrics.pricingTimeMs.toFixed(0) }}ms
      </span>
      <span class="text-[var(--text-muted)]">
        <i class="fas fa-microchip mr-1"></i>{{ store.computationMetrics.method }}
      </span>
      <span class="text-[var(--text-muted)]">
        <i class="fas fa-calendar mr-1"></i
        >{{ new Date(store.computationMetrics.timestamp).toLocaleTimeString() }}
      </span>
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
