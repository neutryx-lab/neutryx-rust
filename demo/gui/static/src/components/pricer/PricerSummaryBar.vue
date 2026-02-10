<script setup lang="ts">
import { usePricerStore } from '@/stores/pricer';

const store = usePricerStore();
</script>

<template>
  <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
    <div v-for="stat in store.summaryStats" :key="stat.label" class="glass-card p-4">
      <div class="flex items-start justify-between">
        <div>
          <p class="text-sm text-[var(--text-muted)] mb-1">{{ stat.label }}</p>
          <p class="text-2xl font-semibold text-[var(--text-primary)] truncate">
            {{ stat.value }}
          </p>
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
