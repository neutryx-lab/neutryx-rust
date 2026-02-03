<script setup lang="ts">
import { ref } from 'vue';
import { useConfigStore } from '@/stores/config';

const props = defineProps<{
  title: string;
  breadcrumb: string;
}>();

const configStore = useConfigStore();
const searchQuery = ref('');

function toggleTheme() {
  const themes = ['dark', 'light', 'oled'] as const;
  const currentIndex = themes.indexOf(configStore.theme);
  const nextIndex = (currentIndex + 1) % themes.length;
  configStore.setTheme(themes[nextIndex]);
  configStore.persist();
}

function handleSearch() {
  if (searchQuery.value.trim()) {
    console.log('Search:', searchQuery.value);
    // TODO: Implement search functionality
  }
}
</script>

<template>
  <header class="top-bar fixed top-0 right-0 left-[var(--sidebar-width)] h-16 z-40">
    <div class="top-bar-inner h-full flex items-center justify-between px-6 bg-[var(--glass-bg)] backdrop-blur-[20px] border-b border-[var(--glass-border)]">
      <!-- Left: Breadcrumb & Title -->
      <div class="header-left flex items-center gap-4">
        <div class="breadcrumb flex items-center gap-2 text-sm">
          <RouterLink to="/dashboard" class="text-[var(--text-muted)] hover:text-[var(--text-primary)] transition-colors">
            <i class="fas fa-home"></i>
          </RouterLink>
          <i class="fas fa-chevron-right text-[var(--text-muted)] text-xs"></i>
          <span class="text-[var(--text-secondary)]">{{ breadcrumb }}</span>
        </div>
        <h1 class="page-title text-lg font-semibold text-[var(--text-primary)]">
          {{ title }}
        </h1>
      </div>

      <!-- Right: Actions -->
      <div class="header-right flex items-center gap-4">
        <!-- Search -->
        <div class="search-container relative">
          <input
            v-model="searchQuery"
            type="text"
            placeholder="Search... (Ctrl+K)"
            class="search-input w-48 px-4 py-2 pl-10 rounded-lg text-sm
                   bg-[var(--surface)] border border-[var(--glass-border)]
                   text-[var(--text-primary)] placeholder:text-[var(--text-muted)]
                   focus:outline-none focus:ring-2 focus:ring-[var(--primary)] focus:border-transparent
                   transition-all duration-200"
            @keydown.enter="handleSearch"
          />
          <i class="fas fa-search absolute left-3 top-1/2 -translate-y-1/2 text-[var(--text-muted)]"></i>
        </div>

        <!-- Theme Toggle -->
        <button
          class="theme-toggle p-2 rounded-lg text-[var(--text-secondary)]
                 hover:text-[var(--text-primary)] hover:bg-[var(--surface-hover)]
                 transition-all duration-200"
          title="Toggle theme"
          @click="toggleTheme"
        >
          <i v-if="configStore.theme === 'dark'" class="fas fa-moon"></i>
          <i v-else-if="configStore.theme === 'light'" class="fas fa-sun"></i>
          <i v-else class="fas fa-adjust"></i>
        </button>

        <!-- Notifications -->
        <button
          class="notifications-btn p-2 rounded-lg text-[var(--text-secondary)]
                 hover:text-[var(--text-primary)] hover:bg-[var(--surface-hover)]
                 transition-all duration-200"
          title="Notifications"
        >
          <i class="fas fa-bell"></i>
        </button>

        <!-- Settings -->
        <button
          class="settings-btn p-2 rounded-lg text-[var(--text-secondary)]
                 hover:text-[var(--text-primary)] hover:bg-[var(--surface-hover)]
                 transition-all duration-200"
          title="Settings"
        >
          <i class="fas fa-cog"></i>
        </button>
      </div>
    </div>
  </header>
</template>

<style scoped>
.top-bar {
  --sidebar-width: 240px;
}

@media (max-width: 768px) {
  .top-bar {
    left: 0;
  }

  .search-input {
    width: 120px;
  }
}
</style>
