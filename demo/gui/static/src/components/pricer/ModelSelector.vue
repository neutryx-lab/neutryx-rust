<script setup lang="ts">
import { watch } from 'vue';
import { usePricerStore } from '@/stores/pricer';
import { STOCHASTIC_MODELS } from '@/constants/pricer';

const store = usePricerStore();

const modelItems = STOCHASTIC_MODELS.map((m) => ({ title: m.label, value: m.type }));

watch(
  () => store.modelType,
  () => {
    const config = STOCHASTIC_MODELS.find((m) => m.type === store.modelType);
    if (config) {
      const defaults: Record<string, number> = {};
      config.params.forEach((p) => {
        defaults[p.name] = p.defaultValue;
      });
      store.modelParams = defaults;
    }
  },
);
</script>

<template>
  <div class="model-grid">
    <div class="grid-label">Model</div>
    <div class="grid-input">
      <v-select v-model="store.modelType" :items="modelItems" density="compact" variant="outlined" hide-details />
    </div>

    <template v-for="param in store.selectedModelConfig.params" :key="param.name">
      <div class="grid-label">{{ param.label }}</div>
      <div class="grid-input">
        <v-text-field
          v-model.number="store.modelParams[param.name]"
          type="number"
          :min="param.min"
          :max="param.max"
          :step="param.step"
          density="compact"
          variant="outlined"
          hide-details
        />
      </div>
    </template>
  </div>
</template>

<style scoped>
.model-grid {
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
