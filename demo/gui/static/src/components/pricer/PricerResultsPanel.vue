<script setup lang="ts">
import PvDisplay from '@/components/pricer/PvDisplay.vue';
import GreeksDisplay from '@/components/pricer/GreeksDisplay.vue';
import ComputationMetrics from '@/components/pricer/ComputationMetrics.vue';
</script>

<template>
  <div class="d-flex flex-column" style="gap: 16px">
    <PvDisplay />
    <GreeksDisplay />
    <ComputationMetrics />
  </div>
</template>
