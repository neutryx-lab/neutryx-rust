/**
 * FrictionalBank Dashboard - Main Entry Point
 * TypeScript + Vite build
 */

import { ConfigLoader } from '@/services/config-loader';
import { initPricer, initMarketData, initCurveBuilder, initVolcubeBuilder } from '@/components';
import { createScopedLogger } from '@/utils/logger';

const log = createScopedLogger('App');

// =============================================================================
// Global Configuration
// =============================================================================

// Types are defined in @/types/index.ts
// Initialize global config
window.__FB_CONFIG__ = {
  debugMode: false,
  logLevel: 'INFO',
};

// =============================================================================
// View Management
// =============================================================================

type ViewId =
  | 'dashboard-view'
  | 'market-data-view'
  | 'curve-builder-view'
  | 'volcube-calibration-view'
  | 'pricer-view';

const viewInitializers: Record<ViewId, () => Promise<void>> = {
  'dashboard-view': async () => {
    log.debug('Dashboard view activated');
  },
  'market-data-view': async () => {
    await initMarketData();
  },
  'curve-builder-view': async () => {
    await initCurveBuilder();
  },
  'volcube-calibration-view': async () => {
    await initVolcubeBuilder();
  },
  'pricer-view': async () => {
    await initPricer();
  },
};

// Track current view for state management (exported for external access)
let currentView: ViewId | null = null;

export function getCurrentView(): ViewId | null {
  return currentView;
}

function navigateTo(viewId: string): void {
  const views = document.querySelectorAll('.view');
  const navItems = document.querySelectorAll('.nav-item');

  views.forEach((view) => {
    view.classList.remove('active');
    if (view.id === viewId) {
      view.classList.add('active');
    }
  });

  navItems.forEach((item) => {
    item.classList.remove('active');
    if ((item as HTMLElement).dataset.view === viewId) {
      item.classList.add('active');
    }
  });

  currentView = viewId as ViewId;

  // Initialize view if needed
  const initializer = viewInitializers[viewId as ViewId];
  if (initializer) {
    void initializer();
  }

  // Dispatch custom event for view change
  window.dispatchEvent(new CustomEvent('viewChanged', { detail: { view: viewId } }));

  log.debug(`Navigated to: ${viewId}`);
}

// Expose navigateTo globally
window.navigateTo = navigateTo;

// =============================================================================
// Toast Notifications
// =============================================================================

function showToast(
  message: string,
  type: 'success' | 'error' | 'warning' | 'info' = 'info'
): void {
  const container = document.getElementById('toast-container');
  if (!container) return;

  const toast = document.createElement('div');
  toast.className = `toast toast-${type}`;

  const icons: Record<string, string> = {
    success: 'fa-check-circle',
    error: 'fa-exclamation-circle',
    warning: 'fa-exclamation-triangle',
    info: 'fa-info-circle',
  };

  toast.innerHTML = `
    <i class="fas ${icons[type]}"></i>
    <span>${message}</span>
    <button class="toast-close"><i class="fas fa-times"></i></button>
  `;

  container.appendChild(toast);

  // Auto-remove after 5 seconds
  setTimeout(() => {
    toast.classList.add('fade-out');
    setTimeout(() => toast.remove(), 300);
  }, 5000);

  // Manual close
  toast.querySelector('.toast-close')?.addEventListener('click', () => {
    toast.remove();
  });
}

// Expose showToast globally
window.showToast = showToast;

// =============================================================================
// Navigation Setup
// =============================================================================

function setupNavigation(): void {
  document.querySelectorAll('.nav-item').forEach((item) => {
    item.addEventListener('click', () => {
      const viewId = (item as HTMLElement).dataset.view;
      if (viewId) {
        navigateTo(viewId);
      }
    });
  });

  // Handle keyboard navigation
  document.addEventListener('keydown', (e) => {
    // Cmd/Ctrl + K for command palette (if implemented)
    if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
      e.preventDefault();
      log.debug('Command palette triggered');
    }
  });
}

// =============================================================================
// Application Initialization
// =============================================================================

async function initializeApp(): Promise<void> {
  log.info('Initializing FrictionalBank Dashboard...');

  try {
    // Load configuration
    await ConfigLoader.load();
    log.info('Configuration loaded');

    // Setup navigation
    setupNavigation();

    // Navigate to default view
    const defaultView = 'dashboard-view';
    const activeView = document.querySelector('.view.active');
    if (activeView) {
      navigateTo(activeView.id);
    } else {
      navigateTo(defaultView);
    }

    log.info('Application initialized successfully');
  } catch (error) {
    log.error('Failed to initialize application', error);
    showToast('Failed to initialize application', 'error');
  }
}

// =============================================================================
// DOM Ready
// =============================================================================

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', () => void initializeApp());
} else {
  void initializeApp();
}

// =============================================================================
// Exports for Module System
// =============================================================================

export { navigateTo, showToast, ConfigLoader };
