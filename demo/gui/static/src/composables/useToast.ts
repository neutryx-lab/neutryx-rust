import { ref, readonly } from 'vue';

export type ToastType = 'success' | 'error' | 'warning' | 'info';

export interface Toast {
  id: number;
  message: string;
  type: ToastType;
  duration: number;
}

const toasts = ref<Toast[]>([]);
let nextId = 1;

export function useToast() {
  function show(message: string, type: ToastType = 'info', duration = 5000): number {
    const id = nextId++;
    const toast: Toast = { id, message, type, duration };

    toasts.value.push(toast);

    // Auto-remove after duration
    if (duration > 0) {
      setTimeout(() => {
        dismiss(id);
      }, duration);
    }

    return id;
  }

  function dismiss(id: number) {
    const index = toasts.value.findIndex((t) => t.id === id);
    if (index !== -1) {
      toasts.value.splice(index, 1);
    }
  }

  function dismissAll() {
    toasts.value = [];
  }

  // Convenience methods
  function success(message: string, duration?: number) {
    return show(message, 'success', duration);
  }

  function error(message: string, duration?: number) {
    return show(message, 'error', duration);
  }

  function warning(message: string, duration?: number) {
    return show(message, 'warning', duration);
  }

  function info(message: string, duration?: number) {
    return show(message, 'info', duration);
  }

  return {
    toasts: readonly(toasts),
    show,
    dismiss,
    dismissAll,
    success,
    error,
    warning,
    info,
  };
}

// Global instance for use outside of Vue components
const globalToast = useToast();

// Expose globally for compatibility with legacy code
if (typeof window !== 'undefined') {
  (window as unknown as { showToast: typeof globalToast.show }).showToast = globalToast.show;
}

export { globalToast };
