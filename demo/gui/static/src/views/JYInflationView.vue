<script setup lang="ts">
import { computed } from 'vue';
import { useJYInflation } from '@/composables/useJYInflation';
import JyInputPanel from '@/components/jy/JyInputPanel.vue';
import JyInstrumentPanel from '@/components/jy/JyInstrumentPanel.vue';
import JyCurvePanel from '@/components/jy/JyCurvePanel.vue';
import JySimulationPanel from '@/components/jy/JySimulationPanel.vue';
import JyPricingPanel from '@/components/jy/JyPricingPanel.vue';
import JyXvaPanel from '@/components/jy/JyXvaPanel.vue';

const { store, buildCurves, generateCashflows, runSimulation, runPricing, runXva } = useJYInflation();

const tabs = [
  { label: 'Input', icon: 'fa-sliders-h' },
  { label: 'Instrument', icon: 'fa-file-contract' },
  { label: 'Curves', icon: 'fa-chart-line' },
  { label: 'Simulation', icon: 'fa-random' },
  { label: 'Pricing', icon: 'fa-calculator' },
  { label: 'XVA', icon: 'fa-shield-alt' },
];

const canRunStep = computed(() => {
  switch (store.activeStep) {
    case 2: return store.nominalRates.length > 0 && store.realRates.length > 0;
    case 3: return true;
    case 4: return true;
    case 5: return true;
    default: return false;
  }
});

async function runCurrentStep() {
  switch (store.activeStep) {
    case 2: await buildCurves(); break;
    case 3: await runSimulation(); break;
    case 4: await runPricing(); break;
    case 5: await runXva(); break;
  }
}

function nextStep() {
  if (store.activeStep < 5) store.activeStep++;
}

function prevStep() {
  if (store.activeStep > 0) store.activeStep--;
}
</script>

<template>
  <div class="jy-view">
    <!-- Header -->
    <div class="flex items-center justify-between mb-6">
      <div>
        <h2 class="text-2xl font-semibold text-[var(--text-primary)]">Jarrow-Yildirim Inflation Model</h2>
        <p class="text-sm text-[var(--text-muted)]">3-Factor Stochastic Model: Nominal Rate + Real Rate + Inflation Index</p>
      </div>
      <div class="flex gap-3">
        <button
          v-if="store.activeStep > 0"
          class="px-4 py-2 rounded-lg text-sm font-medium bg-[var(--surface)] text-[var(--text-secondary)] hover:bg-[var(--surface-hover)] transition-all"
          @click="prevStep"
        >
          <i class="fas fa-arrow-left mr-2"></i>Previous
        </button>
        <button
          v-if="canRunStep"
          :class="[
            'px-4 py-2 rounded-lg text-sm font-medium transition-all',
            store.loading ? 'bg-gray-500 cursor-not-allowed' : 'bg-[var(--primary)] hover:opacity-90'
          ]"
          class="text-white"
          :disabled="store.loading"
          @click="runCurrentStep"
        >
          <i :class="['fas mr-2', store.loading ? 'fa-spinner fa-spin' : 'fa-play']"></i>
          {{ store.loading ? 'Computing...' : tabs[store.activeStep].label }}
        </button>
        <button
          v-if="store.activeStep < 5"
          class="px-4 py-2 rounded-lg text-sm font-medium bg-[var(--primary)] text-white hover:opacity-90 transition-all"
          @click="nextStep"
        >
          Next<i class="fas fa-arrow-right ml-2"></i>
        </button>
      </div>
    </div>

    <!-- Summary Cards -->
    <div class="grid grid-cols-4 gap-4 mb-6">
      <div
        v-for="stat in store.summaryStats"
        :key="stat.label"
        class="glass-card p-4 flex items-center gap-3"
      >
        <div
          class="w-10 h-10 rounded-lg flex items-center justify-center text-white text-sm"
          :style="{ backgroundColor: stat.color }"
        >
          <i :class="['fas', stat.icon]"></i>
        </div>
        <div>
          <div class="text-xs text-[var(--text-muted)]">{{ stat.label }}</div>
          <div class="text-sm font-semibold text-[var(--text-primary)]">{{ stat.value }}</div>
        </div>
      </div>
    </div>

    <!-- Tab Navigation -->
    <div class="glass-card mb-6">
      <div class="flex border-b border-[var(--border)]">
        <button
          v-for="(tab, index) in tabs"
          :key="tab.label"
          :class="[
            'flex items-center gap-2 px-5 py-3 text-sm font-medium transition-all border-b-2',
            store.activeStep === index
              ? 'border-[var(--primary)] text-[var(--primary)]'
              : 'border-transparent text-[var(--text-muted)] hover:text-[var(--text-secondary)]'
          ]"
          @click="store.activeStep = index"
        >
          <i :class="['fas', tab.icon]"></i>
          {{ tab.label }}
        </button>
      </div>

      <!-- Tab Content -->
      <div class="p-6">
        <JyInputPanel v-if="store.activeStep === 0" />
        <JyInstrumentPanel v-else-if="store.activeStep === 1" @generate="generateCashflows" />
        <JyCurvePanel v-else-if="store.activeStep === 2" :result="store.curveResult" />
        <JySimulationPanel v-else-if="store.activeStep === 3" :result="store.simulationResult" />
        <JyPricingPanel v-else-if="store.activeStep === 4" :result="store.pricingResult" />
        <JyXvaPanel v-else-if="store.activeStep === 5" :result="store.xvaResult" />
      </div>
    </div>
  </div>
</template>

<style scoped>
.jy-view {
  padding: 1.5rem;
  max-width: 1400px;
  margin: 0 auto;
}

.glass-card {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 12px;
  backdrop-filter: blur(8px);
}
</style>
