<script setup lang="ts">
import { usePricerStore } from '@/stores/pricer';
import { CURVE_OPTIONS } from '@/constants/pricer';

const store = usePricerStore();

const curveItems = CURVE_OPTIONS.map((c) => ({ title: c.label, value: c.index }));
</script>

<template>
  <v-select v-model="store.selectedCurveIndex" :items="curveItems" label="Discount Curve" />
</template>
