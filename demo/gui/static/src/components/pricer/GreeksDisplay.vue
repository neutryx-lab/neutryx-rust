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
  <div v-if="store.greeksResult" class="glass-card p-4">
    <div class="text-xs font-semibold uppercase tracking-wider text-text-muted mb-2">Greeks</div>
    <v-row dense>
      <v-col v-for="g in greekItems" :key="g.label" cols="6">
        <div class="bg-surface-alt rounded-lg p-3 text-center">
          <div class="stat-label">{{ g.label }}</div>
          <div
            class="text-base font-bold"
            :class="g.colored ? (g.value >= 0 ? 'text-success' : 'text-error') : ''"
          >
            {{ formatCurrency(g.value) }}
          </div>
        </div>
      </v-col>
    </v-row>
  </div>
</template>
