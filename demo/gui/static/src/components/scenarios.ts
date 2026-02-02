/**
 * Scenarios View Module
 * Handles scenario analysis, parameter editing, and results rendering
 */

import { Chart, registerables, TooltipItem } from 'chart.js';
import { createScopedLogger } from '@/utils/logger';

// Register Chart.js components
Chart.register(...registerables);

const log = createScopedLogger('ScenariosView');

// =============================================================================
// Types
// =============================================================================

type ScenarioType = 'parametric' | 'historical' | 'reverse';

interface ScenarioParams {
  rateShift: number;    // bps
  volShift: number;     // %
  fxShift: number;      // %
  creditSpread: number; // bps
}

interface Scenario {
  id: string;
  name: string;
  description: string;
  type: ScenarioType;
  params: ScenarioParams;
  pnl: number | null;  // null = not calculated yet
}

interface ScenariosState {
  chart: Chart | null;
  selectedType: ScenarioType;
  scenarios: Scenario[];
  selectedScenarioId: string | null;
  isRunning: boolean;
}

// =============================================================================
// Default Scenarios Data
// =============================================================================

const defaultScenarios: Scenario[] = [
  // Parametric scenarios
  { id: 'base', name: 'Base Case', description: 'Current market conditions', type: 'parametric',
    params: { rateShift: 0, volShift: 0, fxShift: 0, creditSpread: 0 }, pnl: null },
  { id: 'rates_up', name: 'Rates +100bp', description: 'Parallel shift up', type: 'parametric',
    params: { rateShift: 100, volShift: 0, fxShift: 0, creditSpread: 0 }, pnl: null },
  { id: 'rates_down', name: 'Rates -100bp', description: 'Parallel shift down', type: 'parametric',
    params: { rateShift: -100, volShift: 0, fxShift: 0, creditSpread: 0 }, pnl: null },
  { id: 'vol_up', name: 'Vol +25%', description: 'Volatility increase', type: 'parametric',
    params: { rateShift: 0, volShift: 25, fxShift: 0, creditSpread: 0 }, pnl: null },
  { id: 'fx_stress', name: 'FX Stress', description: 'USD +10% vs all', type: 'parametric',
    params: { rateShift: 0, volShift: 0, fxShift: 10, creditSpread: 0 }, pnl: null },
  // Historical scenarios
  { id: 'crisis_2008', name: '2008 Crisis', description: 'Historical replay', type: 'historical',
    params: { rateShift: -150, volShift: 80, fxShift: 15, creditSpread: 200 }, pnl: null },
  { id: 'covid_2020', name: 'COVID-19', description: 'March 2020 shock', type: 'historical',
    params: { rateShift: -100, volShift: 120, fxShift: 8, creditSpread: 150 }, pnl: null },
  { id: 'euro_crisis', name: 'Euro Crisis 2011', description: 'European debt crisis', type: 'historical',
    params: { rateShift: 50, volShift: 40, fxShift: -12, creditSpread: 180 }, pnl: null },
  // Reverse stress scenarios
  { id: 'reverse_var', name: 'Reverse VaR', description: 'Find -$5M scenario', type: 'reverse',
    params: { rateShift: 0, volShift: 0, fxShift: 0, creditSpread: 0 }, pnl: null },
  { id: 'reverse_default', name: 'CP Default', description: 'Counterparty default stress', type: 'reverse',
    params: { rateShift: 0, volShift: 0, fxShift: 0, creditSpread: 0 }, pnl: null },
];

// =============================================================================
// State
// =============================================================================

const state: ScenariosState = {
  chart: null,
  selectedType: 'parametric',
  scenarios: JSON.parse(JSON.stringify(defaultScenarios)),
  selectedScenarioId: 'base',
  isRunning: false,
};

let initialised = false;

// =============================================================================
// DOM Element References
// =============================================================================

function getSliders(): { rate: HTMLInputElement | null; vol: HTMLInputElement | null; fx: HTMLInputElement | null; credit: HTMLInputElement | null } {
  const paramGroups = document.querySelectorAll('#scenarios-view .scenario-params .param-group');
  return {
    rate: paramGroups[0]?.querySelector('input') as HTMLInputElement | null,
    vol: paramGroups[1]?.querySelector('input') as HTMLInputElement | null,
    fx: paramGroups[2]?.querySelector('input') as HTMLInputElement | null,
    credit: paramGroups[3]?.querySelector('input') as HTMLInputElement | null,
  };
}

