<script setup lang="ts">
/**
 * PricerView — Orchestrator
 *
 * Composes sub-components via wrapper panels for configuration,
 * results, cashflow display, and summary. All logic is delegated
 * to composables and the Pinia store.
 */
import { onMounted } from 'vue';
import { usePricerStore } from '@/stores/pricer';
import { useInstruments } from '@/composables/useInstruments';
import { usePricer } from '@/composables/usePricer';

import PricerSummaryBar from '@/components/pricer/PricerSummaryBar.vue';
import PricerConfigPanel from '@/components/pricer/PricerConfigPanel.vue';
import PricerResultsPanel from '@/components/pricer/PricerResultsPanel.vue';
import CashflowTable from '@/components/pricer/CashflowTable.vue';
import PricerHistory from '@/components/pricer/PricerHistory.vue';

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
    <!-- Summary Bar -->
    <PricerSummaryBar />

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
        <!-- Left Panel: Config + Results (1/3) -->
        <div class="space-y-6">
          <PricerConfigPanel />
          <PricerResultsPanel />
          <PricerHistory />
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
