<script setup lang="ts">
import { computed } from 'vue';
import { useRoute } from 'vue-router';
import Sidebar from './Sidebar.vue';
import TopBar from './TopBar.vue';
import type { ViewMeta } from '@/router';

const route = useRoute();

const currentMeta = computed<ViewMeta | undefined>(() => {
  return route.meta as ViewMeta | undefined;
});
</script>

<template>
  <div class="app-layout min-h-screen bg-[var(--bg-primary)]">
    <!-- Sidebar -->
    <Sidebar />

    <!-- Main Content Area -->
    <div class="main-content ml-[var(--sidebar-width)]">
      <!-- Top Bar -->
      <TopBar
        :title="currentMeta?.title ?? 'Dashboard'"
        :breadcrumb="currentMeta?.breadcrumb ?? 'Overview'"
      />

      <!-- Page Content -->
      <main class="page-content p-6">
        <slot />
      </main>
    </div>
  </div>
</template>

<style scoped>
.app-layout {
  --sidebar-width: 240px;
}

.main-content {
  min-height: 100vh;
  transition: margin-left var(--transition-normal);
}

.page-content {
  padding-top: calc(var(--header-height, 64px) + 1.5rem);
}

/* Responsive: Collapse sidebar on mobile */
@media (max-width: 768px) {
  .app-layout {
    --sidebar-width: 0px;
  }
}
</style>
