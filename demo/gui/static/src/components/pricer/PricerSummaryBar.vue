<script setup lang="ts">
import { usePricerStore } from '@/stores/pricer';

const store = usePricerStore();

const iconMap: Record<string, string> = {
  'fa-calendar': 'mdi-calendar',
  'fa-file-contract': 'mdi-file-document-outline',
  'fa-dollar-sign': 'mdi-currency-usd',
  'fa-chart-line': 'mdi-chart-line',
};
</script>

<template>
  <v-row class="mb-4">
    <v-col v-for="stat in store.summaryStats" :key="stat.label" cols="6" md="3">
      <div class="stat-card glass-card">
        <div class="d-flex align-center justify-space-between">
          <div>
            <div class="stat-label">{{ stat.label }}</div>
            <div class="stat-value text-truncate">{{ stat.value }}</div>
          </div>
          <v-avatar :color="stat.color" size="40" variant="tonal">
            <v-icon :icon="iconMap[stat.icon] || 'mdi-information'" />
          </v-avatar>
        </div>
      </div>
    </v-col>
  </v-row>
</template>
