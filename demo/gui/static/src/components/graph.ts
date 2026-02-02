/**
 * AD Graph Module
 * Visualises the automatic differentiation computation graph using D3.js
 */

import type { GraphNode, GraphEdge, GraphMetadata, TradeSummary } from '@/types';
import { fetchPortfolioGraph, fetchPortfolioTrades } from '@/services/api';
import { createScopedLogger } from '@/utils/logger';
import { getElementById } from '@/utils/dom';

const log = createScopedLogger('ADGraph');

// =============================================================================
// Types
// =============================================================================

interface GraphElements {
  container: HTMLDivElement | null;
  loading: HTMLDivElement | null;
  tradeSelector: HTMLSelectElement | null;
  searchInput: HTMLInputElement | null;
  searchClear: HTMLButtonElement | null;
  searchResults: HTMLDivElement | null;
  zoomIn: HTMLButtonElement | null;
  zoomOut: HTMLButtonElement | null;
  zoomReset: HTMLButtonElement | null;
  zoomFit: HTMLButtonElement | null;
  clearSelection: HTMLButtonElement | null;
  nodeCount: HTMLElement | null;
  edgeCount: HTMLElement | null;
  depth: HTMLElement | null;
  generatedAt: HTMLElement | null;
  nodeInfoPanel: HTMLDivElement | null;
}

interface GraphState {
  nodes: GraphNode[];
  edges: GraphEdge[];
  metadata: GraphMetadata | null;
  trades: TradeSummary[];
  selectedTradeId: string | null;
  selectedNodeId: string | null;
  searchResults: GraphNode[];
  searchIndex: number;
  zoom: number;
  isInitialised: boolean;
}

interface D3Node extends GraphNode {
  x?: number;
  y?: number;
  fx?: number | null;
  fy?: number | null;
}

interface D3Link {
  source: string | D3Node;
  target: string | D3Node;
  weight?: number;
}

// =============================================================================
// State
// =============================================================================

const state: GraphState = {
  nodes: [],
  edges: [],
  metadata: null,
  trades: [],
  selectedTradeId: null,
  selectedNodeId: null,
  searchResults: [],
  searchIndex: 0,
  zoom: 1,
  isInitialised: false,
};

const elements: GraphElements = {
  container: null,
  loading: null,
  tradeSelector: null,
  searchInput: null,
  searchClear: null,
  searchResults: null,
  zoomIn: null,
  zoomOut: null,
  zoomReset: null,
  zoomFit: null,
  clearSelection: null,
  nodeCount: null,
  edgeCount: null,
  depth: null,
  generatedAt: null,
  nodeInfoPanel: null,
};

// D3 simulation and SVG references
let svg: d3.Selection<SVGSVGElement, unknown, null, undefined> | null = null;
let simulation: d3.Simulation<D3Node, D3Link> | null = null;
let zoomBehavior: d3.ZoomBehavior<SVGSVGElement, unknown> | null = null;
let mainGroup: d3.Selection<SVGGElement, unknown, null, undefined> | null = null;

// =============================================================================
// Colour Schemes
// =============================================================================

const nodeColours: Record<string, string> = {
  Input: '#4ade80',      // Green
  Output: '#f87171',     // Red
  Mul: '#60a5fa',        // Blue
  Add: '#a78bfa',        // Purple
  Sub: '#fbbf24',        // Yellow
  Div: '#fb923c',        // Orange
  Exp: '#2dd4bf',        // Teal
  Log: '#e879f9',        // Pink
  default: '#94a3b8',    // Gray
};

const groupColours: Record<string, string> = {
  Sensitivity: '#4ade80',
  Intermediate: '#60a5fa',
  Output: '#f87171',
  Shared: '#fbbf24',
  default: '#94a3b8',
};

// =============================================================================
// Initialisation
// =============================================================================