// =============================================================================
// Chart Configuration
// =============================================================================

function createResultsChartConfig(scenarios: Scenario[]) {
  const calculatedScenarios = scenarios.filter(s => s.type === state.selectedType && s.pnl !== null);

  if (calculatedScenarios.length === 0) {
    return null;
  }

  const labels = calculatedScenarios.map(s => s.name);
  const data = calculatedScenarios.map(s => (s.pnl ?? 0) / 1000000);
  const colors = data.map(v => v >= 0 ? 'rgba(16, 185, 129, 0.8)' : 'rgba(239, 68, 68, 0.8)');
  const borderColors = data.map(v => v >= 0 ? '#10b981' : '#ef4444');

  return {
    type: 'bar' as const,
    data: {
      labels,
      datasets: [{
        label: 'P&L Impact',
        data,
        backgroundColor: colors,
        borderColor: borderColors,
        borderWidth: 2,
        borderRadius: 4,
      }],
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      indexAxis: 'y' as const,
      plugins: {
        legend: { display: false },
        tooltip: {
          backgroundColor: 'rgba(0, 0, 0, 0.8)',
          titleColor: '#fff',
          bodyColor: '#fff',
          padding: 12,
          cornerRadius: 8,
          callbacks: {
            label: (context: TooltipItem<'bar'>) => {
              const value = context.parsed.x ?? 0;
              const sign = value >= 0 ? '+' : '';
              return `P&L: ${sign}$${value.toFixed(2)}M`;
            },
          },
        },
      },
      scales: {
        x: {
          grid: { color: 'rgba(255, 255, 255, 0.05)' },
          ticks: {
            color: '#94a3b8',
            callback: (value: number | string) => `$${value}M`,
          },
        },
        y: {
          grid: { display: false },
          ticks: { color: '#94a3b8' },
        },
      },
    },
  };
}

// =============================================================================
// UI Updates
// =============================================================================

function renderChart(): void {
  const container = document.getElementById('scenario-chart');
  if (!container) return;

  // Destroy existing chart
  if (state.chart) {
    state.chart.destroy();
    state.chart = null;
  }

  const config = createResultsChartConfig(state.scenarios);

  if (!config) {
    showPlaceholder();
    return;
  }

  // Create canvas if needed
  let canvas = container.querySelector('canvas') as HTMLCanvasElement | null;
  if (!canvas) {
    container.innerHTML = '';
    canvas = document.createElement('canvas');
    canvas.id = 'scenario-results-canvas';
    container.appendChild(canvas);
  }

  const ctx = canvas.getContext('2d');
  if (!ctx) return;

  state.chart = new Chart(ctx, config);
  log.info('Scenario results chart rendered');
}

function showPlaceholder(): void {
  const container = document.getElementById('scenario-chart');
  if (!container) return;

  if (state.chart) {
    state.chart.destroy();
    state.chart = null;
  }

  container.innerHTML = `
    <div class="chart-placeholder">
      <i class="fas fa-chart-bar"></i>
      <p>Run scenarios to view results</p>
    </div>
  `;
}

function updateScenarioList(): void {
  const listContainer = document.getElementById('scenario-list');
  if (!listContainer) return;

  const filteredScenarios = state.scenarios.filter(s => s.type === state.selectedType);

  listContainer.innerHTML = filteredScenarios.map(s => {
    const isActive = s.id === state.selectedScenarioId;
    let pnlDisplay = '--';
    let pnlClass = '';

    if (s.pnl !== null) {
      const pnlValue = s.pnl / 1000000;
      pnlClass = pnlValue >= 0 ? 'positive' : 'negative';
      if (pnlValue === 0) {
        pnlDisplay = '$0';
      } else if (Math.abs(pnlValue) >= 1) {
        pnlDisplay = pnlValue > 0 ? `+$${pnlValue.toFixed(1)}M` : `-$${Math.abs(pnlValue).toFixed(1)}M`;
      } else {
        const kValue = s.pnl / 1000;
        pnlDisplay = kValue > 0 ? `+$${kValue.toFixed(0)}K` : `-$${Math.abs(kValue).toFixed(0)}K`;
      }
    }

    return `
      <div class="scenario-item ${isActive ? 'active' : ''}" data-scenario-id="${s.id}">
        <div class="scenario-info">
          <span class="scenario-name">${s.name}</span>
          <span class="scenario-desc">${s.description}</span>
        </div>
        <span class="scenario-pnl ${pnlClass}">${pnlDisplay}</span>
      </div>
    `;
  }).join('');

  // Reattach event listeners
  setupScenarioItemListeners();
}

