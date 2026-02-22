<script setup lang="ts">
import { useJyInflationStore } from '@/stores/jyInflation';

const store = useJyInflationStore();

function addNominalRate() {
  store.nominalRates.push({ instrumentType: 'Swap', tenor: '', rate: 0.04 });
}

function removeNominalRate(index: number) {
  store.nominalRates.splice(index, 1);
}

function addRealRate() {
  store.realRates.push({ instrumentType: 'TIPS', tenor: '', rate: 0.01 });
}

function removeRealRate(index: number) {
  store.realRates.splice(index, 1);
}
</script>

<template>
  <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
    <!-- Model Parameters -->
    <div class="space-y-4">
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

    <!-- Nominal Rates Table -->
    <div class="space-y-4">
      <div class="flex items-center justify-between">
        <h4 class="text-sm font-semibold text-[var(--text-primary)] flex items-center gap-2">
          <i class="fas fa-table text-blue-500"></i>
          Nominal Rates (USD Swaps)
        </h4>
        <button class="text-xs text-[var(--primary)] hover:underline" @click="addNominalRate">+ Add</button>
      </div>

      <div class="overflow-auto max-h-96">
        <table class="w-full text-xs">
          <thead>
            <tr class="text-[var(--text-muted)] border-b border-[var(--border)]">
              <th class="text-left py-2 px-2">Type</th>
              <th class="text-left py-2 px-2">Tenor</th>
              <th class="text-right py-2 px-2">Rate (%)</th>
              <th class="py-2 px-1"></th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(pt, i) in store.nominalRates" :key="i" class="border-b border-[var(--border)] border-opacity-50">
              <td class="py-1.5 px-2">
                <input v-model="pt.instrumentType" class="input-field-sm w-20" />
              </td>
              <td class="py-1.5 px-2">
                <input v-model="pt.tenor" class="input-field-sm w-16" />
              </td>
              <td class="py-1.5 px-2 text-right">
                <input v-model.number="pt.rate" type="number" step="0.001" class="input-field-sm w-20 text-right" />
              </td>
              <td class="py-1.5 px-1">
                <button class="text-[var(--text-muted)] hover:text-red-500 text-xs" @click="removeNominalRate(i)">
                  <i class="fas fa-times"></i>
                </button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- Real Rates Table -->
    <div class="space-y-4">
      <div class="flex items-center justify-between">
        <h4 class="text-sm font-semibold text-[var(--text-primary)] flex items-center gap-2">
          <i class="fas fa-table text-green-500"></i>
          Real Rates (TIPS Yields)
        </h4>
        <button class="text-xs text-[var(--primary)] hover:underline" @click="addRealRate">+ Add</button>
      </div>

      <div class="overflow-auto max-h-96">
        <table class="w-full text-xs">
          <thead>
            <tr class="text-[var(--text-muted)] border-b border-[var(--border)]">
              <th class="text-left py-2 px-2">Type</th>
              <th class="text-left py-2 px-2">Tenor</th>
              <th class="text-right py-2 px-2">Rate (%)</th>
              <th class="py-2 px-1"></th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(pt, i) in store.realRates" :key="i" class="border-b border-[var(--border)] border-opacity-50">
              <td class="py-1.5 px-2">
                <input v-model="pt.instrumentType" class="input-field-sm w-20" />
              </td>
              <td class="py-1.5 px-2">
                <input v-model="pt.tenor" class="input-field-sm w-16" />
              </td>
              <td class="py-1.5 px-2 text-right">
                <input v-model.number="pt.rate" type="number" step="0.001" class="input-field-sm w-20 text-right" />
              </td>
              <td class="py-1.5 px-1">
                <button class="text-[var(--text-muted)] hover:text-red-500 text-xs" @click="removeRealRate(i)">
                  <i class="fas fa-times"></i>
                </button>
              </td>
            </tr>
          </tbody>
        </table>
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
.input-field-sm {
  padding: 0.25rem 0.4rem;
  border: 1px solid var(--border);
  border-radius: 4px;
  background: var(--surface);
  color: var(--text-primary);
  font-size: 0.75rem;
}
.input-field-sm:focus {
  outline: none;
  border-color: var(--primary);
}
</style>
