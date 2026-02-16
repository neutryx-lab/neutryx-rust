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
      <v-card variant="outlined">
        <v-card-text class="d-flex align-center justify-space-between">
          <div>
            <div class="text-caption text-medium-emphasis">{{ stat.label }}</div>
            <div class="text-h6 font-weight-bold text-truncate">{{ stat.value }}</div>
          </div>
          <v-avatar :color="stat.color" size="40" variant="tonal">
            <v-icon :icon="iconMap[stat.icon] || 'mdi-information'" />
          </v-avatar>
        </v-card-text>
      </v-card>
    </v-col>
  </v-row>
</template>
