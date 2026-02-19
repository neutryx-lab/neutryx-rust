<script setup lang="ts">
import { usePricerStore } from '@/stores/pricer';

const store = usePricerStore();
</script>

<template>
  <v-card v-if="store.computationMetrics" variant="outlined">
    <v-card-text class="d-flex justify-space-between text-caption text-medium-emphasis">
      <span>
        <v-icon icon="mdi-timer-outline" size="14" class="mr-1" />
        {{ store.computationMetrics.pricingTimeMs.toFixed(0) }}ms
      </span>
      <span>
        <v-icon icon="mdi-chip" size="14" class="mr-1" />
        {{ store.computationMetrics.method }}
      </span>
      <span>
        <v-icon icon="mdi-clock-outline" size="14" class="mr-1" />
        {{ new Date(store.computationMetrics.timestamp).toLocaleTimeString() }}
      </span>
    </v-card-text>
  </v-card>
</template>
