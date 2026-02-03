<script setup lang="ts">
import { useToast, type ToastType } from '@/composables/useToast';

const { toasts, dismiss } = useToast();

const icons: Record<ToastType, string> = {
  success: 'fa-check-circle',
  error: 'fa-exclamation-circle',
  warning: 'fa-exclamation-triangle',
  info: 'fa-info-circle',
};

const colors: Record<ToastType, string> = {
  success: 'border-l-[var(--success)]',
  error: 'border-l-[var(--danger)]',
  warning: 'border-l-[var(--warning)]',
  info: 'border-l-[var(--primary)]',
};

const iconColors: Record<ToastType, string> = {
  success: 'text-[var(--success)]',
  error: 'text-[var(--danger)]',
  warning: 'text-[var(--warning)]',
  info: 'text-[var(--primary)]',
};
</script>

<template>
  <Teleport to="body">
    <div class="toast-container fixed bottom-6 right-6 z-[9999] flex flex-col gap-3">
      <TransitionGroup name="toast">
        <div
          v-for="toast in toasts"
          :key="toast.id"
          :class="[
            'toast flex items-center gap-3 px-4 py-3 rounded-lg border-l-4',
            'bg-[var(--glass-bg)] backdrop-blur-[20px] border border-[var(--glass-border)]',
            'shadow-lg min-w-[280px] max-w-[400px]',
            colors[toast.type]
          ]"
        >
          <i :class="['fas', icons[toast.type], iconColors[toast.type]]"></i>
          <span class="flex-1 text-sm text-[var(--text-primary)]">{{ toast.message }}</span>
          <button
            class="toast-close p-1 rounded hover:bg-[var(--surface-hover)] text-[var(--text-muted)] hover:text-[var(--text-primary)] transition-colors"
            @click="dismiss(toast.id)"
          >
            <i class="fas fa-times text-xs"></i>
          </button>
        </div>
      </TransitionGroup>
    </div>
  </Teleport>
</template>

<style scoped>
.toast-enter-active,
.toast-leave-active {
  transition: all 0.3s ease;
}

.toast-enter-from {
  opacity: 0;
  transform: translateX(100%);
}

.toast-leave-to {
  opacity: 0;
  transform: translateX(100%);
}

.toast-move {
  transition: transform 0.3s ease;
}
</style>
