<script setup lang="ts">
/**
 * PricerView — MVP Orchestrator
 *
 * Composes sub-components for instrument selection, valuation settings,
 * cashflow display, pricing results, and action buttons.
 * All logic is delegated to composables and the Pinia store.
 */
import { onMounted } from 'vue';
import { usePricerStore } from '@/stores/pricer';
import { useInstruments } from '@/composables/useInstruments';
import { usePricer } from '@/composables/usePricer';

import InstrumentSelector from '@/components/pricer/InstrumentSelector.vue';
import ValuationSettings from '@/components/pricer/ValuationSettings.vue';
import PricerActions from '@/components/pricer/PricerActions.vue';
import CashflowTable from '@/components/pricer/CashflowTable.vue';
import PvDisplay from '@/components/pricer/PvDisplay.vue';

const store = usePricerStore();
const { loadInstruments } = useInstruments();
const { expandCashflows } = usePricer();

onMounted(async () => {
  await loadInstruments();
  // Auto-expand after IRS auto-selection
  if (store.selectedInstrumentId && store.instruments.length > 0) {
    expandCashflows();
  }
});
</script>

<template>
  <div class="pricer-view">
    <!-- Summary Stats -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
      <div v-for="stat in store.summaryStats" :key="stat.label" class="glass-card p-4">
        <div class="flex items-start justify-between">
          <div>
            <p class="text-sm text-[var(--text-muted)] mb-1">{{ stat.label }}</p>
            <p class="text-2xl font-semibold text-[var(--text-primary)] truncate">
              {{ stat.value }}
            </p>
          </div>
          <div
            class="w-10 h-10 rounded-lg flex items-center justify-center flex-shrink-0"
            :style="{ backgroundColor: `${stat.color}1a` }"
          >
            <i :class="['fas', stat.icon]" :style="{ color: stat.color }"></i>
          </div>
        </div>
      </div>
    </div>

    <!-- API Not Available Fallback -->
    <div v-if="!store.apiAvailable" class="glass-card p-8 text-center">
      <i class="fas fa-info-circle text-4xl text-[var(--text-muted)] mb-4"></i>
      <p class="text-[var(--text-muted)]">
        Pricer API is not available in this build configuration.
      </p>
    </div>

    <!-- Main Layout: 3-column grid -->
    <template v-else>
      <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <!-- Left Panel: Configuration (1/3) -->
        <div class="space-y-6">
          <InstrumentSelector />
          <ValuationSettings />
          <PricerActions />
          <PvDisplay />
        </div>

        <!-- Right Panel: Cashflows (2/3) -->
        <div class="lg:col-span-2">
          <CashflowTable />
        </div>
      </div>
    </template>
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