function cacheElements(): void {
  elements.container = getElementById<HTMLDivElement>('graph-container');
  elements.loading = getElementById<HTMLDivElement>('graph-loading');
  elements.tradeSelector = getElementById<HTMLSelectElement>('graph-trade-selector');
  elements.searchInput = getElementById<HTMLInputElement>('graph-search-input');
  elements.searchClear = getElementById<HTMLButtonElement>('graph-search-clear');
  elements.searchResults = getElementById<HTMLDivElement>('graph-search-results');
  elements.zoomIn = getElementById<HTMLButtonElement>('graph-zoom-in');
  elements.zoomOut = getElementById<HTMLButtonElement>('graph-zoom-out');
  elements.zoomReset = getElementById<HTMLButtonElement>('graph-zoom-reset');
  elements.zoomFit = getElementById<HTMLButtonElement>('graph-zoom-fit');
  elements.clearSelection = getElementById<HTMLButtonElement>('graph-clear-selection');
  elements.nodeCount = getElementById<HTMLElement>('graph-node-count');
  elements.edgeCount = getElementById<HTMLElement>('graph-edge-count');
  elements.depth = getElementById<HTMLElement>('graph-depth');
  elements.generatedAt = getElementById<HTMLElement>('graph-generated-at');
  elements.nodeInfoPanel = getElementById<HTMLDivElement>('node-info-panel');
}

function attachEventListeners(): void {
  elements.tradeSelector?.addEventListener('change', handleTradeChange);
  elements.searchInput?.addEventListener('input', handleSearch);
  elements.searchClear?.addEventListener('click', clearSearch);
  elements.zoomIn?.addEventListener('click', () => handleZoom(1.2));
  elements.zoomOut?.addEventListener('click', () => handleZoom(0.8));
  elements.zoomReset?.addEventListener('click', resetZoom);
  elements.zoomFit?.addEventListener('click', fitToView);
  elements.clearSelection?.addEventListener('click', clearSelection);

  // Search navigation
  getElementById('search-prev')?.addEventListener('click', () => navigateSearch(-1));
  getElementById('search-next')?.addEventListener('click', () => navigateSearch(1));
}

async function loadTrades(): Promise<void> {
  try {
    const response = await fetchPortfolioTrades();
    state.trades = response.trades;
    renderTradeSelector();
  } catch (error) {
    log.error('Failed to load trades', error);
  }
}

function renderTradeSelector(): void {
  if (!elements.tradeSelector) return;

  // Keep the "All Trades" option
  elements.tradeSelector.innerHTML = '<option value="">All Trades</option>';

  state.trades.forEach((trade) => {
    const option = document.createElement('option');
    option.value = trade.id;
    option.textContent = `${trade.id} - ${trade.instrument_type} (${trade.currency})`;
    elements.tradeSelector!.appendChild(option);
  });
}

async function loadGraph(): Promise<void> {
  showLoading(true);

  try {
    const tradeIds = state.selectedTradeId ? [state.selectedTradeId] : undefined;
    const response = await fetchPortfolioGraph(tradeIds);

    state.nodes = response.nodes;
    state.edges = response.links;
    state.metadata = response.metadata;

    updateStatistics();
    renderGraph();
  } catch (error) {
    log.error('Failed to load graph', error);
    showError('Failed to load computation graph');
  } finally {
    showLoading(false);
  }
}

// =============================================================================
// Graph Rendering
// =============================================================================

