<script setup lang="ts">
import { useJyInflationStore } from '@/stores/jyInflation';

const store = useJyInflationStore();
</script>

<template>
  <div class="space-y-4">
    <!-- Model Parameters -->
    <h4 class="text-sm font-semibold text-[var(--text-primary)] flex items-center gap-2">
      <i class="fas fa-cog text-[var(--primary)]"></i>
      Model Parameters
    </h4>

    <div class="space-y-3">
      <div>
        <label class="text-xs text-[var(--text-muted)] mb-1 block">Nominal Mean Reversion (a<sub>n</sub>)</label>
        <input v-model.number="store.modelParams.aN" type="number" step="0.001" min="0.001" max="10"
          class="input-field" />
      </div>
      <div>
        <label class="text-xs text-[var(--text-muted)] mb-1 block">Nominal Volatility (&sigma;<sub>n</sub>)</label>
        <input v-model.number="store.modelParams.sigmaN" type="number" step="0.001" min="0.0001" max="1"
          class="input-field" />
      </div>
      <div>
        <label class="text-xs text-[var(--text-muted)] mb-1 block">Real Mean Reversion (a<sub>r</sub>)</label>
        <input v-model.number="store.modelParams.aR" type="number" step="0.001" min="0.001" max="10"
          class="input-field" />
      </div>
      <div>
        <label class="text-xs text-[var(--text-muted)] mb-1 block">Real Volatility (&sigma;<sub>r</sub>)</label>
        <input v-model.number="store.modelParams.sigmaR" type="number" step="0.001" min="0.0001" max="1"
          class="input-field" />
      </div>
      <div>
        <label class="text-xs text-[var(--text-muted)] mb-1 block">Inflation Volatility (&sigma;<sub>I</sub>)</label>
        <input v-model.number="store.modelParams.sigmaI" type="number" step="0.001" min="0.0001" max="1"
          class="input-field" />
      </div>
    </div>

    <!-- Correlations -->
    <h4 class="text-sm font-semibold text-[var(--text-primary)] flex items-center gap-2 mt-6">
      <i class="fas fa-link text-[var(--primary)]"></i>
      Correlations
    </h4>

    <div class="space-y-3">
      <div>
        <label class="text-xs text-[var(--text-muted)] mb-1 block">&rho;<sub>nr</sub> (Nominal-Real): {{ store.correlation.rhoNr.toFixed(2) }}</label>
        <input v-model.number="store.correlation.rhoNr" type="range" min="-1" max="1" step="0.01"
          class="w-full accent-[var(--primary)]" />
      </div>
      <div>
        <label class="text-xs text-[var(--text-muted)] mb-1 block">&rho;<sub>nI</sub> (Nominal-Inflation): {{ store.correlation.rhoNi.toFixed(2) }}</label>
        <input v-model.number="store.correlation.rhoNi" type="range" min="-1" max="1" step="0.01"
          class="w-full accent-[var(--primary)]" />
      </div>
      <div>
        <label class="text-xs text-[var(--text-muted)] mb-1 block">&rho;<sub>rI</sub> (Real-Inflation): {{ store.correlation.rhoRi.toFixed(2) }}</label>
        <input v-model.number="store.correlation.rhoRi" type="range" min="-1" max="1" step="0.01"
          class="w-full accent-[var(--primary)]" />
      </div>
    </div>

    <!-- Initial Conditions -->
    <h4 class="text-sm font-semibold text-[var(--text-primary)] flex items-center gap-2 mt-6">
      <i class="fas fa-play-circle text-[var(--primary)]"></i>
      Initial Conditions
    </h4>

    <div class="space-y-3">
      <div>
        <label class="text-xs text-[var(--text-muted)] mb-1 block">Nominal Rate</label>
        <input v-model.number="store.initialNominalRate" type="number" step="0.001" class="input-field" />
      </div>
      <div>
        <label class="text-xs text-[var(--text-muted)] mb-1 block">Real Rate</label>
        <input v-model.number="store.initialRealRate" type="number" step="0.001" class="input-field" />
      </div>
      <div>
        <label class="text-xs text-[var(--text-muted)] mb-1 block">Inflation Index</label>
        <input v-model.number="store.initialIndex" type="number" step="1" min="1" class="input-field" />
      </div>
      <div>
        <label class="text-xs text-[var(--text-muted)] mb-1 block">Valuation Date</label>
        <input v-model="store.valuationDate" type="date" class="input-field" />
      </div>
    </div>
  </div>
</template>

<style scoped>
.input-field {
  width: 100%;
  padding: 0.4rem 0.6rem;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--surface);
  color: var(--text-primary);
  font-size: 0.8rem;
}
.input-field:focus {
  outline: none;
  border-color: var(--primary);
}
</style>
