<script setup lang="ts">
import { computed, ref } from 'vue';
import { usePricerStore } from '@/stores/pricer';
import { resolveTenor } from '@/services/api';

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

const allParams = computed(() => {
  const inst = store.selectedInstrument;
  if (!inst) return [];
  const required = (inst.requiredParams ?? []).map((p) => ({ ...p, required: true }));
  const optional = (inst.optionalParams ?? []).map((p) => ({ ...p, required: false }));
  return [...required, ...optional];
});

// --- Tenor-first date input ---
// Display text per date field — shows tenor or resolved date
const dateDisplay = ref<Record<string, string>>({});

function getDateDisplay(paramName: string): string {
  return dateDisplay.value[paramName] ?? (store.instrumentParams[paramName] as string) ?? '';
}

function onTenorInput(paramName: string, raw: string) {
  dateDisplay.value[paramName] = raw;
}

async function onTenorCommit(paramName: string) {
  const raw = dateDisplay.value[paramName] ?? '';
  if (!raw) {
    store.instrumentParams[paramName] = '';
    return;
  }
  try {
    const resolved = await resolveTenor(raw);
    store.instrumentParams[paramName] = resolved;
    dateDisplay.value[paramName] = resolved;
  } catch {
    // Invalid input — revert to stored value
    dateDisplay.value[paramName] = (store.instrumentParams[paramName] as string) ?? '';
  }
}

function onCalendarPick(paramName: string, date: string) {
  store.instrumentParams[paramName] = date;
  dateDisplay.value[paramName] = date;
}

async function applyTenorChip(paramName: string, tenor: string) {
  try {
    const resolved = await resolveTenor(tenor);
    store.instrumentParams[paramName] = resolved;
    dateDisplay.value[paramName] = resolved;
  } catch { /* ignore */ }
}
</script>

<template>
  <div class="config-grid">
    <!-- Instrument Type -->
    <div class="grid-label">Type</div>
    <div class="grid-input">
      <v-select
        v-model="store.selectedInstrumentId"
        :items="instrumentItems"
        placeholder="Select..."
        density="compact"
        variant="outlined"
        hide-details="auto"
        :error-messages="getFieldError('instrumentType')"
      />
    </div>

    <!-- Dynamic params in grid rows -->
    <template v-for="param in allParams" :key="param.name">
      <div class="grid-label" :class="{ required: param.required }">
        {{ param.label || param.name }}
      </div>
      <div class="grid-input">
        <!-- Number field -->
        <v-text-field
          v-if="param.fieldType === 'number'"
          v-model.number="store.instrumentParams[param.name]"
          type="number"
          density="compact"
          variant="outlined"
          hide-details="auto"
          :error-messages="getFieldError(param.name)"
        />

        <!-- Date field: tenor text input (default) + calendar picker -->
        <v-text-field
          v-else-if="param.fieldType === 'date'"
          :model-value="getDateDisplay(param.name)"
          density="compact"
          variant="outlined"
          hide-details="auto"
          placeholder="5Y, 3M, TD or YYYY-MM-DD"
          :error-messages="getFieldError(param.name)"
          @update:model-value="(v: string) => onTenorInput(param.name, v)"
          @blur="onTenorCommit(param.name)"
          @keydown.enter="onTenorCommit(param.name)"
        >
          <template #prepend-inner>
            <div class="d-flex" style="gap: 1px">
              <v-btn
                v-for="t in ['TD', '1Y', '5Y']"
                :key="t"
                size="x-small"
                variant="text"
                density="compact"
                class="tenor-chip"
                @click="applyTenorChip(param.name, t)"
              >
                {{ t }}
              </v-btn>
            </div>
          </template>
          <template #append-inner>
            <v-menu :close-on-content-click="false" location="bottom end">
              <template #activator="{ props: menuProps }">
                <v-btn
                  v-bind="menuProps"
                  icon
                  density="compact"
                  variant="text"
                  size="x-small"
                  tabindex="-1"
                >
                  <v-icon size="14">mdi-calendar</v-icon>
                </v-btn>
              </template>
              <v-date-picker
                :model-value="(store.instrumentParams[param.name] as string) ?? undefined"
                @update:model-value="(v: any) => onCalendarPick(param.name, v)"
              />
            </v-menu>
          </template>
        </v-text-field>

        <!-- Select field -->
        <v-select
          v-else-if="param.fieldType === 'select'"
          v-model="store.instrumentParams[param.name]"
          :items="(param.options || []).map((o: any) => ({ title: o.label, value: o.value }))"
          density="compact"
          variant="outlined"
          hide-details="auto"
          :error-messages="getFieldError(param.name)"
        />

        <!-- Default text field -->
        <v-text-field
          v-else
          v-model="store.instrumentParams[param.name]"
          density="compact"
          variant="outlined"
          hide-details="auto"
          :error-messages="getFieldError(param.name)"
        />
      </div>
    </template>
  </div>
</template>

