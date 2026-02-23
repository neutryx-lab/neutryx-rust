<script setup lang="ts">
/**
 * PricerView — Unified pricer layout.
 *
 * Standard pricer with all instrument types (including Exotic and MFM products
 * integrated into the standard asset-class tabs).
 * When the Inflation tab is selected, JY Inflation panels replace the standard
 * cashflow table and results panels.
 */
import { computed, onMounted } from 'vue';
import { usePricerStore } from '@/stores/pricer';
import { useInstruments } from '@/composables/useInstruments';
import { usePricer } from '@/composables/usePricer';

import AssetTabBar from '@/components/common/AssetTabBar.vue';
import type { TabItem } from '@/components/common/AssetTabBar.vue';
import PricerSummaryBar from '@/components/pricer/PricerSummaryBar.vue';
import PricerConfigPanel from '@/components/pricer/PricerConfigPanel.vue';
import PricerResultsPanel from '@/components/pricer/PricerResultsPanel.vue';
import CashflowTable from '@/components/pricer/CashflowTable.vue';

import JyInstrumentPanel from '@/components/jy/JyInstrumentPanel.vue';
import JyPricingPanel from '@/components/jy/JyPricingPanel.vue';
import JyXvaPanel from '@/components/jy/JyXvaPanel.vue';

import { useJyInflationStore } from '@/stores/jyInflation';

const store = usePricerStore();
const { loadInstruments } = useInstruments();
usePricer();

const jyStore = useJyInflationStore();
const isInflation = computed(() => store.assetTab === 'Inflation');

const assetTabIcons: Record<string, string> = {
  Rates: 'fa-chart-line',
  FX: 'fa-exchange-alt',
  Equity: 'fa-arrow-trend-up',
  Credit: 'fa-shield-halved',
  Commodity: 'fa-oil-can',
  Inflation: 'fa-chart-bar',
};

const assetTabs = computed<TabItem[]>(() =>
  store.assetClasses.map((cls) => ({
    key: cls,
    label: cls,
    icon: assetTabIcons[cls],
  })),
);

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

    <!-- Asset Class Tabs -->
    <AssetTabBar v-model="store.assetTab" :tabs="assetTabs" class="mb-6" />

    <!-- API Not Available Fallback -->
    <v-alert v-if="!store.apiAvailable" type="info" variant="tonal" class="mb-6">
      Pricer API is not available in this build configuration.
    </v-alert>

    <!-- Main Layout -->
    <template v-else>
      <v-row>
        <!-- Left Panel: Config (4 cols) -->
        <v-col cols="12" lg="4">
          <PricerConfigPanel />
        </v-col>

        <!-- Right Panel: Standard Cashflows or Inflation Cashflows -->
        <v-col cols="12" lg="8">
          <template v-if="!isInflation">
            <CashflowTable />
          </template>
          <template v-else>
            <JyInstrumentPanel />
          </template>
        </v-col>
      </v-row>

      <!-- Results (below, full width) -->
      <v-row class="mt-2">
        <v-col cols="12">
          <template v-if="!isInflation">
            <PricerResultsPanel />
          </template>
          <template v-else>
            <JyPricingPanel :result="jyStore.pricingResult" />
            <div v-if="jyStore.xvaResult" class="mt-4">
              <JyXvaPanel :result="jyStore.xvaResult" />
            </div>
          </template>
        </v-col>
      </v-row>
    </template>
  </div>
</template>