function renderGraph(): void {
  if (!elements.container) return;

  // Clear existing content
  elements.container.innerHTML = '';

  // Check if D3 is available
  if (typeof d3 === 'undefined') {
    showError('D3.js library not loaded');
    return;
  }

  const width = elements.container.clientWidth || 800;
  const height = elements.container.clientHeight || 600;

  // Create SVG
  svg = d3.select(elements.container)
    .append('svg')
    .attr('width', '100%')
    .attr('height', '100%')
    .attr('viewBox', `0 0 ${width} ${height}`)
    .attr('class', 'graph-svg');

  // Add zoom behaviour
  zoomBehavior = d3.zoom<SVGSVGElement, unknown>()
    .scaleExtent([0.1, 4])
    .on('zoom', (event: d3.D3ZoomEvent<SVGSVGElement, unknown>) => {
      mainGroup?.attr('transform', event.transform.toString());
      state.zoom = event.transform.k;
    });

  svg.call(zoomBehavior);

  // Create main group for transformations
  mainGroup = svg.append('g').attr('class', 'graph-main');

  // Add arrow marker for directed edges
  svg.append('defs').append('marker')
    .attr('id', 'arrowhead')
    .attr('viewBox', '-0 -5 10 10')
    .attr('refX', 20)
    .attr('refY', 0)
    .attr('orient', 'auto')
    .attr('markerWidth', 6)
    .attr('markerHeight', 6)
    .append('path')
    .attr('d', 'M 0,-5 L 10 ,0 L 0,5')
    .attr('fill', '#64748b');

  // Prepare data for D3
  const nodes: D3Node[] = state.nodes.map((n) => ({ ...n }));
  const links: D3Link[] = state.edges.map((e) => ({
    source: e.source,
    target: e.target,
    weight: e.weight,
  }));

  // Create simulation
  simulation = d3.forceSimulation<D3Node>(nodes)
    .force('link', d3.forceLink<D3Node, D3Link>(links)
      .id((d) => d.id)
      .distance(80))
    .force('charge', d3.forceManyBody().strength(-300))
    .force('center', d3.forceCenter(width / 2, height / 2))
    .force('collision', d3.forceCollide().radius(30));

  // Draw edges
  const link = mainGroup.append('g')
    .attr('class', 'links')
    .selectAll('line')
    .data(links)
    .enter()
    .append('line')
    .attr('class', 'graph-link')
    .attr('stroke', '#64748b')
    .attr('stroke-opacity', 0.6)
    .attr('stroke-width', 1.5)
    .attr('marker-end', 'url(#arrowhead)');

  // Draw nodes
  const node = mainGroup.append('g')
    .attr('class', 'nodes')
    .selectAll('g')
    .data(nodes)
    .enter()
    .append('g')
    .attr('class', 'graph-node')
    .call(d3.drag<SVGGElement, D3Node>()
      .on('start', dragStarted)
      .on('drag', dragged)
      .on('end', dragEnded))
    .on('click', (event: MouseEvent, d: D3Node) => selectNode(d));

  // Node circles
  node.append('circle')
    .attr('r', (d) => d.is_sensitivity_target ? 12 : 10)
    .attr('fill', (d) => nodeColours[d.type] || nodeColours.default)
    .attr('stroke', (d) => d.is_sensitivity_target ? '#fff' : 'none')
    .attr('stroke-width', 2)
    .attr('class', 'node-circle');

  // Node labels
  node.append('text')
    .attr('dx', 15)
    .attr('dy', 4)
    .attr('class', 'node-label')
    .attr('fill', '#e2e8f0')
    .attr('font-size', '11px')
    .text((d) => d.label);

  // Update positions on simulation tick
  simulation.on('tick', () => {
    link
      .attr('x1', (d) => (d.source as D3Node).x ?? 0)
      .attr('y1', (d) => (d.source as D3Node).y ?? 0)
      .attr('x2', (d) => (d.target as D3Node).x ?? 0)
      .attr('y2', (d) => (d.target as D3Node).y ?? 0);

    node.attr('transform', (d) => `translate(${d.x ?? 0},${d.y ?? 0})`);
  });

  log.info(`Graph rendered with ${nodes.length} nodes and ${links.length} edges`);
}

// =============================================================================
// Drag Handlers
// =============================================================================

function dragStarted(event: d3.D3DragEvent<SVGGElement, D3Node, D3Node>, d: D3Node): void {
  if (!event.active) simulation?.alphaTarget(0.3).restart();
  d.fx = d.x;
  d.fy = d.y;
}

function dragged(event: d3.D3DragEvent<SVGGElement, D3Node, D3Node>, d: D3Node): void {
  d.fx = event.x;
  d.fy = event.y;
}

function dragEnded(event: d3.D3DragEvent<SVGGElement, D3Node, D3Node>, d: D3Node): void {
  if (!event.active) simulation?.alphaTarget(0);
  d.fx = null;
  d.fy = null;
}

