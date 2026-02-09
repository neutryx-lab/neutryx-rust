<script setup lang="ts">
import { usePricerStore } from '@/stores/pricer';

const store = usePricerStore();

function getFieldError(fieldName: string): string | undefined {
  return store.validationErrors.find((e) => e.field === fieldName)?.message;
}

function fieldClass(fieldName: string): string {
  const base =
    'w-full px-4 py-2.5 rounded-lg bg-[var(--surface)] text-[var(--text-primary)] focus:outline-none focus:ring-2';
  return getFieldError(fieldName)
    ? `${base} border-2 border-[var(--danger)] focus:ring-[var(--danger)]`
    : `${base} border border-[var(--glass-border)] focus:ring-[var(--primary)]`;
}
</script>

<template>
  <div class="glass-card p-6">
    <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Trade Setup</h3>
    <div class="space-y-4">
      <!-- Instrument Type Dropdown -->
      <div>
        <label class="block text-sm text-[var(--text-muted)] mb-2">Instrument Type</label>
        <select v-model="store.selectedInstrumentId" :class="fieldClass('instrumentType')">
          <option value="">Select instrument...</option>
          <optgroup
            v-for="(items, group) in store.groupedInstruments"
            :key="group"
            :label="group"
          >
            <option
              v-for="inst in items"
              :key="inst.instrumentType || inst.id || inst.type"
              :value="inst.instrumentType || inst.id || inst.type"
            >
              {{ inst.displayName || inst.name || inst.instrumentType || inst.id }}
            </option>
          </optgroup>
        </select>
        <p v-if="getFieldError('instrumentType')" class="text-xs text-[var(--danger)] mt-1">
          {{ getFieldError('instrumentType') }}
        </p>
      </div>

      <!-- Dynamic Parameter Form -->
      <template
        v-if="
          store.selectedInstrument?.requiredParams?.length ||
          store.selectedInstrument?.optionalParams?.length
        "
      >
        <div
          v-for="param in store.selectedInstrument!.requiredParams"
          :key="param.name"
          class="form-field"
        >
          <label class="block text-sm text-[var(--text-muted)] mb-2">
            {{ param.label || param.name }} <span class="text-red-500">*</span>
          </label>

          <input
            v-if="param.fieldType === 'number'"
            type="number"
            v-model.number="store.instrumentParams[param.name]"
            :class="fieldClass(param.name)"
          />
          <input
            v-else-if="param.fieldType === 'date'"
            type="date"
            v-model="store.instrumentParams[param.name]"
            :class="fieldClass(param.name)"
          />
          <select
            v-else-if="param.fieldType === 'select'"
            v-model="store.instrumentParams[param.name]"
            :class="fieldClass(param.name)"
          >
            <option v-for="opt in param.options" :key="opt.value" :value="opt.value">
              {{ opt.label }}
            </option>
          </select>
          <input
            v-else
            type="text"
            v-model="store.instrumentParams[param.name]"
            :class="fieldClass(param.name)"
          />

          <p v-if="getFieldError(param.name)" class="text-xs text-[var(--danger)] mt-1">
            {{ getFieldError(param.name) }}
          </p>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.glass-card {
  background: var(--glass-bg);
  backdrop-filter: blur(20px);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-lg);
  box-shadow: var(--glass-shadow);
}
</style>
