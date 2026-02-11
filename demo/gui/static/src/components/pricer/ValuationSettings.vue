<script setup lang="ts">
import { usePricerStore } from '@/stores/pricer';

const store = usePricerStore();

const ccyItems = [
  { title: 'USD', value: 'USD' },
  { title: 'EUR', value: 'EUR' },
  { title: 'GBP', value: 'GBP' },
  { title: 'JPY', value: 'JPY' },
];
</script>

<template>
  <div class="d-flex flex-column" style="gap: 12px">
    <v-text-field v-model="store.valuationDate" label="Valuation Date" type="date" />
    <v-select v-model="store.reportingCcy" :items="ccyItems" label="Reporting Currency" />

    <v-switch
      v-model="store.useDefaults"
      label="Use Default Model Config"
      color="primary"
      density="compact"
      hide-details
    />

    <v-row v-if="!store.useDefaults" dense>
      <v-col cols="6">
        <v-text-field v-model.number="store.numPaths" label="Paths" type="number" />
      </v-col>
      <v-col cols="6">
        <v-text-field v-model.number="store.numSteps" label="Steps" type="number" />
      </v-col>
    </v-row>

    <v-row dense>
      <v-col cols="4">
        <v-text-field v-model.number="store.rateBump" label="Rate (bp)" type="number" step="0.1" />
      </v-col>
      <v-col cols="4">
        <v-text-field v-model.number="store.fxBump" label="FX (%)" type="number" step="0.1" />
      </v-col>
      <v-col cols="4">
        <v-text-field v-model.number="store.volBump" label="Vol (%)" type="number" step="0.1" />
      </v-col>
    </v-row>
  </div>
</template>