// =============================================================================
// Event Handlers
// =============================================================================

async function handleTradeChange(event: Event): Promise<void> {
  const select = event.target as HTMLSelectElement;
  state.selectedTradeId = select.value || null;
  await loadGraph();
}

function handleSearch(event: Event): void {
  const input = event.target as HTMLInputElement;
  const query = input.value.toLowerCase().trim();

  if (!query) {
    clearSearch();
    return;
  }

  state.searchResults = state.nodes.filter(
    (node) =>
      node.id.toLowerCase().includes(query) ||
      node.label.toLowerCase().includes(query) ||
      node.type.toLowerCase().includes(query)
  );
  state.searchIndex = 0;

  updateSearchResults();

  if (state.searchResults.length > 0) {
    highlightSearchResult();
  }
}

function updateSearchResults(): void {
  if (!elements.searchResults) return;

  const count = state.searchResults.length;
  const countEl = getElementById('search-results-count');
  if (countEl) {
    countEl.textContent = `${count} result${count !== 1 ? 's' : ''}`;
  }

  elements.searchResults.style.display = count > 0 ? 'block' : 'none';
  if (elements.searchClear) {
    elements.searchClear.style.display = count > 0 ? 'block' : 'none';
  }

  // Enable/disable navigation
  const prevBtn = getElementById<HTMLButtonElement>('search-prev');
  const nextBtn = getElementById<HTMLButtonElement>('search-next');
  if (prevBtn) prevBtn.disabled = count <= 1;
  if (nextBtn) nextBtn.disabled = count <= 1;
}

function navigateSearch(direction: number): void {
  if (state.searchResults.length === 0) return;

  state.searchIndex =
    (state.searchIndex + direction + state.searchResults.length) %
    state.searchResults.length;
  highlightSearchResult();
}

function highlightSearchResult(): void {
  if (!mainGroup || state.searchResults.length === 0) return;

  const node = state.searchResults[state.searchIndex];

  // Remove previous highlights
  mainGroup.selectAll('.node-circle').classed('search-highlight', false);

  // Add highlight to current node
  mainGroup
    .selectAll('.graph-node')
    .filter((d: unknown) => (d as D3Node).id === node.id)
    .select('.node-circle')
    .classed('search-highlight', true);

  // Centre view on node
  const d3Node = mainGroup
    .selectAll('.graph-node')
    .filter((d: unknown) => (d as D3Node).id === node.id)
    .datum() as D3Node;

  if (d3Node && svg && zoomBehavior) {
    const width = elements.container?.clientWidth || 800;
    const height = elements.container?.clientHeight || 600;

    const transform = d3.zoomIdentity
      .translate(width / 2, height / 2)
      .scale(1.5)
      .translate(-(d3Node.x ?? 0), -(d3Node.y ?? 0));

    svg.transition().duration(500).call(zoomBehavior.transform, transform);
  }
}

function clearSearch(): void {
  if (elements.searchInput) elements.searchInput.value = '';
  if (elements.searchResults) elements.searchResults.style.display = 'none';
  if (elements.searchClear) elements.searchClear.style.display = 'none';

  state.searchResults = [];
  state.searchIndex = 0;

  mainGroup?.selectAll('.node-circle').classed('search-highlight', false);
}

function selectNode(node: D3Node): void {
  state.selectedNodeId = node.id;

  // Update visual selection
  mainGroup?.selectAll('.node-circle').classed('selected', false);
  mainGroup
    ?.selectAll('.graph-node')
    .filter((d: unknown) => (d as D3Node).id === node.id)
    .select('.node-circle')
    .classed('selected', true);

  // Update node info panel
  updateNodeInfoPanel(node);
}

