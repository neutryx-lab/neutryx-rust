<script setup lang="ts">
export interface TabItem {
  key: string;
  label: string;
  icon?: string;
}

defineProps<{
  tabs: TabItem[];
  modelValue: string;
}>();

const emit = defineEmits<{
  'update:modelValue': [value: string];
}>();
</script>

<template>
  <div class="flex gap-2">
    <button
      v-for="tab in tabs"
      :key="tab.key"
      class="tab-btn"
      :class="{ active: modelValue === tab.key }"
      @click="emit('update:modelValue', tab.key)"
    >
      <i v-if="tab.icon" :class="['fas', tab.icon]"></i>
      {{ tab.label }}
    </button>
  </div>
</template>
