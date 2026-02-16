<script setup lang="ts">
import { usePricerStore } from '@/stores/pricer';
import { CURVE_OPTIONS } from '@/constants/pricer';

const store = usePricerStore();

const curveItems = CURVE_OPTIONS.map((c) => ({ title: c.label, value: c.index }));
</script>

<template>
  <div class="market-grid">
    <div class="grid-label">Curve</div>
    <div class="grid-input">
      <v-select v-model="store.selectedCurveIndex" :items="curveItems" density="compact" variant="outlined" hide-details />
    </div>
  </div>
</template>

<style scoped>
.market-grid {
  display: grid;
  grid-template-columns: 90px 1fr;
  align-items: center;
  gap: 4px 8px;
}

.grid-label {
  font-size: 0.8rem;
  color: rgba(var(--v-theme-on-surface), 0.7);
  text-align: right;
  padding-right: 4px;
  white-space: nowrap;
  line-height: 1.2;
}

.grid-input {
  min-width: 0;
}
</style>
