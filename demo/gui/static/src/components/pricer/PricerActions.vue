<script setup lang="ts">
import { usePricerStore } from '@/stores/pricer';
import { usePricer } from '@/composables/usePricer';

const store = usePricerStore();
const { expandCashflows, calculateAll, resetAll } = usePricer();
</script>

<template>
  <v-card>
    <v-card-text class="d-flex flex-column" style="gap: 8px">
      <v-btn
        block
        variant="tonal"
        :disabled="!store.selectedInstrumentId || store.isExpanding"
        :loading="store.isExpanding"
        prepend-icon="mdi-arrow-expand-all"
        @click="expandCashflows"
      >
        Expand Cashflows
      </v-btn>

      <v-btn
        block
        color="primary"
        :disabled="!store.expandedTrade || store.isCalculating"
        :loading="store.isCalculating"
        prepend-icon="mdi-play"
        @click="calculateAll"
      >
        Price &amp; Risks
      </v-btn>

      <v-btn
        block
        variant="text"
        :disabled="!store.expandedTrade"
        prepend-icon="mdi-undo"
        @click="resetAll"
      >
        Reset
      </v-btn>
    </v-card-text>
  </v-card>
</template>
