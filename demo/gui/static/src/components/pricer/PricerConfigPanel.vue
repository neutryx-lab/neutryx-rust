<script setup lang="ts">
import InstrumentSelector from '@/components/pricer/InstrumentSelector.vue';
import ValuationSettings from '@/components/pricer/ValuationSettings.vue';
import MarketDataSelector from '@/components/pricer/MarketDataSelector.vue';
import ModelSelector from '@/components/pricer/ModelSelector.vue';
import PricerActions from '@/components/pricer/PricerActions.vue';
</script>

<template>
  <v-expansion-panels multiple variant="accordion" :model-value="[0, 1]">
    <v-expansion-panel title="Trade Setup">
      <v-expansion-panel-text>
        <InstrumentSelector />
      </v-expansion-panel-text>
    </v-expansion-panel>

    <v-expansion-panel title="Valuation Settings">
      <v-expansion-panel-text>
        <ValuationSettings />
      </v-expansion-panel-text>
    </v-expansion-panel>

    <v-expansion-panel title="Market Data">
      <v-expansion-panel-text>
        <MarketDataSelector />
      </v-expansion-panel-text>
    </v-expansion-panel>

    <v-expansion-panel title="Stochastic Model">
      <v-expansion-panel-text>
        <ModelSelector />
      </v-expansion-panel-text>
    </v-expansion-panel>
  </v-expansion-panels>

  <PricerActions class="mt-4" />
</template>
