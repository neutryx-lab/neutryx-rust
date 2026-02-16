<script setup lang="ts">
import { usePricerStore } from '@/stores/pricer';
import { parseTenorToDate } from '@/utils/format';
import { ref } from 'vue';

const store = usePricerStore();

const ccyItems = [
  { title: 'USD', value: 'USD' },
  { title: 'EUR', value: 'EUR' },
  { title: 'GBP', value: 'GBP' },
  { title: 'JPY', value: 'JPY' },
];

// Tenor-first valuation date input
const valDateDisplay = ref(store.valuationDate);

function onValInput(raw: string) {
  valDateDisplay.value = raw;
  const resolved = parseTenorToDate(raw);
  if (resolved) {
    store.valuationDate = resolved;
  }
}

function onValCommit() {
  const raw = valDateDisplay.value;
  const resolved = parseTenorToDate(raw);
  if (resolved) {
    store.valuationDate = resolved;
    valDateDisplay.value = resolved;
  } else {
    valDateDisplay.value = store.valuationDate;
  }
}

function applyValTenor(tenor: string) {
  const resolved = parseTenorToDate(tenor);
  if (resolved) {
    store.valuationDate = resolved;
    valDateDisplay.value = resolved;
  }
}

function onValCalendarPick(date: unknown) {
  const d = String(date);
  store.valuationDate = d;
  valDateDisplay.value = d;
}
</script>

<template>
  <div class="settings-grid">
    <div class="grid-label">Val Date</div>
    <div class="grid-input">
      <v-text-field
        :model-value="valDateDisplay"
        density="compact"
        variant="outlined"
        hide-details
        placeholder="TD, 1D or YYYY-MM-DD"
        @update:model-value="onValInput"
        @blur="onValCommit"
        @keydown.enter="onValCommit"
      >
        <template #prepend-inner>
          <div class="d-flex" style="gap: 1px">
            <v-btn v-for="t in ['TD', '1D']" :key="t" size="x-small" variant="text" density="compact" class="tenor-chip" @click="applyValTenor(t)">{{ t }}</v-btn>
          </div>
        </template>
        <template #append-inner>
          <v-menu :close-on-content-click="false" location="bottom end">
            <template #activator="{ props: menuProps }">
              <v-btn v-bind="menuProps" icon density="compact" variant="text" size="x-small" tabindex="-1">
                <v-icon size="14">mdi-calendar</v-icon>
              </v-btn>
            </template>
            <v-date-picker
              :model-value="store.valuationDate || undefined"
              @update:model-value="onValCalendarPick"
            />
          </v-menu>
        </template>
      </v-text-field>
    </div>

    <div class="grid-label">Rpt Ccy</div>
    <div class="grid-input">
      <v-select v-model="store.reportingCcy" :items="ccyItems" density="compact" variant="outlined" hide-details />
    </div>

    <div class="grid-label">Defaults</div>
    <div class="grid-input">
      <v-switch v-model="store.useDefaults" color="primary" density="compact" hide-details />
    </div>

    <template v-if="!store.useDefaults">
      <div class="grid-label">Paths</div>
      <div class="grid-input">
        <v-text-field v-model.number="store.numPaths" type="number" density="compact" variant="outlined" hide-details />
      </div>
      <div class="grid-label">Steps</div>
      <div class="grid-input">
        <v-text-field v-model.number="store.numSteps" type="number" density="compact" variant="outlined" hide-details />
      </div>
    </template>

    <div class="grid-label">Rate bp</div>
    <div class="grid-input">
      <v-text-field v-model.number="store.rateBump" type="number" step="0.1" density="compact" variant="outlined" hide-details />
    </div>
    <div class="grid-label">FX %</div>
    <div class="grid-input">
      <v-text-field v-model.number="store.fxBump" type="number" step="0.1" density="compact" variant="outlined" hide-details />
    </div>
    <div class="grid-label">Vol %</div>
    <div class="grid-input">
      <v-text-field v-model.number="store.volBump" type="number" step="0.1" density="compact" variant="outlined" hide-details />
    </div>
  </div>
</template>

<style scoped>
.settings-grid {
  display: grid;
  grid-template-columns: 90px 1fr;
  align-items: center;
  gap: 4px 8px;
}

.grid-label {
  font-size: 0.8rem;
  color: rgba(var(--v-theme-on-surface), 0.7);
  text-align: right;
  padding-right: 4px;
  white-space: nowrap;
  line-height: 1.2;
}

.grid-input {
  min-width: 0;
}

.tenor-chip {
  min-width: 0 !important;
  padding: 0 4px !important;
  font-size: 0.7rem !important;
}
</style>
