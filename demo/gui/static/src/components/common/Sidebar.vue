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

// Accordion state for sections
const analyticsExpanded = ref(true);
const toolsExpanded = ref(true);

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
        <RouterLink to="/dashboard" class="logo-link group flex items-center gap-3 text-[var(--text-primary)] no-underline">
          <div class="logo-icon w-10 h-10 rounded-xl"></div>
          <div class="logo-text leading-tight">
            <div>
              <span class="font-semibold text-base tracking-wide">Frictional</span>
              <span class="text-[var(--text-muted)] font-light text-[11px] tracking-[0.15em] uppercase">Bank</span>
              <sup class="text-[9px] font-medium text-emerald-400 ml-0.5 tracking-wider">DEMO</sup>
            </div>
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

        <!-- Tools Section (Accordion) -->
        <div class="mt-4">
          <button
            class="accordion-header w-full flex items-center justify-between px-6 py-2 text-xs font-semibold uppercase tracking-wider text-[var(--text-muted)]"
            @click="toolsExpanded = !toolsExpanded"
          >
            <span>Tools</span>
            <i :class="['fas fa-chevron-down transition-transform duration-200', { 'rotate-180': !toolsExpanded }]"></i>
          </button>

          <Transition name="accordion">
            <ul v-show="toolsExpanded" class="nav-list px-3 space-y-1">
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
          </Transition>
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

/* Logo — animated geometric pattern */
.logo-icon {
  position: relative;
  background: linear-gradient(135deg, var(--primary), var(--primary-dark));
  box-shadow: 0 4px 12px rgb(99 102 241 / .25);
  transition: box-shadow 0.3s;
  overflow: hidden;
}
.logo-link:hover .logo-icon { box-shadow: 0 4px 24px rgb(99 102 241 / .45); }

.logo-icon::before, .logo-icon::after {
  content: ''; position: absolute; inset: 0; border-radius: inherit; transition: filter 0.3s;
}
.logo-icon::before {
  background: conic-gradient(from 0deg, transparent, rgb(255 255 255 / .45) 30deg, transparent 60deg, rgb(255 255 255 / .3) 120deg, transparent 150deg, rgb(255 255 255 / .2) 210deg, transparent 240deg, rgb(255 255 255 / .35) 300deg, transparent);
  animation: geo-spin 6s linear infinite;
}
.logo-icon::after {
  background: conic-gradient(from 90deg at 40% 60%, transparent, rgb(255 255 255 / .25) 40deg, transparent 80deg, rgb(255 255 255 / .4) 160deg, transparent 200deg, rgb(255 255 255 / .15) 280deg, transparent);
  animation: geo-spin 10s linear infinite reverse;
}
.logo-link:hover .logo-icon::before { animation-duration: 2s; filter: brightness(1.4); }
.logo-link:hover .logo-icon::after  { animation-duration: 3.5s; filter: brightness(1.4); }
@keyframes geo-spin { to { transform: rotate(360deg); } }

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
