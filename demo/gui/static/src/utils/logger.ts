/**
 * Logger utility for Ergodic Bank Dashboard
 * Integrates with FB_Logger if available, falls back to console.
 */

import type { Logger } from '@/types';

/**
 * Create a logger instance that uses FB_Logger if available.
 */
function createLogger(): Logger {
  if (window.FB_Logger) {
    return window.FB_Logger;
  }

  return {
    debug(component: string, message: string, data?: unknown): void {
      if (window.__FB_CONFIG__?.debugMode) {
        console.debug(`[DEBUG] [${component}] ${message}`, data ?? '');
      }
    },
    info(component: string, message: string, data?: unknown): void {
      console.info(`[INFO] [${component}] ${message}`, data ?? '');
    },
    warn(component: string, message: string, data?: unknown): void {
      console.warn(`[WARN] [${component}] ${message}`, data ?? '');
    },
    error(component: string, message: string, data?: unknown): void {
      console.error(`[ERROR] [${component}] ${message}`, data ?? '');
    },
    isDebugEnabled(): boolean {
      return window.__FB_CONFIG__?.debugMode ?? false;
    },
  };
}

export const logger = createLogger();

/**
 * Create a scoped logger for a specific component.
 */
export function createScopedLogger(component: string) {
  return {
    debug: (message: string, data?: unknown) => logger.debug(component, message, data),
    info: (message: string, data?: unknown) => logger.info(component, message, data),
    warn: (message: string, data?: unknown) => logger.warn(component, message, data),
    error: (message: string, data?: unknown) => logger.error(component, message, data),
  };
}
