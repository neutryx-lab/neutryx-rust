<script setup lang="ts">
import { watch } from 'vue';
import { usePricerStore } from '@/stores/pricer';
import { STOCHASTIC_MODELS } from '@/constants/pricer';

const store = usePricerStore();

// Reset model params to defaults when model type changes
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
  <div class="glass-card p-6">
    <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Stochastic Model</h3>
    <div class="space-y-4">
      <!-- Model Type -->
      <div>
        <label class="block text-sm text-[var(--text-muted)] mb-2">Model Type</label>
        <select
          v-model="store.modelType"
          class="w-full px-4 py-2.5 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--primary)]"
        >
          <option v-for="m in STOCHASTIC_MODELS" :key="m.type" :value="m.type">
            {{ m.label }}
          </option>
        </select>
      </div>

      <!-- Dynamic Model Parameters -->
      <div v-for="param in store.selectedModelConfig.params" :key="param.name">
        <label class="block text-xs text-[var(--text-muted)] mb-1">{{ param.label }}</label>
        <input
          type="number"
          v-model.number="store.modelParams[param.name]"
          :min="param.min"
          :max="param.max"
          :step="param.step"
          class="w-full px-3 py-2 text-sm rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)]"
        />
      </div>
    </div>
  </div>
</template>

<style scoped>
.glass-card {
  background: var(--glass-bg);
  backdrop-filter: blur(20px);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-lg);
  box-shadow: var(--glass-shadow);
}
</style>
