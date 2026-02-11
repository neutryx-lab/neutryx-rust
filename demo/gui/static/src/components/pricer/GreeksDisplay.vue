<script setup lang="ts">
import { computed } from 'vue';
import { usePricerStore } from '@/stores/pricer';
import { formatCurrency } from '@/utils/format';

const store = usePricerStore();

interface GreekItem {
  label: string;
  value: number;
  colored: boolean;
}

const greekItems = computed<GreekItem[]>(() => {
  if (!store.greeksResult) return [];
  const items: GreekItem[] = [
    { label: 'DV01', value: store.greeksResult.delta, colored: true },
  ];
  if (store.greeksResult.gamma !== null) {
    items.push({ label: 'Gamma', value: store.greeksResult.gamma!, colored: false });
  }
  if (store.greeksResult.theta !== null) {
    items.push({ label: 'Theta', value: store.greeksResult.theta!, colored: true });
  }
  if (store.greeksResult.vega !== null) {
    items.push({ label: 'Vega', value: store.greeksResult.vega!, colored: false });
  }
  return items;
});
</script>

<template>
  <v-card v-if="store.greeksResult">
    <v-card-title>Greeks</v-card-title>
    <v-card-text>
      <v-row dense>
        <v-col v-for="g in greekItems" :key="g.label" cols="6">
          <v-sheet rounded="lg" class="pa-3 text-center" color="surface-variant">
            <div class="text-caption text-medium-emphasis">{{ g.label }}</div>
            <div
              class="text-subtitle-1 font-weight-bold"
              :class="g.colored ? (g.value >= 0 ? 'text-success' : 'text-error') : ''"
            >
              {{ formatCurrency(g.value) }}
            </div>
          </v-sheet>
        </v-col>
      </v-row>
    </v-card-text>
  </v-card>
</template>