function updateTypeButtons(): void {
  const buttons = document.querySelectorAll('#scenarios-view .scenario-type-btn');
  buttons.forEach(btn => {
    const type = (btn as HTMLElement).dataset.type;
    btn.classList.toggle('active', type === state.selectedType);
  });
}

function updateParameterSliders(): void {
  const selectedScenario = state.scenarios.find(s => s.id === state.selectedScenarioId);
  if (!selectedScenario) return;

  const sliders = getSliders();
  const params = selectedScenario.params;

  if (sliders.rate) {
    sliders.rate.value = String(params.rateShift);
    const valueSpan = sliders.rate.parentElement?.querySelector('.param-value');
    if (valueSpan) valueSpan.textContent = formatParamValue(params.rateShift);
  }

  if (sliders.vol) {
    sliders.vol.value = String(params.volShift);
    const valueSpan = sliders.vol.parentElement?.querySelector('.param-value');
    if (valueSpan) valueSpan.textContent = formatParamValue(params.volShift);
  }

  if (sliders.fx) {
    sliders.fx.value = String(params.fxShift);
    const valueSpan = sliders.fx.parentElement?.querySelector('.param-value');
    if (valueSpan) valueSpan.textContent = formatParamValue(params.fxShift);
  }

  if (sliders.credit) {
    sliders.credit.value = String(params.creditSpread);
    const valueSpan = sliders.credit.parentElement?.querySelector('.param-value');
    if (valueSpan) valueSpan.textContent = formatParamValue(params.creditSpread);
  }
}

function formatParamValue(value: number): string {
  if (value === 0) return '0';
  return value > 0 ? `+${value}` : String(value);
}

// =============================================================================
// Scenario Calculation (Mock)
// =============================================================================

function calculatePnL(params: ScenarioParams): number {
  // Simplified P&L model for demo
  // Rate sensitivity: -24K per bp
  // Vol sensitivity: +26K per %
  // FX sensitivity: -89K per %
  // Credit sensitivity: -15K per bp
  const ratePnL = params.rateShift * -24000;
  const volPnL = params.volShift * 26000;
  const fxPnL = params.fxShift * -89000;
  const creditPnL = params.creditSpread * -15000;

  // Add some randomness for realism
  const noise = (Math.random() - 0.5) * 100000;

  return ratePnL + volPnL + fxPnL + creditPnL + noise;
}

async function runScenarios(): Promise<void> {
  if (state.isRunning) return;

  state.isRunning = true;
  const runBtn = document.getElementById('run-scenarios');
  if (runBtn) {
    runBtn.innerHTML = '<i class="fas fa-spinner fa-spin"></i> Running...';
    (runBtn as HTMLButtonElement).disabled = true;
  }

  log.info('Running scenarios...');

  // Simulate calculation delay
  await new Promise(resolve => setTimeout(resolve, 800));

  // Calculate P&L for all scenarios of current type
  state.scenarios = state.scenarios.map(s => {
    if (s.type === state.selectedType) {
      return { ...s, pnl: calculatePnL(s.params) };
    }
    return s;
  });

  // Update UI
  updateScenarioList();
  renderChart();

  state.isRunning = false;
  if (runBtn) {
    runBtn.innerHTML = '<i class="fas fa-play"></i> Run Scenarios';
    (runBtn as HTMLButtonElement).disabled = false;
  }

  window.showToast?.('Scenario calculation completed', 'success');
  log.info('Scenarios completed');
}

// =============================================================================
// Event Handlers
// =============================================================================

function setupScenarioItemListeners(): void {
  const items = document.querySelectorAll('#scenarios-view .scenario-item');
  items.forEach(item => {
    item.addEventListener('click', () => {
      const id = (item as HTMLElement).dataset.scenarioId;
      if (!id) return;

      state.selectedScenarioId = id;

      // Update active state
      items.forEach(i => i.classList.remove('active'));
      item.classList.add('active');

      // Update parameter sliders to show selected scenario's params
      updateParameterSliders();

      log.debug(`Selected scenario: ${id}`);
    });
  });
}

