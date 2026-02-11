<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { useConfigStore } from '@/stores/config';
import { useToast } from '@/composables/useToast';

const props = defineProps<{
  title: string;
  breadcrumb: string;
}>();

const configStore = useConfigStore();
const toast = useToast();
const searchQuery = ref('');

// Dropdown/Modal states
const showNotifications = ref(false);
const showSettings = ref(false);

// Sample notifications data
interface Notification {
  id: number;
  type: 'info' | 'warning' | 'success' | 'error';
  title: string;
  time: string;
  read: boolean;
}

const notifications = ref<Notification[]>([
  { id: 1, type: 'info', title: 'Market data updated', time: '2 min ago', read: false },
  { id: 2, type: 'warning', title: 'Curve calibration warning', time: '15 min ago', read: false },
  { id: 3, type: 'success', title: 'Portfolio loaded', time: '1 hour ago', read: true },
  { id: 4, type: 'error', title: 'Connection timeout', time: '2 hours ago', read: true },
]);

const unreadCount = ref(notifications.value.filter(n => !n.read).length);

function toggleTheme() {
  const themes = ['dark', 'light', 'oled'] as const;
  const currentIndex = themes.indexOf(configStore.theme);
  const nextIndex = (currentIndex + 1) % themes.length;
  configStore.setTheme(themes[nextIndex]);
  configStore.persist();
  toast.success(`Theme changed to ${themes[nextIndex]}`);
}

function handleSearch() {
  if (searchQuery.value.trim()) {
    toast.info(`Searching for "${searchQuery.value}"...`);
    // TODO: Implement search functionality
  }
}

function toggleNotifications() {
  showNotifications.value = !showNotifications.value;
  showSettings.value = false;
}

function toggleSettings() {
  showSettings.value = !showSettings.value;
  showNotifications.value = false;
}

function closeDropdowns(event: MouseEvent) {
  const target = event.target as HTMLElement;
  if (!target.closest('.notifications-container') && !target.closest('.settings-container')) {
    showNotifications.value = false;
    showSettings.value = false;
  }
}

function markAllRead() {
  notifications.value.forEach(n => n.read = true);
  unreadCount.value = 0;
  toast.success('All notifications marked as read');
}

function handleNotificationClick(notif: Notification) {
  if (!notif.read) {
    notif.read = true;
    unreadCount.value = notifications.value.filter(n => !n.read).length;
  }
  toast.info(notif.title);
  showNotifications.value = false;
}

function viewAllNotifications() {
  showNotifications.value = false;
  toast.info('Notifications page coming soon');
}

function handleSettingClick(setting: string) {
  showSettings.value = false;
  switch (setting) {
    case 'theme':
      toggleTheme();
      break;
    case 'language':
      toast.info('Language settings coming soon');
      break;
    case 'datasource':
      toast.info('Data source configuration coming soon');
      break;
    case 'about':
      toast.info('Neutryx v0.1.0 - Derivatives Pricing Library');
      break;
  }
}

function getNotificationIcon(type: string): string {
  switch (type) {
    case 'info': return 'fa-info-circle text-blue-400';
    case 'warning': return 'fa-exclamation-triangle text-yellow-400';
    case 'success': return 'fa-check-circle text-green-400';
    case 'error': return 'fa-times-circle text-red-400';
    default: return 'fa-bell text-gray-400';
  }
}

onMounted(() => {
  document.addEventListener('click', closeDropdowns);
});

onUnmounted(() => {
  document.removeEventListener('click', closeDropdowns);
});
</script>

