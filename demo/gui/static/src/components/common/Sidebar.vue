<script setup lang="ts">
import { computed, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { getRoutesByGroup, type ViewMeta } from '@/router';

const route = useRoute();
const router = useRouter();

// Navigation groups
const mainNavItems = computed(() =>
  getRoutesByGroup('main').map((r) => ({
    name: r.name as string,
    path: r.path,
    ...(r.meta as ViewMeta),
  }))
);

const analyticsNavItems = computed(() =>
  getRoutesByGroup('analytics').map((r) => ({
    name: r.name as string,
    path: r.path,
    ...(r.meta as ViewMeta),
  }))
);

const toolsNavItems = computed(() =>
  getRoutesByGroup('tools').map((r) => ({
    name: r.name as string,
    path: r.path,
    ...(r.meta as ViewMeta),
  }))
);

// Accordion state for analytics section
const analyticsExpanded = ref(true);

function isActive(path: string): boolean {
  return route.path === path;
}

function navigateTo(name: string) {
  router.push({ name });
}
</script>

<template>
  <aside class="sidebar fixed top-0 left-0 h-full w-[var(--sidebar-width)] z-50">
    <div class="sidebar-inner h-full flex flex-col bg-[var(--glass-bg)] backdrop-blur-[20px] border-r border-[var(--glass-border)]">
      <!-- Logo -->
      <div class="logo-container p-4 border-b border-[var(--glass-border)]">
        <RouterLink to="/dashboard" class="logo flex items-center gap-3 text-[var(--text-primary)] no-underline">
          <div class="logo-icon w-10 h-10 rounded-xl bg-gradient-to-br from-[var(--primary)] to-[var(--primary-dark)] flex items-center justify-center">
            <i class="fas fa-chart-line text-white text-lg"></i>
          </div>
          <div class="logo-text">
            <span class="font-semibold text-base">Frictional</span>
            <span class="text-[var(--text-secondary)] font-light text-sm">Bank</span>
          </div>
        </RouterLink>
      </div>

      <!-- Navigation -->
      <nav class="flex-1 overflow-y-auto py-4">
        <!-- Main Navigation -->
        <ul class="nav-list px-3 space-y-1">
          <li
            v-for="item in mainNavItems"
            :key="item.name"
            class="nav-item"
          >
            <button
              :class="[
                'nav-link w-full flex items-center gap-3 px-3 py-2.5 rounded-lg transition-all duration-200',
                'text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--surface-hover)]',
                { 'active bg-[var(--surface)] text-[var(--primary)]': isActive(item.path) }
              ]"
              @click="navigateTo(item.name)"
            >
              <i :class="['fas', item.icon, 'w-5 text-center']"></i>
              <span class="text-sm font-medium">{{ item.title }}</span>
            </button>
          </li>
        </ul>

        <!-- Analytics Section (Accordion) -->
        <div class="mt-4">
          <button
            class="accordion-header w-full flex items-center justify-between px-6 py-2 text-xs font-semibold uppercase tracking-wider text-[var(--text-muted)]"
            @click="analyticsExpanded = !analyticsExpanded"
          >
            <span>Analytics</span>
            <i :class="['fas fa-chevron-down transition-transform duration-200', { 'rotate-180': !analyticsExpanded }]"></i>
          </button>

          <Transition name="accordion">
            <ul v-show="analyticsExpanded" class="nav-list px-3 space-y-1">
              <li
                v-for="item in analyticsNavItems"
                :key="item.name"
                class="nav-item"
              >
                <button
                  :class="[
                    'nav-link w-full flex items-center gap-3 px-3 py-2.5 rounded-lg transition-all duration-200',
                    'text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--surface-hover)]',
                    { 'active bg-[var(--surface)] text-[var(--primary)]': isActive(item.path) }
                  ]"
                  @click="navigateTo(item.name)"
                >
                  <i :class="['fas', item.icon, 'w-5 text-center']"></i>
                  <span class="text-sm font-medium">{{ item.title }}</span>
                </button>
              </li>
            </ul>
          </Transition>
        </div>

        <!-- Tools Section -->
        <div class="mt-4">
          <div class="section-header px-6 py-2 text-xs font-semibold uppercase tracking-wider text-[var(--text-muted)]">
            Tools
          </div>
          <ul class="nav-list px-3 space-y-1">
            <li
              v-for="item in toolsNavItems"
              :key="item.name"
              class="nav-item"
            >
              <button
                :class="[
                  'nav-link w-full flex items-center gap-3 px-3 py-2.5 rounded-lg transition-all duration-200',
                  'text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--surface-hover)]',
                  { 'active bg-[var(--surface)] text-[var(--primary)]': isActive(item.path) }
                ]"
                @click="navigateTo(item.name)"
              >
                <i :class="['fas', item.icon, 'w-5 text-center']"></i>
                <span class="text-sm font-medium">{{ item.title }}</span>
              </button>
            </li>
          </ul>
        </div>
      </nav>

      <!-- Footer -->
      <div class="sidebar-footer p-4 border-t border-[var(--glass-border)]">
        <div class="text-xs text-[var(--text-muted)] text-center">
          Neutryx v0.1.0
        </div>
      </div>
    </div>
  </aside>
</template>

<style scoped>
.sidebar {
  --sidebar-width: 240px;
}

.nav-link.active {
  background: var(--surface);
  color: var(--primary);
}

.nav-link.active i {
  color: var(--primary);
}

/* Accordion transition */
.accordion-enter-active,
.accordion-leave-active {
  transition: all 0.2s ease;
  overflow: hidden;
}

.accordion-enter-from,
.accordion-leave-to {
  opacity: 0;
  max-height: 0;
}

.accordion-enter-to,
.accordion-leave-from {
  opacity: 1;
  max-height: 300px;
}
</style>
