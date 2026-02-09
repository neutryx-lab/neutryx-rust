<script setup lang="ts">
import PvDisplay from '@/components/pricer/PvDisplay.vue';
import GreeksDisplay from '@/components/pricer/GreeksDisplay.vue';
import ComputationMetrics from '@/components/pricer/ComputationMetrics.vue';
</script>

<template>
  <div class="space-y-6">
    <PvDisplay />
    <GreeksDisplay />
    <ComputationMetrics />
  </div>
</template>
