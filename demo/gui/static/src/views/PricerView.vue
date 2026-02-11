<script setup lang="ts">
/**
 * PricerView — Orchestrator (Vuetify Material UI)
 *
 * Flexible table-based pricer layout using Vuetify v-data-table,
 * v-card, v-expansion-panels, and Material Design components.
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
    <v-alert v-if="!store.apiAvailable" type="info" variant="tonal" class="mb-6">
      Pricer API is not available in this build configuration.
    </v-alert>

    <!-- Main Layout -->
    <template v-else>
      <v-row>
        <!-- Left Panel: Config + Results (4 cols) -->
        <v-col cols="12" lg="4">
          <div class="d-flex flex-column" style="gap: 16px">
            <PricerConfigPanel />
            <PricerResultsPanel />
            <PricerHistory />
          </div>
        </v-col>

        <!-- Right Panel: Cashflows (8 cols) -->
        <v-col cols="12" lg="8">
          <CashflowTable />
        </v-col>
      </v-row>
    </template>
  </div>
</template>
