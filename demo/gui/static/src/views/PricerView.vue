<script setup lang="ts">
/**
 * PricerView — Unified pricer layout.
 *
 * Standard pricer with all instrument types (including Exotic and MFM products
 * integrated into the standard asset-class tabs), plus JY Inflation Swaps
 * available as a collapsible expansion panel below.
 */
import { onMounted, ref } from 'vue';
import { usePricerStore } from '@/stores/pricer';
import { useInstruments } from '@/composables/useInstruments';
import { usePricer } from '@/composables/usePricer';

import PricerSummaryBar from '@/components/pricer/PricerSummaryBar.vue';
import PricerConfigPanel from '@/components/pricer/PricerConfigPanel.vue';
import PricerResultsPanel from '@/components/pricer/PricerResultsPanel.vue';
import CashflowTable from '@/components/pricer/CashflowTable.vue';

import JyInstrumentPanel from '@/components/jy/JyInstrumentPanel.vue';
import JyPricingPanel from '@/components/jy/JyPricingPanel.vue';
import JyXvaPanel from '@/components/jy/JyXvaPanel.vue';

import { useJyInflationStore } from '@/stores/jyInflation';
import { useJYInflation } from '@/composables/useJYInflation';

const store = usePricerStore();
const { loadInstruments } = useInstruments();
usePricer();

// ---------------------------------------------------------------------------
// JY Inflation state
// ---------------------------------------------------------------------------
const jyStore = useJyInflationStore();
const { generateCashflows: jyGenerateCashflows, runPricing: jyRunPricing, runXva: jyRunXva } = useJYInflation();
const inflationExpanded = ref(false);

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------
onMounted(() => {
  loadInstruments();
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
      <!-- Standard Pricer -->
      <v-row>
        <!-- Left Panel: Config (4 cols) -->
        <v-col cols="12" lg="4">
          <PricerConfigPanel />
        </v-col>

        <!-- Right Panel: Cashflows (8 cols) -->
        <v-col cols="12" lg="8">
          <CashflowTable />
        </v-col>
      </v-row>

      <!-- Results (below, full width) -->
      <v-row class="mt-2">
        <v-col cols="12">
          <PricerResultsPanel />
        </v-col>
      </v-row>

      <!-- Inflation Swaps (collapsible) -->
      <v-row class="mt-4">
        <v-col cols="12">
          <v-expansion-panels v-model="inflationExpanded">
            <v-expansion-panel value="inflation">
              <v-expansion-panel-title>
                <div class="d-flex align-center gap-2">
                  <v-icon icon="mdi-chart-bar" size="20" />
                  <span class="text-subtitle-1 font-weight-medium">Inflation Swaps (JY Model)</span>
                </div>
              </v-expansion-panel-title>
              <v-expansion-panel-text>
                <!-- Instrument Configuration + Cashflows -->
                <JyInstrumentPanel @generate="jyGenerateCashflows" />

                <!-- Pricing -->
                <div class="mt-6">
                  <div class="d-flex align-center gap-2 mb-4">
                    <v-btn
                      color="primary"
                      :loading="jyStore.loading"
                      :disabled="jyStore.loading"
                      @click="jyRunPricing"
                    >
                      <v-icon start>mdi-calculator</v-icon>
                      Price
                    </v-btn>
                    <v-btn
                      color="secondary"
                      variant="outlined"
                      :loading="jyStore.loading"
                      :disabled="jyStore.loading"
                      @click="jyRunXva"
                    >
                      <v-icon start>mdi-shield-half-full</v-icon>
                      Compute XVA
                    </v-btn>
                  </div>
                  <JyPricingPanel :result="jyStore.pricingResult" />
                </div>

                <!-- XVA -->
                <div v-if="jyStore.xvaResult" class="mt-6">
                  <JyXvaPanel :result="jyStore.xvaResult" />
                </div>
              </v-expansion-panel-text>
            </v-expansion-panel>
          </v-expansion-panels>
        </v-col>
      </v-row>
    </template>
  </div>
</template>
