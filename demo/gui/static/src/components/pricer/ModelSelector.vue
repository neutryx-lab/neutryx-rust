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
  <div class="d-flex flex-column" style="gap: 12px">
    <v-select v-model="store.modelType" :items="modelItems" label="Model Type" />

    <v-text-field
      v-for="param in store.selectedModelConfig.params"
      :key="param.name"
      v-model.number="store.modelParams[param.name]"
      :label="param.label"
      type="number"
      :min="param.min"
      :max="param.max"
      :step="param.step"
    />
  </div>
</template>
