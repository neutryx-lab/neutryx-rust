/**
 * FrictionalBank Dashboard - Main Entry Point
 * TypeScript + Vite build
 */

// Import global styles
import '../../style.css';

import { ConfigLoader } from '@/services/config-loader';
import { initPricer, initMarketData, initCurveBuilder, initVolcubeBuilder, initExposure, initDashboard, initScenarios, initTradeExpansion, initPortfolio } from '@/components';
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
  | 'portfolio-view'
  | 'risk-view'
  | 'exposure-view'
  | 'scenarios-view'
  | 'market-data-view'
  | 'trade-expansion-view'
  | 'curve-builder-view'
  | 'volcube-calibration-view'
  | 'pricer-view'
  | 'graph-view';

// View titles for header/breadcrumb
const viewTitles: Record<ViewId, { title: string; breadcrumb: string }> = {
  'dashboard-view': { title: 'Dashboard', breadcrumb: 'Overview' },
  'portfolio-view': { title: 'Portfolio', breadcrumb: 'Portfolio Management' },
  'risk-view': { title: 'Risk', breadcrumb: 'Risk Analytics' },
  'exposure-view': { title: 'Exposure', breadcrumb: 'Exposure Analysis' },
  'scenarios-view': { title: 'Scenarios', breadcrumb: 'Scenario Analysis' },
  'market-data-view': { title: 'Market Data', breadcrumb: 'Market Data' },
  'trade-expansion-view': { title: 'Trade Expansion', breadcrumb: 'Trade Expansion' },
  'curve-builder-view': { title: 'Curve Builder', breadcrumb: 'Curve Builder' },
  'volcube-calibration-view': { title: 'Vol Cube', breadcrumb: 'Vol Cube Calibration' },
  'pricer-view': { title: 'Pricer', breadcrumb: 'Instrument Pricer' },
  'graph-view': { title: 'Graph', breadcrumb: 'Computation Graph' },
};

const viewInitializers: Record<ViewId, () => Promise<void>> = {
  'dashboard-view': async () => {
    await initDashboard();
  },
  'portfolio-view': async () => {
    await initPortfolio();
  },
  'risk-view': async () => {
    log.debug('Risk view activated');
  },
  'exposure-view': async () => {
    await initExposure();
  },
  'scenarios-view': async () => {
    await initScenarios();
  },
  'market-data-view': async () => {
    await initMarketData();
  },
  'trade-expansion-view': async () => {
    await initTradeExpansion();
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
  'graph-view': async () => {
    log.debug('Graph view activated');
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

  // Update header title and breadcrumb
  const titles = viewTitles[viewId as ViewId];
  if (titles) {
    const pageTitle = document.getElementById('page-title');
    const breadcrumb = document.getElementById('breadcrumb-current');
    if (pageTitle) pageTitle.textContent = titles.title;
    if (breadcrumb) breadcrumb.textContent = titles.breadcrumb;
  }

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
  // Handle nav item clicks
  document.querySelectorAll('.nav-item').forEach((item) => {
    item.addEventListener('click', (e) => {
      e.preventDefault();
      const viewId = (item as HTMLElement).dataset.view;
      if (viewId) {
        navigateTo(viewId);
      }
    });
  });

  // Handle logo click
  const logo = document.querySelector('.logo[data-view]');
  if (logo) {
    logo.addEventListener('click', (e) => {
      e.preventDefault();
      const viewId = (logo as HTMLElement).dataset.view;
      if (viewId) {
        navigateTo(viewId);
      }
    });
  }

  // Handle accordion toggle
  const accordion = document.getElementById('analysis-accordion');
  const accordionBtn = document.getElementById('analysis-accordion-btn');
  if (accordion && accordionBtn) {
    // Start with accordion open
    accordion.classList.add('expanded');

    accordionBtn.addEventListener('click', () => {
      accordion.classList.toggle('expanded');
    });
  }

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
