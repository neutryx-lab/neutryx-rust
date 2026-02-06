<script setup lang="ts">
import { onMounted } from 'vue';
import { useConfigStore } from '@/stores/config';
import AppLayout from '@/components/common/AppLayout.vue';
import ToastContainer from '@/components/common/ToastContainer.vue';

const configStore = useConfigStore();

onMounted(async () => {
  await configStore.initialize();
});
</script>

<template>
  <AppLayout>
    <RouterView v-slot="{ Component }">
      <Transition name="fade" mode="out-in">
        <component :is="Component" />
      </Transition>
    </RouterView>
  </AppLayout>
  <ToastContainer />
</template>

<style>
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.15s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
