<script setup lang="ts">
import { computed } from 'vue';
import { usePricerStore } from '@/stores/pricer';

const store = usePricerStore();

function getFieldError(fieldName: string): string | undefined {
  return store.validationErrors.find((e) => e.field === fieldName)?.message;
}

const instrumentItems = computed(() => {
  const items: { title: string; value: string }[] = [];
  for (const [, instruments] of Object.entries(store.groupedInstruments)) {
    for (const inst of instruments) {
      const id = inst.instrumentType || inst.id || inst.type || '';
      const name = inst.displayName || inst.name || id || '';
      items.push({ title: name, value: id });
    }
  }
  return items;
});
</script>

<template>
  <div class="d-flex flex-column" style="gap: 12px">
    <!-- Instrument Type -->
    <v-select
      v-model="store.selectedInstrumentId"
      :items="instrumentItems"
      label="Instrument Type"
      placeholder="Select instrument..."
      :error-messages="getFieldError('instrumentType')"
    />

    <!-- Dynamic Parameter Form -->
    <template
      v-if="
        store.selectedInstrument?.requiredParams?.length ||
        store.selectedInstrument?.optionalParams?.length
      "
    >
      <template v-for="param in store.selectedInstrument!.requiredParams" :key="param.name">
        <v-text-field
          v-if="param.fieldType === 'number'"
          v-model.number="store.instrumentParams[param.name]"
          :label="`${param.label || param.name} *`"
          type="number"
          :error-messages="getFieldError(param.name)"
        />

        <v-text-field
          v-else-if="param.fieldType === 'date'"
          v-model="store.instrumentParams[param.name]"
          :label="`${param.label || param.name} *`"
          type="date"
          :error-messages="getFieldError(param.name)"
        />

        <v-select
          v-else-if="param.fieldType === 'select'"
          v-model="store.instrumentParams[param.name]"
          :label="`${param.label || param.name} *`"
          :items="(param.options || []).map((o: any) => ({ title: o.label, value: o.value }))"
          :error-messages="getFieldError(param.name)"
        />

        <v-text-field
          v-else
          v-model="store.instrumentParams[param.name]"
          :label="`${param.label || param.name} *`"
          :error-messages="getFieldError(param.name)"
        />
      </template>
    </template>
  </div>
</template>
