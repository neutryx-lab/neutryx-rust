<script setup lang="ts">
import { usePricerStore } from '@/stores/pricer';

const store = usePricerStore();
</script>

<template>
  <div class="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
    <div
      v-for="stat in store.summaryStats"
      :key="stat.label"
      class="glass-card p-4"
    >
      <div class="flex items-start justify-between">
        <div>
          <p class="text-sm text-[var(--text-muted)] mb-1">{{ stat.label }}</p>
          <p class="text-xl font-semibold text-[var(--text-primary)]">{{ stat.value }}</p>
        </div>
        <div
          class="w-9 h-9 rounded-lg flex items-center justify-center flex-shrink-0"
          :style="{ backgroundColor: `${stat.color}1a` }"
        >
          <i :class="['fas', stat.icon, 'text-sm']" :style="{ color: stat.color }"></i>
        </div>
      </div>
    </div>
  </div>
</template>