function setupSliderListeners(): void {
  const sliders = getSliders();

  const updateScenarioParam = (param: keyof ScenarioParams, value: number, slider: HTMLInputElement) => {
    const scenario = state.scenarios.find(s => s.id === state.selectedScenarioId);
    if (!scenario) return;

    scenario.params[param] = value;
    scenario.pnl = null; // Reset P&L when params change

    const valueSpan = slider.parentElement?.querySelector('.param-value');
    if (valueSpan) valueSpan.textContent = formatParamValue(value);

    updateScenarioList();
  };

  if (sliders.rate) {
    sliders.rate.addEventListener('input', (e) => {
      const value = parseInt((e.target as HTMLInputElement).value);
      updateScenarioParam('rateShift', value, sliders.rate!);
    });
  }

  if (sliders.vol) {
    sliders.vol.addEventListener('input', (e) => {
      const value = parseInt((e.target as HTMLInputElement).value);
      updateScenarioParam('volShift', value, sliders.vol!);
    });
  }

  if (sliders.fx) {
    sliders.fx.addEventListener('input', (e) => {
      const value = parseInt((e.target as HTMLInputElement).value);
      updateScenarioParam('fxShift', value, sliders.fx!);
    });
  }

  if (sliders.credit) {
    sliders.credit.addEventListener('input', (e) => {
      const value = parseInt((e.target as HTMLInputElement).value);
      updateScenarioParam('creditSpread', value, sliders.credit!);
    });
  }
}

function setupEventListeners(): void {
  // Scenario type buttons
  const typeButtons = document.querySelectorAll('#scenarios-view .scenario-type-btn');
  typeButtons.forEach(btn => {
    btn.addEventListener('click', () => {
      const type = (btn as HTMLElement).dataset.type as ScenarioType;
      if (!type || type === state.selectedType) return;

      state.selectedType = type;

      // Select first scenario of new type
      const firstOfType = state.scenarios.find(s => s.type === type);
      state.selectedScenarioId = firstOfType?.id ?? null;

      updateTypeButtons();
      updateScenarioList();
      updateParameterSliders();

      // Show placeholder or existing results
      const hasResults = state.scenarios.some(s => s.type === type && s.pnl !== null);
      if (hasResults) {
        renderChart();
      } else {
        showPlaceholder();
      }

      log.info(`Switched to ${type} scenarios`);
    });
  });

  // Run scenarios button
  const runBtn = document.getElementById('run-scenarios');
  if (runBtn) {
    runBtn.addEventListener('click', () => void runScenarios());
  }

  // Add scenario button
  const addBtn = document.getElementById('add-scenario');
  if (addBtn) {
    addBtn.addEventListener('click', () => {
      const newId = `custom_${Date.now()}`;
      const newScenario: Scenario = {
        id: newId,
        name: `Custom ${state.scenarios.filter(s => s.type === state.selectedType).length + 1}`,
        description: 'User-defined scenario',
        type: state.selectedType,
        params: { rateShift: 0, volShift: 0, fxShift: 0, creditSpread: 0 },
        pnl: null,
      };
      state.scenarios.push(newScenario);
      state.selectedScenarioId = newId;
      updateScenarioList();
      updateParameterSliders();
      window.showToast?.('New scenario added', 'success');
    });
  }

  // Save scenario button
  const saveBtn = document.getElementById('save-scenario');
  if (saveBtn) {
    saveBtn.addEventListener('click', () => {
      window.showToast?.('Scenario saved', 'success');
    });
  }

  // Load scenario button
  const loadBtn = document.getElementById('load-scenario');
  if (loadBtn) {
    loadBtn.addEventListener('click', () => {
      window.showToast?.('Load scenario - coming soon', 'info');
    });
  }

  // Parameter sliders
  setupSliderListeners();

  // Initial scenario item listeners
  setupScenarioItemListeners();
}

// =============================================================================
// Public API
// =============================================================================

export async function init(): Promise<void> {
  if (initialised) {
    // Re-render when navigating back
    updateTypeButtons();
    updateScenarioList();
    updateParameterSliders();

    const hasResults = state.scenarios.some(s => s.type === state.selectedType && s.pnl !== null);
    if (hasResults) {
      renderChart();
    }
    return;
  }

  setupEventListeners();
  updateTypeButtons();
  updateScenarioList();
  updateParameterSliders();
  initialised = true;
  log.info('Scenarios view module initialised');
}

export const scenariosView = {
  init,
  state,
  runScenarios,
};

export default scenariosView;
