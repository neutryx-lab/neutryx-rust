<script setup lang="ts">
import InstrumentSelector from '@/components/pricer/InstrumentSelector.vue';
import ValuationSettings from '@/components/pricer/ValuationSettings.vue';
import MarketDataSelector from '@/components/pricer/MarketDataSelector.vue';
import ModelSelector from '@/components/pricer/ModelSelector.vue';
import PricerActions from '@/components/pricer/PricerActions.vue';
</script>

<template>
  <div class="space-y-6">
    <InstrumentSelector />
    <ValuationSettings />
    <MarketDataSelector />
    <ModelSelector />
    <PricerActions />
  </div>
</template>