function updateNodeInfoPanel(node: GraphNode): void {
  if (!elements.nodeInfoPanel) return;

  elements.nodeInfoPanel.innerHTML = `
    <div class="node-info-row">
      <span class="node-info-label">ID</span>
      <span class="node-info-value">${node.id}</span>
    </div>
    <div class="node-info-row">
      <span class="node-info-label">Type</span>
      <span class="node-info-value">${node.type}</span>
    </div>
    <div class="node-info-row">
      <span class="node-info-label">Label</span>
      <span class="node-info-value">${node.label}</span>
    </div>
    <div class="node-info-row">
      <span class="node-info-label">Group</span>
      <span class="node-info-value">${node.group}</span>
    </div>
    <div class="node-info-row">
      <span class="node-info-label">Value</span>
      <span class="node-info-value">${node.value !== undefined ? node.value.toFixed(6) : '-'}</span>
    </div>
    <div class="node-info-row">
      <span class="node-info-label">Sensitivity</span>
      <span class="node-info-value">${node.is_sensitivity_target ? 'Yes' : 'No'}</span>
    </div>
    <div class="node-info-row">
      <span class="node-info-label">Trades</span>
      <span class="node-info-value">${node.trade_ids.join(', ') || '-'}</span>
    </div>
  `;
}

function clearSelection(): void {
  state.selectedNodeId = null;
  mainGroup?.selectAll('.node-circle').classed('selected', false);

  if (elements.nodeInfoPanel) {
    elements.nodeInfoPanel.innerHTML = '<div class="no-selection">Click a node to see details</div>';
  }
}

// =============================================================================
// Zoom Controls
// =============================================================================

function handleZoom(factor: number): void {
  if (!svg || !zoomBehavior) return;

  svg.transition().duration(300).call(zoomBehavior.scaleBy, factor);
}

function resetZoom(): void {
  if (!svg || !zoomBehavior) return;

  svg.transition().duration(300).call(zoomBehavior.transform, d3.zoomIdentity);
}

function fitToView(): void {
  if (!svg || !zoomBehavior || !mainGroup || state.nodes.length === 0) return;

  const width = elements.container?.clientWidth || 800;
  const height = elements.container?.clientHeight || 600;

  // Get the bounds of all nodes
  const bounds = mainGroup.node()?.getBBox();
  if (!bounds) return;

  const dx = bounds.width;
  const dy = bounds.height;
  const x = bounds.x + dx / 2;
  const y = bounds.y + dy / 2;

  const scale = 0.9 / Math.max(dx / width, dy / height);
  const translate = [width / 2 - scale * x, height / 2 - scale * y];

  const transform = d3.zoomIdentity
    .translate(translate[0], translate[1])
    .scale(scale);

  svg.transition().duration(500).call(zoomBehavior.transform, transform);
}

// =============================================================================
// UI Helpers
// =============================================================================

function showLoading(show: boolean): void {
  if (elements.loading) {
    elements.loading.style.display = show ? 'flex' : 'none';
  }
}

function showError(message: string): void {
  if (!elements.container) return;

  elements.container.innerHTML = `
    <div class="graph-placeholder error">
      <i class="fas fa-exclamation-triangle"></i>
      <p>${message}</p>
    </div>
  `;
}

function updateStatistics(): void {
  if (!state.metadata) return;

  if (elements.nodeCount) elements.nodeCount.textContent = String(state.metadata.node_count);
  if (elements.edgeCount) elements.edgeCount.textContent = String(state.metadata.edge_count);
  if (elements.depth) elements.depth.textContent = String(state.metadata.depth);
  if (elements.generatedAt) {
    const date = new Date(state.metadata.generated_at);
    elements.generatedAt.textContent = date.toLocaleTimeString();
  }
}

// =============================================================================
// Public API
// =============================================================================

export async function init(): Promise<void> {
  if (state.isInitialised) {
    log.debug('Graph already initialised, refreshing...');
    await loadGraph();
    return;
  }

  log.info('Initialising AD Graph module');

  cacheElements();
  attachEventListeners();

  await loadTrades();
  await loadGraph();

  state.isInitialised = true;
  log.info('AD Graph module initialised');
}

export const adGraph = {
  init,
  loadGraph,
  clearSelection,
  fitToView,
};
