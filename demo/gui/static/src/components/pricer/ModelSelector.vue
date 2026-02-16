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
  <div class="config-grid">
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

