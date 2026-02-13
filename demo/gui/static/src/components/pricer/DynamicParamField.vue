<script setup lang="ts">
import type { ExoticParameterDef } from '@/types/api';

const props = defineProps<{
  param: ExoticParameterDef;
  modelValue: any;
}>();

const emit = defineEmits<{
  'update:modelValue': [value: any];
}>();

function onInput(val: string | number | null) {
  if (props.param.fieldType === 'number') {
    emit('update:modelValue', val === '' || val === null ? null : Number(val));
  } else {
    emit('update:modelValue', val);
  }
}
</script>

<template>
  <v-text-field
    :model-value="modelValue"
    :label="param.displayName"
    :type="param.fieldType === 'number' ? 'number' : 'text'"
    :hint="param.description ?? undefined"
    :persistent-hint="!!param.description"
    :required="param.required"
    variant="outlined"
    density="compact"
    class="mb-1"
    step="any"
    @update:model-value="onInput"
  />
</template>
