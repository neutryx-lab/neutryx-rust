<template>
  <div class="mb-3">
    <label :for="param.name" class="block text-sm font-medium text-gray-300 mb-1">
      {{ param.displayName }}
      <span v-if="param.required" class="text-red-400">*</span>
    </label>
    <input
      :id="param.name"
      v-model="modelValue"
      :type="inputType"
      :step="param.fieldType === 'number' ? 'any' : undefined"
      :placeholder="param.description || ''"
      class="w-full bg-gray-700 border border-gray-600 rounded-md px-3 py-2 text-sm text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
      @input="$emit('update:modelValue', param.fieldType === 'number' ? Number(($event.target as HTMLInputElement).value) : ($event.target as HTMLInputElement).value)"
    />
    <p v-if="param.description" class="mt-1 text-xs text-gray-500">{{ param.description }}</p>
  </div>
</template>

<script setup lang="ts">
import type { ExoticParameterDef } from '../../types/api';

const props = defineProps<{
  param: ExoticParameterDef;
  modelValue: any;
}>();

defineEmits<{
  'update:modelValue': [value: any];
}>();

const inputType = props.param.fieldType === 'number' ? 'number' : 'text';
</script>
