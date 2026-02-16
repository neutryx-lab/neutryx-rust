<script setup lang="ts">
import { usePricerStore } from '@/stores/pricer';
import { useMarketEnvStore } from '@/stores/marketEnv';

const store = usePricerStore();
const marketEnv = useMarketEnvStore();
</script>

<template>
  <div class="market-grid">
    <div class="grid-label">Curve</div>
    <div class="grid-input">
      <v-select
        v-model="store.selectedCurveIndex"
        :items="marketEnv.allCurveItems"
        density="compact"
        variant="outlined"
        hide-details
      />
    </div>
    <template v-if="marketEnv.volSurfaces.length > 0">
      <div class="grid-label">Vol Surf</div>
      <div class="grid-input">
        <v-select
          v-model="store.selectedVolSurfaceId"
          :items="[{ title: '(none)', value: '' }, ...marketEnv.allVolSurfaceItems]"
          density="compact"
          variant="outlined"
          hide-details
        />
      </div>
    </template>
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