<template>
  <header class="top-bar fixed top-0 right-0 left-[var(--sidebar-width)] h-16 z-40">
    <div class="top-bar-inner h-full flex items-center justify-between px-6 bg-[var(--glass-bg)] backdrop-blur-[20px] border-b border-[var(--glass-border)]">
      <!-- Left: Title -->
      <div class="header-left flex items-center gap-3">
        <RouterLink to="/dashboard" class="text-[var(--text-muted)] hover:text-[var(--text-primary)] transition-colors">
          <i class="fas fa-home"></i>
        </RouterLink>
        <i class="fas fa-chevron-right text-[var(--text-muted)] text-xs"></i>
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

        <!-- Rust Docs -->
        <a
          href="/doc/neutryx/index.html"
          target="_blank"
          rel="noopener noreferrer"
          class="docs-btn p-2 rounded-lg text-[var(--text-secondary)]
                 hover:text-[var(--text-primary)] hover:bg-[var(--surface-hover)]
                 transition-all duration-200"
          title="Rust Documentation"
        >
          <i class="fas fa-book"></i>
        </a>

        <!-- Notifications -->
        <div class="notifications-container relative">
          <button
            class="notifications-btn p-2 rounded-lg text-[var(--text-secondary)]
                   hover:text-[var(--text-primary)] hover:bg-[var(--surface-hover)]
                   transition-all duration-200 relative"
            title="Notifications"
            @click.stop="toggleNotifications"
          >
            <i class="fas fa-bell"></i>
            <span
              v-if="unreadCount > 0"
              class="absolute -top-1 -right-1 w-4 h-4 rounded-full bg-red-500 text-white text-xs flex items-center justify-center"
            >
              {{ unreadCount }}
            </span>
          </button>

          <!-- Notifications Dropdown -->
          <Transition name="dropdown">
            <div
              v-if="showNotifications"
              class="notifications-dropdown absolute right-0 top-full mt-2 w-80 rounded-xl
                     bg-[var(--surface)] border border-[var(--glass-border)]
                     shadow-xl overflow-hidden z-50"
            >
              <div class="dropdown-header flex items-center justify-between px-4 py-3 border-b border-[var(--glass-border)]">
                <span class="font-semibold text-[var(--text-primary)]">Notifications</span>
                <button
                  v-if="unreadCount > 0"
                  class="text-xs text-[var(--primary)] hover:underline"
                  @click="markAllRead"
                >
                  Mark all read
                </button>
              </div>
              <ul class="dropdown-list max-h-80 overflow-y-auto">
                <li
                  v-for="notif in notifications"
                  :key="notif.id"
                  :class="[
                    'notification-item flex items-start gap-3 px-4 py-3 border-b border-[var(--glass-border)] last:border-b-0',
                    'hover:bg-[var(--surface-hover)] transition-colors cursor-pointer',
                    { 'bg-[var(--surface-hover)]/50': !notif.read }
                  ]"
                  @click="handleNotificationClick(notif)"
                >
                  <i :class="['fas', getNotificationIcon(notif.type), 'mt-0.5']"></i>
                  <div class="flex-1 min-w-0">
                    <p class="text-sm text-[var(--text-primary)] truncate">{{ notif.title }}</p>
                    <span class="text-xs text-[var(--text-muted)]">{{ notif.time }}</span>
                  </div>
                  <span v-if="!notif.read" class="w-2 h-2 rounded-full bg-[var(--primary)] mt-1.5"></span>
                </li>
              </ul>
              <div class="dropdown-footer px-4 py-3 border-t border-[var(--glass-border)] text-center">
                <button class="text-sm text-[var(--primary)] hover:underline" @click="viewAllNotifications">
                  View all notifications
                </button>
              </div>
            </div>
          </Transition>
        </div>

        <!-- Settings -->
        <div class="settings-container relative">
          <button
            class="settings-btn p-2 rounded-lg text-[var(--text-secondary)]
                   hover:text-[var(--text-primary)] hover:bg-[var(--surface-hover)]
                   transition-all duration-200"
            title="Settings"
            @click.stop="toggleSettings"
          >
            <i class="fas fa-cog"></i>
          </button>

          <!-- Settings Dropdown -->
          <Transition name="dropdown">
            <div
              v-if="showSettings"
              class="settings-dropdown absolute right-0 top-full mt-2 w-64 rounded-xl
                     bg-[var(--surface)] border border-[var(--glass-border)]
                     shadow-xl overflow-hidden z-50"
            >
              <div class="dropdown-header px-4 py-3 border-b border-[var(--glass-border)]">
                <span class="font-semibold text-[var(--text-primary)]">Settings</span>
              </div>
              <ul class="dropdown-list">
                <li class="setting-item px-4 py-3 border-b border-[var(--glass-border)] hover:bg-[var(--surface-hover)] transition-colors cursor-pointer" @click="handleSettingClick('theme')">
                  <div class="flex items-center justify-between">
                    <div class="flex items-center gap-3">
                      <i class="fas fa-palette text-[var(--text-muted)]"></i>
                      <span class="text-sm text-[var(--text-primary)]">Theme</span>
                    </div>
                    <span class="text-xs text-[var(--text-muted)] capitalize">{{ configStore.theme }}</span>
                  </div>
                </li>
                <li class="setting-item px-4 py-3 border-b border-[var(--glass-border)] hover:bg-[var(--surface-hover)] transition-colors cursor-pointer" @click="handleSettingClick('language')">
                  <div class="flex items-center gap-3">
                    <i class="fas fa-language text-[var(--text-muted)]"></i>
                    <span class="text-sm text-[var(--text-primary)]">Language</span>
                  </div>
                </li>
                <li class="setting-item px-4 py-3 border-b border-[var(--glass-border)] hover:bg-[var(--surface-hover)] transition-colors cursor-pointer" @click="handleSettingClick('datasource')">
                  <div class="flex items-center gap-3">
                    <i class="fas fa-database text-[var(--text-muted)]"></i>
                    <span class="text-sm text-[var(--text-primary)]">Data Source</span>
                  </div>
                </li>
                <li class="setting-item px-4 py-3 border-b border-[var(--glass-border)] hover:bg-[var(--surface-hover)] transition-colors cursor-pointer" @click="handleSettingClick('about')">
                  <div class="flex items-center gap-3">
                    <i class="fas fa-info-circle text-[var(--text-muted)]"></i>
                    <span class="text-sm text-[var(--text-primary)]">About</span>
                  </div>
                </li>
                <li class="setting-item px-4 py-3 hover:bg-[var(--surface-hover)] transition-colors cursor-pointer">
                  <a href="https://github.com/neutryx-lab/neutryx-rust" target="_blank" rel="noopener noreferrer" class="flex items-center gap-3 no-underline">
                    <i class="fab fa-github text-[var(--text-muted)]"></i>
                    <span class="text-sm text-[var(--text-primary)]">GitHub</span>
                    <i class="fas fa-external-link-alt text-[10px] text-[var(--text-muted)] ml-auto"></i>
                  </a>
                </li>
              </ul>
            </div>
          </Transition>
        </div>
      </div>
    </div>
  </header>
</template>

<style scoped>
.top-bar {
  --sidebar-width: 240px;
}

/* Dropdown transitions */
.dropdown-enter-active,
.dropdown-leave-active {
  transition: all 0.2s ease;
}

.dropdown-enter-from,
.dropdown-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}

.dropdown-enter-to,
.dropdown-leave-from {
  opacity: 1;
  transform: translateY(0);
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
