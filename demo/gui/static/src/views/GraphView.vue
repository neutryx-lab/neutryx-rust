<script setup lang="ts">
import { ref, reactive, computed, onMounted, onUnmounted, watch } from 'vue';
import { fetchPortfolioGraph, fetchPortfolioTrades } from '@/services/api';
import { useMarketEnvStore } from '@/stores/marketEnv';
import type {
  GraphNode,
  GraphEdge,
  GraphMetadata,
  TradeSummary,
  TradeStatistics,
} from '@/types';

// D3 is loaded via CDN - using global namespace
// eslint-disable-next-line @typescript-eslint/no-explicit-any
declare const d3: any;

// =============================================================================
// D3 Extended Types
// =============================================================================

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

interface AdjacencyInfo {
  incoming: string[];
  outgoing: string[];
  neighbours: string[];
}

type LayoutMode = 'force' | 'hierarchical';
type ActiveTab = 'portfolio' | 'pricer';
type AnalysisMode = 'none' | 'critical-path' | 'path-finder';

// =============================================================================
// State
// =============================================================================

// Market Environment
const marketEnv = useMarketEnvStore();
const selectedPricerGraphId = ref('');

// Data state
const nodes = ref<GraphNode[]>([]);
const edges = ref<GraphEdge[]>([]);
const metadata = ref<GraphMetadata | null>(null);
const trades = ref<TradeSummary[]>([]);
const tradeStatistics = ref<TradeStatistics | null>(null);

// UI state
const containerRef = ref<HTMLDivElement | null>(null);
const selectedTradeId = ref('');
const selectedNode = ref<GraphNode | null>(null);
const searchQuery = ref('');
const searchResults = ref<GraphNode[]>([]);
const searchIndex = ref(0);
const isLoading = ref(false);
const loadError = ref<string | null>(null);
const activeTab = ref<ActiveTab>('portfolio');
const layoutMode = ref<LayoutMode>('force');
const analysisMode = ref<AnalysisMode>('none');
const showExportMenu = ref(false);

// Node type filter
const activeNodeTypes = reactive(new Set<string>([
  'Input', 'Output', 'Mul', 'Add', 'Sub', 'Div', 'Exp', 'Log', 'Sqrt', 'Custom',
]));

// Path finder state
const pathFinderSource = ref<string | null>(null);
const pathFinderTarget = ref<string | null>(null);
const foundPath = ref<string[]>([]);

// Critical path state
const criticalPath = ref<string[]>([]);

// Adjacency map (built on graph load)
const adjacencyMap = ref<Map<string, AdjacencyInfo>>(new Map());

// D3 references (any types due to CDN loading)
// eslint-disable-next-line @typescript-eslint/no-explicit-any
let svg: any = null;
// eslint-disable-next-line @typescript-eslint/no-explicit-any
let simulation: any = null;
// eslint-disable-next-line @typescript-eslint/no-explicit-any
let zoomBehavior: any = null;
// eslint-disable-next-line @typescript-eslint/no-explicit-any
let mainGroup: any = null;

// =============================================================================
// Node Colours & Legend
// =============================================================================

const nodeColours: Record<string, string> = {
  Input: '#4ade80',
  Output: '#f87171',
  Mul: '#60a5fa',
  Add: '#a78bfa',
  Sub: '#fbbf24',
  Div: '#fb923c',
  Exp: '#2dd4bf',
  Log: '#e879f9',
  Sqrt: '#f472b6',
  Custom: '#94a3b8',
  default: '#94a3b8',
};

const legendItems = [
  { type: 'Input', color: '#4ade80' },
  { type: 'Output', color: '#f87171' },
  { type: 'Mul', color: '#60a5fa' },
  { type: 'Add', color: '#a78bfa' },
  { type: 'Sub', color: '#fbbf24' },
  { type: 'Div', color: '#fb923c' },
  { type: 'Exp', color: '#2dd4bf' },
  { type: 'Log', color: '#e879f9' },
  { type: 'Sqrt', color: '#f472b6' },
];

// =============================================================================
// Computed
// =============================================================================

const summaryStats = computed(() => [
  { label: 'Nodes', value: String(metadata.value?.node_count ?? 0), icon: 'fa-circle', color: '#3b82f6' },
  { label: 'Edges', value: String(metadata.value?.edge_count ?? 0), icon: 'fa-arrow-right', color: '#10b981' },
  { label: 'Depth', value: String(metadata.value?.depth ?? 0), icon: 'fa-layer-group', color: '#8b5cf6' },
  { label: 'Trades', value: String(metadata.value?.trade_count ?? trades.value.length), icon: 'fa-file-contract', color: '#f59e0b' },
  { label: 'Shared', value: String(metadata.value?.shared_node_count ?? 0), icon: 'fa-share-alt', color: '#ec4899' },
  {
    label: 'Optimisation',
    value: metadata.value?.optimisation_ratio != null
      ? `${(metadata.value.optimisation_ratio * 100).toFixed(1)}%`
      : '-',
    icon: 'fa-compress-arrows-alt',
    color: '#14b8a6',
  },
]);

const connectedNodes = computed(() => {
  if (!selectedNode.value) return { incoming: [] as GraphNode[], outgoing: [] as GraphNode[] };
  const adj = adjacencyMap.value.get(selectedNode.value.id);
  if (!adj) return { incoming: [] as GraphNode[], outgoing: [] as GraphNode[] };

  const nodeMap = new Map(nodes.value.map(n => [n.id, n]));
  return {
    incoming: adj.incoming.map(id => nodeMap.get(id)).filter((n): n is GraphNode => !!n),
    outgoing: adj.outgoing.map(id => nodeMap.get(id)).filter((n): n is GraphNode => !!n),
  };
});

const isPathFinderActive = computed(() => analysisMode.value === 'path-finder');
const isCriticalPathActive = computed(() => analysisMode.value === 'critical-path');

// =============================================================================
// API Calls
// =============================================================================

async function loadTrades() {
  try {
    const data = await fetchPortfolioTrades();
    trades.value = data.trades || [];
    tradeStatistics.value = data.statistics || null;
  } catch (err) {
    const message = err instanceof Error ? err.message : 'Unknown error';
    console.error('Failed to load trades:', message);
  }
}

async function loadGraph() {
  isLoading.value = true;
  loadError.value = null;
  clearAnalysis();
  try {
    const tradeIds = selectedTradeId.value ? [selectedTradeId.value] : undefined;
    const data = await fetchPortfolioGraph(tradeIds);

    nodes.value = data.nodes || [];
    edges.value = data.links || [];
    metadata.value = data.metadata || null;

    buildAdjacencyMap();
    renderGraph();
  } catch (err) {
    const message = err instanceof Error ? err.message : 'Unknown error';
    loadError.value = message;
  } finally {
    isLoading.value = false;
  }
}

// =============================================================================
// Adjacency Map
// =============================================================================

function buildAdjacencyMap() {
  const map = new Map<string, AdjacencyInfo>();

  for (const node of nodes.value) {
    map.set(node.id, { incoming: [], outgoing: [], neighbours: [] });
  }

  for (const edge of edges.value) {
    const src = map.get(edge.source);
    const tgt = map.get(edge.target);
    if (src) {
      src.outgoing.push(edge.target);
      if (!src.neighbours.includes(edge.target)) src.neighbours.push(edge.target);
    }
    if (tgt) {
      tgt.incoming.push(edge.source);
      if (!tgt.neighbours.includes(edge.source)) tgt.neighbours.push(edge.source);
    }
  }

  adjacencyMap.value = map;
}

// =============================================================================
// Graph Rendering
// =============================================================================

function getNodeRadius(d: D3Node): number {
  if (d.trade_ids && d.trade_ids.length > 1) return 14;
  if (d.is_sensitivity_target) return 12;
  return 10;
}

function renderGraph() {
  if (!containerRef.value) return;

  // Clear existing
  containerRef.value.innerHTML = '';
  simulation?.stop();

  if (nodes.value.length === 0) return;

  const width = containerRef.value.clientWidth || 800;
  const height = containerRef.value.clientHeight || 600;

  // Create SVG
  svg = d3.select(containerRef.value)
    .append('svg')
    .attr('width', '100%')
    .attr('height', '100%')
    .attr('viewBox', `0 0 ${width} ${height}`)
    .attr('class', 'graph-svg');

  // Zoom behaviour
  zoomBehavior = d3.zoom()
    .scaleExtent([0.1, 4])
    .on('zoom', (event: { transform: { toString: () => string } }) => {
      mainGroup?.attr('transform', event.transform.toString());
    });

  svg.call(zoomBehavior);

  // Main group
  mainGroup = svg.append('g').attr('class', 'graph-main');

  // Arrow marker
  const defs = svg.append('defs');
  defs.append('marker')
    .attr('id', 'arrowhead')
    .attr('viewBox', '-0 -5 10 10')
    .attr('refX', 20)
    .attr('refY', 0)
    .attr('orient', 'auto')
    .attr('markerWidth', 6)
    .attr('markerHeight', 6)
    .append('path')
    .attr('d', 'M 0,-5 L 10,0 L 0,5')
    .attr('fill', '#64748b');

  defs.append('marker')
    .attr('id', 'arrowhead-highlighted')
    .attr('viewBox', '-0 -5 10 10')
    .attr('refX', 20)
    .attr('refY', 0)
    .attr('orient', 'auto')
    .attr('markerWidth', 6)
    .attr('markerHeight', 6)
    .append('path')
    .attr('d', 'M 0,-5 L 10,0 L 0,5')
    .attr('fill', 'var(--primary, #3b82f6)');

  defs.append('marker')
    .attr('id', 'arrowhead-critical')
    .attr('viewBox', '-0 -5 10 10')
    .attr('refX', 20)
    .attr('refY', 0)
    .attr('orient', 'auto')
    .attr('markerWidth', 6)
    .attr('markerHeight', 6)
    .append('path')
    .attr('d', 'M 0,-5 L 10,0 L 0,5')
    .attr('fill', '#f59e0b');

  // Prepare data
  const d3Nodes: D3Node[] = nodes.value.map(n => ({ ...n }));
  const d3Links: D3Link[] = edges.value.map(e => ({
    source: e.source,
    target: e.target,
    weight: e.weight,
  }));

  if (layoutMode.value === 'hierarchical') {
    computeHierarchicalPositions(d3Nodes, d3Links, width, height);
    renderStaticGraph(d3Nodes, d3Links);
  } else {
    renderForceGraph(d3Nodes, d3Links, width, height);
  }
}

function renderForceGraph(d3Nodes: D3Node[], d3Links: D3Link[], width: number, height: number) {
  // Create simulation
  simulation = d3.forceSimulation(d3Nodes)
    .force('link', d3.forceLink(d3Links)
      .id((d: D3Node) => d.id)
      .distance(80))
    .force('charge', d3.forceManyBody().strength(-300))
    .force('center', d3.forceCenter(width / 2, height / 2))
    .force('collision', d3.forceCollide().radius(30));

  const { link, node } = drawElements(d3Nodes, d3Links);

  // Tick handler
  simulation.on('tick', () => {
    link
      .attr('x1', (d: D3Link) => (d.source as D3Node).x ?? 0)
      .attr('y1', (d: D3Link) => (d.source as D3Node).y ?? 0)
      .attr('x2', (d: D3Link) => (d.target as D3Node).x ?? 0)
      .attr('y2', (d: D3Link) => (d.target as D3Node).y ?? 0);

    node.attr('transform', (d: D3Node) => `translate(${d.x ?? 0},${d.y ?? 0})`);
  });
}

function renderStaticGraph(d3Nodes: D3Node[], d3Links: D3Link[]) {
  // Resolve link references to nodes
  const nodeMap = new Map(d3Nodes.map(n => [n.id, n]));
  for (const link of d3Links) {
    if (typeof link.source === 'string') link.source = nodeMap.get(link.source) || link.source;
    if (typeof link.target === 'string') link.target = nodeMap.get(link.target) || link.target;
  }

  const { link, node } = drawElements(d3Nodes, d3Links);

  // Set static positions
  link
    .attr('x1', (d: D3Link) => (d.source as D3Node).x ?? 0)
    .attr('y1', (d: D3Link) => (d.source as D3Node).y ?? 0)
    .attr('x2', (d: D3Link) => (d.target as D3Node).x ?? 0)
    .attr('y2', (d: D3Link) => (d.target as D3Node).y ?? 0);

  node.attr('transform', (d: D3Node) => `translate(${d.x ?? 0},${d.y ?? 0})`);

  simulation = null;
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function drawElements(d3Nodes: D3Node[], d3Links: D3Link[]): { link: any; node: any } {
  // Draw edges
  const link = mainGroup.append('g')
    .attr('class', 'links')
    .selectAll('line')
    .data(d3Links)
    .enter()
    .append('line')
    .attr('stroke', '#64748b')
    .attr('stroke-opacity', 0.6)
    .attr('stroke-width', 1.5)
    .attr('marker-end', 'url(#arrowhead)')
    .attr('class', 'graph-edge')
    .classed('edge-filtered', (d: D3Link) => {
      const srcType = typeof d.source === 'string' ? '' : (d.source as D3Node).type;
      const tgtType = typeof d.target === 'string' ? '' : (d.target as D3Node).type;
      return !activeNodeTypes.has(srcType) || !activeNodeTypes.has(tgtType);
    });

  // Draw nodes
  const node = mainGroup.append('g')
    .attr('class', 'nodes')
    .selectAll('g')
    .data(d3Nodes)
    .enter()
    .append('g')
    .attr('class', 'graph-node')
    .classed('node-filtered', (d: D3Node) => !activeNodeTypes.has(d.type))
    .call(layoutMode.value === 'force'
      ? d3.drag()
        .on('start', dragStarted)
        .on('drag', dragged)
        .on('end', dragEnded)
      : d3.drag() // no-op drag for hierarchical
    )
    .on('click', (_event: Event, d: D3Node) => handleNodeClick(d))
    .on('mouseenter', (_event: Event, d: D3Node) => highlightConnections(d.id))
    .on('mouseleave', () => clearHighlight());

  // Node circles
  node.append('circle')
    .attr('r', (d: D3Node) => getNodeRadius(d))
    .attr('fill', (d: D3Node) => nodeColours[d.type] || nodeColours.default)
    .attr('stroke', (d: D3Node) => d.is_sensitivity_target ? '#fff' : 'none')
    .attr('stroke-width', 2)
    .attr('class', 'node-circle');

  // Shared node indicator ring
  node.filter((d: D3Node) => d.trade_ids && d.trade_ids.length > 1)
    .append('circle')
    .attr('r', (d: D3Node) => getNodeRadius(d) + 3)
    .attr('fill', 'none')
    .attr('stroke', '#ec4899')
    .attr('stroke-width', 1.5)
    .attr('stroke-dasharray', '3,2')
    .attr('class', 'shared-ring');

  // Node labels
  node.append('text')
    .attr('dx', 15)
    .attr('dy', 4)
    .attr('fill', '#e2e8f0')
    .attr('font-size', '11px')
    .text((d: D3Node) => d.label);

  return { link, node };
}

// =============================================================================
// Hierarchical Layout
// =============================================================================

function computeHierarchicalPositions(d3Nodes: D3Node[], d3Links: D3Link[], width: number, height: number) {
  // Build adjacency from string-based links
  const children = new Map<string, string[]>();
  const inDegree = new Map<string, number>();

  for (const n of d3Nodes) {
    children.set(n.id, []);
    inDegree.set(n.id, 0);
  }

  for (const link of d3Links) {
    const src = typeof link.source === 'string' ? link.source : (link.source as D3Node).id;
    const tgt = typeof link.target === 'string' ? link.target : (link.target as D3Node).id;
    children.get(src)?.push(tgt);
    inDegree.set(tgt, (inDegree.get(tgt) || 0) + 1);
  }

  // BFS topological layering
  const layers = new Map<string, number>();
  const queue: string[] = [];

  for (const [id, deg] of inDegree) {
    if (deg === 0) {
      queue.push(id);
      layers.set(id, 0);
    }
  }

  let maxLayer = 0;
  while (queue.length > 0) {
    const current = queue.shift()!;
    const currentLayer = layers.get(current) || 0;
    for (const child of children.get(current) || []) {
      const newLayer = currentLayer + 1;
      if (!layers.has(child) || layers.get(child)! < newLayer) {
        layers.set(child, newLayer);
        if (newLayer > maxLayer) maxLayer = newLayer;
      }
      const newDeg = (inDegree.get(child) || 1) - 1;
      inDegree.set(child, newDeg);
      if (newDeg <= 0) queue.push(child);
    }
  }

  // Handle unvisited nodes (cycles or disconnected)
  for (const n of d3Nodes) {
    if (!layers.has(n.id)) layers.set(n.id, maxLayer + 1);
  }

  // Group by layer
  const layerGroups = new Map<number, D3Node[]>();
  for (const n of d3Nodes) {
    const layer = layers.get(n.id) || 0;
    if (!layerGroups.has(layer)) layerGroups.set(layer, []);
    layerGroups.get(layer)!.push(n);
  }

  const totalLayers = Math.max(maxLayer + 1, 1);
  const padding = 60;
  const layerHeight = (height - 2 * padding) / totalLayers;

  for (const [layer, layerNodes] of layerGroups) {
    const layerWidth = (width - 2 * padding) / (layerNodes.length + 1);
    layerNodes.forEach((n, i) => {
      n.x = padding + layerWidth * (i + 1);
      n.y = padding + layer * layerHeight;
      n.fx = n.x;
      n.fy = n.y;
    });
  }
}

// =============================================================================
// Drag Handlers (Force Layout)
// =============================================================================

function dragStarted(event: { active: number }, d: D3Node) {
  if (!event.active) simulation?.alphaTarget(0.3).restart();
  d.fx = d.x;
  d.fy = d.y;
}

function dragged(event: { x: number; y: number }, d: D3Node) {
  d.fx = event.x;
  d.fy = event.y;
}

function dragEnded(event: { active: number }, d: D3Node) {
  if (!event.active) simulation?.alphaTarget(0);
  d.fx = null;
  d.fy = null;
}

// =============================================================================
// Node Interaction
// =============================================================================

function handleNodeClick(node: D3Node) {
  if (isPathFinderActive.value) {
    if (!pathFinderSource.value) {
      pathFinderSource.value = node.id;
    } else if (!pathFinderTarget.value) {
      pathFinderTarget.value = node.id;
      runPathFinder();
    }
    return;
  }

  selectedNode.value = node;
  mainGroup?.selectAll('.node-circle').classed('selected', false);
  mainGroup?.selectAll('.graph-node')
    .filter((d: unknown) => (d as D3Node).id === node.id)
    .select('.node-circle')
    .classed('selected', true);
}

function selectAndCentreNode(nodeId: string) {
  const node = nodes.value.find(n => n.id === nodeId);
  if (!node) return;

  selectedNode.value = node;
  mainGroup?.selectAll('.node-circle').classed('selected', false);
  mainGroup?.selectAll('.graph-node')
    .filter((d: unknown) => (d as D3Node).id === nodeId)
    .select('.node-circle')
    .classed('selected', true);

  // Centre on node
  const d3Node = mainGroup?.selectAll('.graph-node')
    .filter((d: unknown) => (d as D3Node).id === nodeId)
    .datum() as D3Node | undefined;

  if (d3Node && svg && zoomBehavior && containerRef.value) {
    const w = containerRef.value.clientWidth || 800;
    const h = containerRef.value.clientHeight || 600;
    const transform = d3.zoomIdentity
      .translate(w / 2, h / 2)
      .scale(1.5)
      .translate(-(d3Node.x ?? 0), -(d3Node.y ?? 0));
    svg.transition().duration(500).call(zoomBehavior.transform, transform);
  }
}

function clearSelection() {
  selectedNode.value = null;
  mainGroup?.selectAll('.node-circle').classed('selected', false);
}

// =============================================================================
// Edge Highlighting (Hover)
// =============================================================================

function highlightConnections(nodeId: string) {
  if (!mainGroup) return;
  const adj = adjacencyMap.value.get(nodeId);
  if (!adj) return;

  const connectedIds = new Set([nodeId, ...adj.neighbours]);

  mainGroup.selectAll('.graph-node')
    .classed('node-dimmed', (d: D3Node) => !connectedIds.has(d.id));

  mainGroup.selectAll('.graph-edge')
    .each(function(this: SVGLineElement, d: D3Link) {
      const srcId = typeof d.source === 'string' ? d.source : (d.source as D3Node).id;
      const tgtId = typeof d.target === 'string' ? d.target : (d.target as D3Node).id;
      const isConnected = srcId === nodeId || tgtId === nodeId;
      d3.select(this)
        .classed('edge-dimmed', !isConnected)
        .classed('edge-highlighted', isConnected)
        .attr('marker-end', isConnected ? 'url(#arrowhead-highlighted)' : 'url(#arrowhead)');
    });
}

function clearHighlight() {
  if (!mainGroup) return;

  mainGroup.selectAll('.graph-node').classed('node-dimmed', false);
  mainGroup.selectAll('.graph-edge')
    .classed('edge-dimmed', false)
    .classed('edge-highlighted', false)
    .attr('marker-end', 'url(#arrowhead)');

  // Re-apply analysis highlights if active
  if (isCriticalPathActive.value && criticalPath.value.length > 0) {
    applyCriticalPathHighlight();
  }
  if (isPathFinderActive.value && foundPath.value.length > 0) {
    applyPathHighlight(foundPath.value);
  }
}

// =============================================================================
// Node Type Filter
// =============================================================================

function toggleNodeType(type: string) {
  if (activeNodeTypes.has(type)) {
    activeNodeTypes.delete(type);
  } else {
    activeNodeTypes.add(type);
  }
  applyNodeFilter();
}

function applyNodeFilter() {
  if (!mainGroup) return;

  mainGroup.selectAll('.graph-node')
    .classed('node-filtered', (d: D3Node) => !activeNodeTypes.has(d.type));

  mainGroup.selectAll('.graph-edge')
    .classed('edge-filtered', (d: D3Link) => {
      const srcType = typeof d.source === 'string' ? '' : (d.source as D3Node).type;
      const tgtType = typeof d.target === 'string' ? '' : (d.target as D3Node).type;
      return !activeNodeTypes.has(srcType) || !activeNodeTypes.has(tgtType);
    });
}

// =============================================================================
// Zoom Controls
// =============================================================================

function zoomIn() {
  if (svg && zoomBehavior) {
    svg.transition().duration(300).call(zoomBehavior.scaleBy, 1.2);
  }
}

function zoomOut() {
  if (svg && zoomBehavior) {
    svg.transition().duration(300).call(zoomBehavior.scaleBy, 0.8);
  }
}

function resetZoom() {
  if (svg && zoomBehavior) {
    svg.transition().duration(300).call(zoomBehavior.transform, d3.zoomIdentity);
  }
}

function fitToView() {
  if (!svg || !zoomBehavior || !mainGroup || !containerRef.value || nodes.value.length === 0) return;

  const width = containerRef.value.clientWidth || 800;
  const height = containerRef.value.clientHeight || 600;
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
// Search
// =============================================================================

function handleSearch() {
  const query = searchQuery.value.toLowerCase().trim();
  if (!query) {
    searchResults.value = [];
    searchIndex.value = 0;
    mainGroup?.selectAll('.node-circle').classed('search-highlight', false);
    return;
  }

  searchResults.value = nodes.value.filter(
    node =>
      node.id.toLowerCase().includes(query) ||
      node.label.toLowerCase().includes(query) ||
      node.type.toLowerCase().includes(query)
  );
  searchIndex.value = 0;

  if (searchResults.value.length > 0) {
    highlightSearchResult();
  }
}

function navigateSearch(direction: number) {
  if (searchResults.value.length === 0) return;
  searchIndex.value = (searchIndex.value + direction + searchResults.value.length) % searchResults.value.length;
  highlightSearchResult();
}

function highlightSearchResult() {
  if (!mainGroup || searchResults.value.length === 0) return;

  const node = searchResults.value[searchIndex.value];

  mainGroup.selectAll('.node-circle').classed('search-highlight', false);
  mainGroup.selectAll('.graph-node')
    .filter((d: unknown) => (d as D3Node).id === node.id)
    .select('.node-circle')
    .classed('search-highlight', true);

  // Centre on node
  const d3Node = mainGroup.selectAll('.graph-node')
    .filter((d: unknown) => (d as D3Node).id === node.id)
    .datum() as D3Node;

  if (d3Node && svg && zoomBehavior && containerRef.value) {
    const width = containerRef.value.clientWidth || 800;
    const height = containerRef.value.clientHeight || 600;

    const transform = d3.zoomIdentity
      .translate(width / 2, height / 2)
      .scale(1.5)
      .translate(-(d3Node.x ?? 0), -(d3Node.y ?? 0));

    svg.transition().duration(500).call(zoomBehavior.transform, transform);
  }
}

function clearSearch() {
  searchQuery.value = '';
  searchResults.value = [];
  searchIndex.value = 0;
  mainGroup?.selectAll('.node-circle').classed('search-highlight', false);
}

// =============================================================================
// Analysis: Critical Path
// =============================================================================

function toggleCriticalPath() {
  if (isCriticalPathActive.value) {
    clearAnalysis();
    return;
  }
  analysisMode.value = 'critical-path';
  criticalPath.value = computeCriticalPath();
  applyCriticalPathHighlight();
}

function computeCriticalPath(): string[] {
  // Topological sort + dynamic programming for longest path
  const children = new Map<string, string[]>();
  const inDeg = new Map<string, number>();
  const nodeIds = new Set(nodes.value.map(n => n.id));

  for (const id of nodeIds) {
    children.set(id, []);
    inDeg.set(id, 0);
  }

  for (const edge of edges.value) {
    if (nodeIds.has(edge.source) && nodeIds.has(edge.target)) {
      children.get(edge.source)!.push(edge.target);
      inDeg.set(edge.target, (inDeg.get(edge.target) || 0) + 1);
    }
  }

  // Kahn's algorithm for topological order
  const queue: string[] = [];
  for (const [id, deg] of inDeg) {
    if (deg === 0) queue.push(id);
  }

  const topoOrder: string[] = [];
  const tempInDeg = new Map(inDeg);
  while (queue.length > 0) {
    const current = queue.shift()!;
    topoOrder.push(current);
    for (const child of children.get(current) || []) {
      const newDeg = (tempInDeg.get(child) || 1) - 1;
      tempInDeg.set(child, newDeg);
      if (newDeg === 0) queue.push(child);
    }
  }

  // DP longest path
  const dist = new Map<string, number>();
  const prev = new Map<string, string | null>();

  for (const id of topoOrder) {
    dist.set(id, 0);
    prev.set(id, null);
  }

  for (const id of topoOrder) {
    for (const child of children.get(id) || []) {
      const newDist = (dist.get(id) || 0) + 1;
      if (newDist > (dist.get(child) || 0)) {
        dist.set(child, newDist);
        prev.set(child, id);
      }
    }
  }

  // Find endpoint with maximum distance
  let maxDist = 0;
  let endNode = '';
  for (const [id, d] of dist) {
    if (d > maxDist) {
      maxDist = d;
      endNode = id;
    }
  }

  // Backtrack to get path
  const path: string[] = [];
  let current: string | null = endNode;
  while (current) {
    path.unshift(current);
    current = prev.get(current) || null;
  }

  return path;
}

function applyCriticalPathHighlight() {
  if (!mainGroup || criticalPath.value.length === 0) return;

  const pathSet = new Set(criticalPath.value);
  const pathEdges = new Set<string>();
  for (let i = 0; i < criticalPath.value.length - 1; i++) {
    pathEdges.add(`${criticalPath.value[i]}->${criticalPath.value[i + 1]}`);
  }

  mainGroup.selectAll('.graph-node')
    .classed('node-on-path', (d: D3Node) => pathSet.has(d.id));

  mainGroup.selectAll('.graph-edge')
    .each(function(this: SVGLineElement, d: D3Link) {
      const srcId = typeof d.source === 'string' ? d.source : (d.source as D3Node).id;
      const tgtId = typeof d.target === 'string' ? d.target : (d.target as D3Node).id;
      const isOnPath = pathEdges.has(`${srcId}->${tgtId}`);
      d3.select(this)
        .classed('edge-critical', isOnPath)
        .attr('marker-end', isOnPath ? 'url(#arrowhead-critical)' : 'url(#arrowhead)');
    });
}

// =============================================================================
// Analysis: Path Finder
// =============================================================================

function togglePathFinder() {
  if (isPathFinderActive.value) {
    clearAnalysis();
    return;
  }
  analysisMode.value = 'path-finder';
  pathFinderSource.value = null;
  pathFinderTarget.value = null;
  foundPath.value = [];
}

function runPathFinder() {
  if (!pathFinderSource.value || !pathFinderTarget.value) return;
  foundPath.value = bfsShortestPath(pathFinderSource.value, pathFinderTarget.value);
  if (foundPath.value.length > 0) {
    applyPathHighlight(foundPath.value);
  }
}

function bfsShortestPath(from: string, to: string): string[] {
  const visited = new Set<string>();
  const prev = new Map<string, string | null>();
  const queue: string[] = [from];
  visited.add(from);
  prev.set(from, null);

  while (queue.length > 0) {
    const current = queue.shift()!;
    if (current === to) break;

    const adj = adjacencyMap.value.get(current);
    if (!adj) continue;

    for (const neighbour of [...adj.outgoing, ...adj.incoming]) {
      if (!visited.has(neighbour)) {
        visited.add(neighbour);
        prev.set(neighbour, current);
        queue.push(neighbour);
      }
    }
  }

  if (!prev.has(to)) return [];

  const path: string[] = [];
  let current: string | null = to;
  while (current) {
    path.unshift(current);
    current = prev.get(current) || null;
  }

  return path;
}

function applyPathHighlight(path: string[]) {
  if (!mainGroup || path.length === 0) return;

  const pathSet = new Set(path);
  const pathEdges = new Set<string>();
  for (let i = 0; i < path.length - 1; i++) {
    pathEdges.add(`${path[i]}->${path[i + 1]}`);
    pathEdges.add(`${path[i + 1]}->${path[i]}`); // BFS may traverse reverse edges
  }

  mainGroup.selectAll('.graph-node')
    .classed('node-on-path', (d: D3Node) => pathSet.has(d.id));

  mainGroup.selectAll('.graph-edge')
    .each(function(this: SVGLineElement, d: D3Link) {
      const srcId = typeof d.source === 'string' ? d.source : (d.source as D3Node).id;
      const tgtId = typeof d.target === 'string' ? d.target : (d.target as D3Node).id;
      const isOnPath = pathEdges.has(`${srcId}->${tgtId}`);
      d3.select(this)
        .classed('edge-critical', isOnPath)
        .attr('marker-end', isOnPath ? 'url(#arrowhead-critical)' : 'url(#arrowhead)');
    });
}

function clearAnalysis() {
  analysisMode.value = 'none';
  criticalPath.value = [];
  pathFinderSource.value = null;
  pathFinderTarget.value = null;
  foundPath.value = [];

  if (mainGroup) {
    mainGroup.selectAll('.graph-node').classed('node-on-path', false);
    mainGroup.selectAll('.graph-edge')
      .classed('edge-critical', false)
      .attr('marker-end', 'url(#arrowhead)');
  }
}

// =============================================================================
// Export
// =============================================================================

function exportSvg() {
  if (!containerRef.value) return;
  const svgElement = containerRef.value.querySelector('svg');
  if (!svgElement) return;

  const serialiser = new XMLSerializer();
  const svgString = serialiser.serializeToString(svgElement);
  const blob = new Blob([svgString], { type: 'image/svg+xml;charset=utf-8' });
  downloadBlob(blob, `graph_${selectedTradeId.value || 'portfolio'}.svg`);
  showExportMenu.value = false;
}

function exportPng() {
  if (!containerRef.value) return;
  const svgElement = containerRef.value.querySelector('svg');
  if (!svgElement) return;

  const serialiser = new XMLSerializer();
  const svgString = serialiser.serializeToString(svgElement);
  const svgBlob = new Blob([svgString], { type: 'image/svg+xml;charset=utf-8' });
  const url = URL.createObjectURL(svgBlob);

  const img = new Image();
  img.onload = () => {
    const canvas = document.createElement('canvas');
    canvas.width = svgElement.clientWidth * 2;
    canvas.height = svgElement.clientHeight * 2;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    ctx.fillStyle = '#0f172a';
    ctx.fillRect(0, 0, canvas.width, canvas.height);
    ctx.drawImage(img, 0, 0, canvas.width, canvas.height);
    canvas.toBlob((blob) => {
      if (blob) downloadBlob(blob, `graph_${selectedTradeId.value || 'portfolio'}.png`);
      URL.revokeObjectURL(url);
    });
  };
  img.src = url;
  showExportMenu.value = false;
}

function exportJson() {
  const data = { nodes: nodes.value, links: edges.value, metadata: metadata.value };
  const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
  downloadBlob(blob, `graph_${selectedTradeId.value || 'portfolio'}.json`);
  showExportMenu.value = false;
}

function downloadBlob(blob: Blob, filename: string) {
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

// =============================================================================
// Layout Mode
// =============================================================================

function setLayoutMode(mode: LayoutMode) {
  if (layoutMode.value === mode) return;
  layoutMode.value = mode;
  renderGraph();
}

// =============================================================================
// Keyboard Shortcuts
// =============================================================================

function handleKeydown(event: KeyboardEvent) {
  if (event.target instanceof HTMLInputElement || event.target instanceof HTMLSelectElement) return;

  switch (event.key) {
    case 'Escape':
      clearSelection();
      clearAnalysis();
      break;
    case 'f':
    case 'F':
      fitToView();
      break;
    case '+':
    case '=':
      zoomIn();
      break;
    case '-':
      zoomOut();
      break;
  }
}

// =============================================================================
// Pricer Graph
// =============================================================================

function loadPricerGraph(graphId: string) {
  const entry = marketEnv.getPricerGraph(graphId);
  if (!entry) return;
  clearAnalysis();
  nodes.value = entry.graphResponse.nodes || [];
  edges.value = entry.graphResponse.links || [];
  metadata.value = entry.graphResponse.metadata || null;
  buildAdjacencyMap();
  renderGraph();
}

// =============================================================================
// Watchers & Lifecycle
// =============================================================================

watch(selectedPricerGraphId, (id) => {
  if (id && activeTab.value === 'pricer') {
    loadPricerGraph(id);
  }
});

watch(selectedTradeId, () => {
  loadGraph();
});

onMounted(() => {
  loadTrades();
  loadGraph();
  window.addEventListener('keydown', handleKeydown);
});

onUnmounted(() => {
  simulation?.stop();
  simulation = null;
  svg = null;
  zoomBehavior = null;
  mainGroup = null;
  window.removeEventListener('keydown', handleKeydown);
});
</script>

<template>
  <div class="graph-view">
    <!-- Summary Stats -->
    <div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4 mb-6">
      <div v-for="stat in summaryStats" :key="stat.label" class="glass-card p-4">
        <div class="flex items-start justify-between">
          <div>
            <p class="text-xs text-[var(--text-muted)] mb-1">{{ stat.label }}</p>
            <p class="text-xl font-semibold text-[var(--text-primary)]">{{ stat.value }}</p>
          </div>
          <div class="w-9 h-9 rounded-lg flex items-center justify-center flex-shrink-0" :style="{ backgroundColor: `${stat.color}1a` }">
            <i :class="['fas', stat.icon, 'text-sm']" :style="{ color: stat.color }"></i>
          </div>
        </div>
      </div>
    </div>

    <!-- Tabs -->
    <div class="flex items-center gap-1 bg-[var(--surface)] rounded-lg p-1 mb-6 w-fit">
      <button
        :class="activeTab === 'portfolio' ? 'bg-[var(--primary)] text-white' : 'text-[var(--text-secondary)] hover:text-[var(--text-primary)]'"
        class="px-4 py-2 rounded-md text-sm font-medium transition-colors"
        @click="activeTab = 'portfolio'"
      >
        <i class="fas fa-wallet mr-2"></i>Portfolio Graph
      </button>
      <button
        :class="activeTab === 'pricer' ? 'bg-[var(--primary)] text-white' : 'text-[var(--text-secondary)] hover:text-[var(--text-primary)]'"
        class="px-4 py-2 rounded-md text-sm font-medium transition-colors"
        @click="activeTab = 'pricer'"
      >
        <i class="fas fa-calculator mr-2"></i>Pricer Graph
      </button>
    </div>

    <!-- Large Graph Warning -->
    <div v-if="metadata?.large_graph_warning" class="mb-4 p-3 rounded-lg bg-amber-500/20 border border-amber-500/50">
      <p class="text-sm text-amber-400 flex items-center gap-2">
        <i class="fas fa-exclamation-triangle"></i>
        Large graph ({{ metadata.node_count }} nodes). Consider filtering by trade for better performance.
      </p>
    </div>

    <!-- Error State -->
    <div v-if="loadError" class="mb-4 p-3 rounded-lg bg-red-500/20 border border-red-500/50">
      <p class="text-sm text-red-400 flex items-center gap-2">
        <i class="fas fa-exclamation-circle"></i>
        {{ loadError }}
      </p>
    </div>

    <!-- ==================== Portfolio Graph Tab ==================== -->
    <template v-if="activeTab === 'portfolio'">
      <!-- Controls -->
      <div class="flex flex-wrap items-center justify-between gap-4 mb-4">
        <div class="flex items-center gap-4 flex-wrap">
          <!-- Trade Selector -->
          <select
            v-model="selectedTradeId"
            class="px-4 py-2 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-[var(--primary)]"
          >
            <option value="">All Trades</option>
            <option v-for="trade in trades" :key="trade.id" :value="trade.id">
              {{ trade.id }} - {{ trade.instrument_type }} ({{ trade.currency }})
            </option>
          </select>

          <!-- Search -->
          <div class="relative">
            <input
              v-model="searchQuery"
              type="text"
              placeholder="Search nodes..."
              class="w-56 px-4 py-2 pl-10 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-[var(--primary)]"
              @input="handleSearch"
            >
            <i class="fas fa-search absolute left-3 top-1/2 -translate-y-1/2 text-[var(--text-muted)]"></i>
            <button
              v-if="searchQuery"
              class="absolute right-3 top-1/2 -translate-y-1/2 text-[var(--text-muted)] hover:text-[var(--text-primary)]"
              @click="clearSearch"
            >
              <i class="fas fa-times"></i>
            </button>
          </div>

          <!-- Search Navigation -->
          <div v-if="searchResults.length > 0" class="flex items-center gap-2 text-sm">
            <span class="text-[var(--text-muted)]">{{ searchIndex + 1 }}/{{ searchResults.length }}</span>
            <button
              class="p-1 rounded hover:bg-[var(--surface-hover)]"
              :disabled="searchResults.length <= 1"
              @click="navigateSearch(-1)"
            >
              <i class="fas fa-chevron-up"></i>
            </button>
            <button
              class="p-1 rounded hover:bg-[var(--surface-hover)]"
              :disabled="searchResults.length <= 1"
              @click="navigateSearch(1)"
            >
              <i class="fas fa-chevron-down"></i>
            </button>
          </div>
        </div>

        <div class="flex items-center gap-2 flex-wrap">
          <!-- Layout Mode Toggle -->
          <div class="flex items-center gap-1 bg-[var(--surface)] rounded-lg p-1">
            <button
              :class="layoutMode === 'force' ? 'bg-[var(--primary)] text-white' : 'text-[var(--text-secondary)]'"
              class="px-3 py-1.5 rounded-md text-xs transition-colors"
              title="Force-directed layout"
              @click="setLayoutMode('force')"
            >
              Force
            </button>
            <button
              :class="layoutMode === 'hierarchical' ? 'bg-[var(--primary)] text-white' : 'text-[var(--text-secondary)]'"
              class="px-3 py-1.5 rounded-md text-xs transition-colors"
              title="Hierarchical layered layout"
              @click="setLayoutMode('hierarchical')"
            >
              Layered
            </button>
          </div>

          <!-- Analysis Buttons -->
          <button
            :class="isCriticalPathActive ? 'bg-amber-500/20 text-amber-400 border-amber-500/50' : 'bg-[var(--surface)] text-[var(--text-secondary)]'"
            class="px-3 py-2 rounded-lg text-xs border border-[var(--glass-border)] hover:bg-[var(--surface-hover)] transition-colors"
            title="Show critical path"
            @click="toggleCriticalPath"
          >
            <i class="fas fa-route mr-1"></i>Critical Path
          </button>
          <button
            :class="isPathFinderActive ? 'bg-blue-500/20 text-blue-400 border-blue-500/50' : 'bg-[var(--surface)] text-[var(--text-secondary)]'"
            class="px-3 py-2 rounded-lg text-xs border border-[var(--glass-border)] hover:bg-[var(--surface-hover)] transition-colors"
            title="Find path between two nodes"
            @click="togglePathFinder"
          >
            <i class="fas fa-bezier-curve mr-1"></i>Find Path
          </button>

          <!-- Zoom Controls -->
          <div class="flex items-center gap-1">
            <button
              class="p-2 rounded-lg bg-[var(--surface)] text-[var(--text-secondary)] hover:bg-[var(--surface-hover)] transition-colors"
              title="Zoom In (+)"
              @click="zoomIn"
            >
              <i class="fas fa-search-plus"></i>
            </button>
            <button
              class="p-2 rounded-lg bg-[var(--surface)] text-[var(--text-secondary)] hover:bg-[var(--surface-hover)] transition-colors"
              title="Zoom Out (-)"
              @click="zoomOut"
            >
              <i class="fas fa-search-minus"></i>
            </button>
            <button
              class="p-2 rounded-lg bg-[var(--surface)] text-[var(--text-secondary)] hover:bg-[var(--surface-hover)] transition-colors"
              title="Reset Zoom"
              @click="resetZoom"
            >
              <i class="fas fa-undo"></i>
            </button>
            <button
              class="p-2 rounded-lg bg-[var(--surface)] text-[var(--text-secondary)] hover:bg-[var(--surface-hover)] transition-colors"
              title="Fit to View (F)"
              @click="fitToView"
            >
              <i class="fas fa-expand"></i>
            </button>
          </div>

          <!-- Export Menu -->
          <div class="relative">
            <button
              class="p-2 rounded-lg bg-[var(--surface)] text-[var(--text-secondary)] hover:bg-[var(--surface-hover)] transition-colors"
              title="Export"
              @click="showExportMenu = !showExportMenu"
            >
              <i class="fas fa-download"></i>
            </button>
            <div
              v-if="showExportMenu"
              class="absolute right-0 top-full mt-1 z-20 glass-card py-1 min-w-[120px]"
            >
              <button class="w-full px-3 py-2 text-left text-xs text-[var(--text-secondary)] hover:bg-[var(--surface-hover)] transition-colors" @click="exportSvg">
                <i class="fas fa-file-code mr-2"></i>Export SVG
              </button>
              <button class="w-full px-3 py-2 text-left text-xs text-[var(--text-secondary)] hover:bg-[var(--surface-hover)] transition-colors" @click="exportPng">
                <i class="fas fa-file-image mr-2"></i>Export PNG
              </button>
              <button class="w-full px-3 py-2 text-left text-xs text-[var(--text-secondary)] hover:bg-[var(--surface-hover)] transition-colors" @click="exportJson">
                <i class="fas fa-file-alt mr-2"></i>Export JSON
              </button>
            </div>
          </div>

          <button
            v-if="selectedNode"
            class="px-3 py-2 rounded-lg bg-[var(--surface)] text-[var(--text-secondary)] hover:bg-[var(--surface-hover)] transition-colors text-xs"
            @click="clearSelection"
          >
            Clear Selection
          </button>
        </div>
      </div>

      <!-- Node Type Filter -->
      <div class="flex flex-wrap items-center gap-2 mb-4">
        <span class="text-xs text-[var(--text-muted)] mr-1">Filter:</span>
        <button
          v-for="item in legendItems"
          :key="item.type"
          :class="[
            'px-2.5 py-1 rounded-full text-xs font-medium transition-all',
            activeNodeTypes.has(item.type)
              ? 'opacity-100'
              : 'opacity-30'
          ]"
          :style="{
            backgroundColor: `${item.color}20`,
            color: item.color,
            border: `1px solid ${item.color}40`,
          }"
          @click="toggleNodeType(item.type)"
        >
          {{ item.type }}
        </button>
      </div>

      <!-- Path Finder Status Bar -->
      <div v-if="isPathFinderActive" class="mb-4 p-3 rounded-lg bg-blue-500/10 border border-blue-500/30">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-3 text-sm">
            <i class="fas fa-bezier-curve text-blue-400"></i>
            <span class="text-[var(--text-secondary)]">
              <template v-if="!pathFinderSource">Click a source node...</template>
              <template v-else-if="!pathFinderTarget">
                Source: <span class="text-blue-400 font-mono">{{ pathFinderSource }}</span> — Click a target node...
              </template>
              <template v-else-if="foundPath.length > 0">
                Path found: {{ foundPath.length }} nodes
              </template>
              <template v-else>
                No path found between <span class="font-mono">{{ pathFinderSource }}</span> and <span class="font-mono">{{ pathFinderTarget }}</span>
              </template>
            </span>
          </div>
          <button class="text-xs text-[var(--text-muted)] hover:text-[var(--text-primary)]" @click="clearAnalysis">
            <i class="fas fa-times mr-1"></i>Close
          </button>
        </div>
        <!-- Path breadcrumb -->
        <div v-if="foundPath.length > 0" class="flex flex-wrap items-center gap-1 mt-2">
          <template v-for="(nodeId, i) in foundPath" :key="nodeId">
            <button
              class="px-2 py-0.5 rounded text-xs bg-blue-500/20 text-blue-400 hover:bg-blue-500/30 transition-colors font-mono"
              @click="selectAndCentreNode(nodeId)"
            >
              {{ nodeId }}
            </button>
            <i v-if="i < foundPath.length - 1" class="fas fa-chevron-right text-[var(--text-muted)] text-[10px]"></i>
          </template>
        </div>
      </div>

      <!-- Critical Path Status Bar -->
      <div v-if="isCriticalPathActive && criticalPath.length > 0" class="mb-4 p-3 rounded-lg bg-amber-500/10 border border-amber-500/30">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-3 text-sm">
            <i class="fas fa-route text-amber-400"></i>
            <span class="text-[var(--text-secondary)]">
              Critical path: {{ criticalPath.length }} nodes, depth {{ criticalPath.length - 1 }}
            </span>
          </div>
          <button class="text-xs text-[var(--text-muted)] hover:text-[var(--text-primary)]" @click="clearAnalysis">
            <i class="fas fa-times mr-1"></i>Close
          </button>
        </div>
        <div class="flex flex-wrap items-center gap-1 mt-2">
          <template v-for="(nodeId, i) in criticalPath" :key="nodeId">
            <button
              class="px-2 py-0.5 rounded text-xs bg-amber-500/20 text-amber-400 hover:bg-amber-500/30 transition-colors font-mono"
              @click="selectAndCentreNode(nodeId)"
            >
              {{ nodeId }}
            </button>
            <i v-if="i < criticalPath.length - 1" class="fas fa-chevron-right text-[var(--text-muted)] text-[10px]"></i>
          </template>
        </div>
      </div>

      <!-- Main Content Grid -->
      <div class="grid grid-cols-1 lg:grid-cols-4 gap-6">
        <!-- Graph Container -->
        <div class="lg:col-span-3">
          <div class="glass-card p-4 relative" style="min-height: 500px; height: calc(100vh - 420px); max-height: 700px;">
            <!-- Loading -->
            <div v-if="isLoading" class="absolute inset-0 flex items-center justify-center bg-[var(--glass-bg)]/80 z-10 rounded-[var(--radius-lg)]">
              <div class="text-center">
                <i class="fas fa-spinner fa-spin text-3xl text-[var(--primary)] mb-4"></i>
                <p class="text-[var(--text-muted)]">Loading graph...</p>
              </div>
            </div>

            <!-- Empty State -->
            <div v-if="!isLoading && !loadError && nodes.length === 0" class="flex flex-col items-center justify-center h-full text-[var(--text-muted)]">
              <i class="fas fa-project-diagram text-5xl mb-4 opacity-30"></i>
              <p class="text-sm">No graph data available</p>
              <p class="text-xs mt-1">Select a trade or check that the portfolio is loaded</p>
            </div>

            <!-- Graph -->
            <div ref="containerRef" class="w-full h-full"></div>

            <!-- Legend -->
            <div v-if="nodes.length > 0" class="absolute bottom-4 left-4 glass-card p-3">
              <div class="text-xs font-medium text-[var(--text-muted)] mb-2">Node Types</div>
              <div class="grid grid-cols-3 gap-x-4 gap-y-1">
                <div v-for="item in legendItems" :key="item.type" class="flex items-center gap-2">
                  <span class="w-3 h-3 rounded-full flex-shrink-0" :style="{ backgroundColor: item.color }"></span>
                  <span class="text-xs text-[var(--text-secondary)]">{{ item.type }}</span>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Right Sidebar -->
        <div class="space-y-6">
          <!-- Node Details Panel -->
          <div class="glass-card p-6">
            <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Node Details</h3>

            <div v-if="!selectedNode" class="text-center py-8">
              <i class="fas fa-hand-pointer text-3xl text-[var(--text-muted)] mb-4 opacity-30"></i>
              <p class="text-sm text-[var(--text-muted)]">Click a node to see details</p>
            </div>

            <template v-else>
              <div class="space-y-3">
                <div class="flex justify-between text-sm">
                  <span class="text-[var(--text-muted)]">ID</span>
                  <span class="text-[var(--text-primary)] font-mono text-xs">{{ selectedNode.id }}</span>
                </div>
                <div class="flex justify-between text-sm">
                  <span class="text-[var(--text-muted)]">Type</span>
                  <span class="px-2 py-0.5 rounded text-xs" :style="{ backgroundColor: `${nodeColours[selectedNode.type] || nodeColours.default}20`, color: nodeColours[selectedNode.type] || nodeColours.default }">
                    {{ selectedNode.type }}
                  </span>
                </div>
                <div class="flex justify-between text-sm">
                  <span class="text-[var(--text-muted)]">Label</span>
                  <span class="text-[var(--text-primary)]">{{ selectedNode.label }}</span>
                </div>
                <div class="flex justify-between text-sm">
                  <span class="text-[var(--text-muted)]">Group</span>
                  <span class="text-[var(--text-primary)]">{{ selectedNode.group }}</span>
                </div>
                <div class="flex justify-between text-sm">
                  <span class="text-[var(--text-muted)]">Value</span>
                  <span class="text-[var(--text-primary)] font-mono">
                    {{ selectedNode.value !== undefined ? selectedNode.value.toFixed(6) : '-' }}
                  </span>
                </div>
                <div class="flex justify-between text-sm">
                  <span class="text-[var(--text-muted)]">Sensitivity Target</span>
                  <span :class="selectedNode.is_sensitivity_target ? 'text-green-400' : 'text-[var(--text-secondary)]'">
                    {{ selectedNode.is_sensitivity_target ? 'Yes' : 'No' }}
                  </span>
                </div>
                <div class="flex justify-between text-sm">
                  <span class="text-[var(--text-muted)]">Edges</span>
                  <span class="text-[var(--text-primary)]">
                    {{ adjacencyMap.get(selectedNode.id)?.incoming.length ?? 0 }} in / {{ adjacencyMap.get(selectedNode.id)?.outgoing.length ?? 0 }} out
                  </span>
                </div>

                <!-- Shared trades badge -->
                <div v-if="selectedNode.trade_ids && selectedNode.trade_ids.length > 1" class="pt-2 border-t border-[var(--glass-border)]">
                  <div class="flex items-center gap-2 mb-2">
                    <span class="px-2 py-0.5 rounded-full text-xs bg-pink-500/20 text-pink-400">
                      Shared across {{ selectedNode.trade_ids.length }} trades
                    </span>
                  </div>
                  <div class="flex flex-wrap gap-1">
                    <span
                      v-for="tradeId in selectedNode.trade_ids"
                      :key="tradeId"
                      class="px-2 py-0.5 rounded bg-[var(--primary)]/10 text-[var(--primary)] text-xs"
                    >
                      {{ tradeId }}
                    </span>
                  </div>
                </div>
                <div v-else-if="selectedNode.trade_ids && selectedNode.trade_ids.length === 1" class="pt-2 border-t border-[var(--glass-border)]">
                  <p class="text-sm text-[var(--text-muted)] mb-1">Trade</p>
                  <span class="px-2 py-0.5 rounded bg-[var(--primary)]/10 text-[var(--primary)] text-xs">
                    {{ selectedNode.trade_ids[0] }}
                  </span>
                </div>

                <!-- Connected nodes -->
                <div v-if="connectedNodes.incoming.length > 0 || connectedNodes.outgoing.length > 0" class="pt-2 border-t border-[var(--glass-border)]">
                  <div v-if="connectedNodes.incoming.length > 0" class="mb-3">
                    <p class="text-xs text-[var(--text-muted)] mb-1">
                      <i class="fas fa-arrow-left mr-1"></i>Upstream ({{ connectedNodes.incoming.length }})
                    </p>
                    <div class="flex flex-wrap gap-1">
                      <button
                        v-for="n in connectedNodes.incoming.slice(0, 8)"
                        :key="n.id"
                        class="px-2 py-0.5 rounded text-xs transition-colors font-mono"
                        :style="{ backgroundColor: `${nodeColours[n.type] || nodeColours.default}15`, color: nodeColours[n.type] || nodeColours.default }"
                        @click="selectAndCentreNode(n.id)"
                      >
                        {{ n.label }}
                      </button>
                      <span v-if="connectedNodes.incoming.length > 8" class="text-xs text-[var(--text-muted)]">
                        +{{ connectedNodes.incoming.length - 8 }} more
                      </span>
                    </div>
                  </div>
                  <div v-if="connectedNodes.outgoing.length > 0">
                    <p class="text-xs text-[var(--text-muted)] mb-1">
                      <i class="fas fa-arrow-right mr-1"></i>Downstream ({{ connectedNodes.outgoing.length }})
                    </p>
                    <div class="flex flex-wrap gap-1">
                      <button
                        v-for="n in connectedNodes.outgoing.slice(0, 8)"
                        :key="n.id"
                        class="px-2 py-0.5 rounded text-xs transition-colors font-mono"
                        :style="{ backgroundColor: `${nodeColours[n.type] || nodeColours.default}15`, color: nodeColours[n.type] || nodeColours.default }"
                        @click="selectAndCentreNode(n.id)"
                      >
                        {{ n.label }}
                      </button>
                      <span v-if="connectedNodes.outgoing.length > 8" class="text-xs text-[var(--text-muted)]">
                        +{{ connectedNodes.outgoing.length - 8 }} more
                      </span>
                    </div>
                  </div>
                </div>
              </div>
            </template>
          </div>

          <!-- Graph Info -->
          <div v-if="metadata" class="glass-card p-6">
            <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Graph Info</h3>
            <div class="space-y-2 text-sm">
              <div class="flex justify-between">
                <span class="text-[var(--text-muted)]">Total Nodes</span>
                <span class="text-[var(--text-primary)]">{{ metadata.node_count }}</span>
              </div>
              <div class="flex justify-between">
                <span class="text-[var(--text-muted)]">Total Edges</span>
                <span class="text-[var(--text-primary)]">{{ metadata.edge_count }}</span>
              </div>
              <div class="flex justify-between">
                <span class="text-[var(--text-muted)]">Max Depth</span>
                <span class="text-[var(--text-primary)]">{{ metadata.depth }}</span>
              </div>
              <div class="flex justify-between">
                <span class="text-[var(--text-muted)]">Shared Nodes</span>
                <span class="text-[var(--text-primary)]">{{ metadata.shared_node_count }}</span>
              </div>
              <div class="flex justify-between">
                <span class="text-[var(--text-muted)]">Optimisation</span>
                <span class="text-[var(--text-primary)]">
                  {{ metadata.optimisation_ratio != null ? `${(metadata.optimisation_ratio * 100).toFixed(1)}%` : '-' }}
                </span>
              </div>
              <div class="flex justify-between">
                <span class="text-[var(--text-muted)]">Generated</span>
                <span class="text-[var(--text-primary)]">{{ new Date(metadata.generated_at).toLocaleTimeString() }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </template>

    <!-- ==================== Pricer Graph Tab ==================== -->
    <template v-if="activeTab === 'pricer'">
      <!-- No saved graphs -->
      <div v-if="marketEnv.pricerGraphs.length === 0" class="glass-card p-8">
        <div class="flex flex-col items-center justify-center min-h-[400px] text-[var(--text-muted)]">
          <i class="fas fa-flask text-5xl mb-4 opacity-30"></i>
          <p class="text-lg font-medium text-[var(--text-secondary)] mb-2">Pricer Graph</p>
          <p class="text-sm text-center max-w-lg mb-4">
            No saved pricer graphs yet. Use the <strong>Save Graph</strong> button in the Pricer
            to capture a TracedFloat computation graph and view it here.
          </p>
          <div class="flex flex-col items-center gap-2 text-xs">
            <div class="flex items-center gap-2">
              <i class="fas fa-check-circle text-green-400"></i>
              <span>Operation-level and scope-level detail modes</span>
            </div>
            <div class="flex items-center gap-2">
              <i class="fas fa-check-circle text-green-400"></i>
              <span>Source location mapping with <code class="px-1 py-0.5 rounded bg-[var(--surface)] text-[var(--text-primary)]">#[track_caller]</code></span>
            </div>
            <div class="flex items-center gap-2">
              <i class="fas fa-check-circle text-green-400"></i>
              <span>Automatic scope generation via <code class="px-1 py-0.5 rounded bg-[var(--surface)] text-[var(--text-primary)]">#[traced_scope]</code> macro</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Saved graphs available -->
      <template v-else>
        <!-- Controls -->
        <div class="flex flex-wrap items-center justify-between gap-4 mb-4">
          <div class="flex items-center gap-4">
            <select
              v-model="selectedPricerGraphId"
              class="px-4 py-2 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-[var(--primary)]"
            >
              <option value="">Select a saved graph...</option>
              <option v-for="g in marketEnv.pricerGraphs" :key="g.id" :value="g.id">
                {{ g.label }} ({{ g.detailLevel }}, {{ g.graphResponse.metadata?.node_count ?? 0 }} nodes)
              </option>
            </select>
            <button
              v-if="selectedPricerGraphId"
              class="px-3 py-2 rounded-lg bg-red-500/20 text-red-400 text-xs border border-red-500/30 hover:bg-red-500/30 transition-colors"
              @click="marketEnv.removePricerGraph(selectedPricerGraphId); selectedPricerGraphId = ''"
            >
              <i class="fas fa-trash mr-1"></i>Remove
            </button>
          </div>
          <div class="flex items-center gap-2">
            <!-- Layout Mode -->
            <div class="flex items-center gap-1 bg-[var(--surface)] rounded-lg p-1">
              <button
                :class="layoutMode === 'force' ? 'bg-[var(--primary)] text-white' : 'text-[var(--text-secondary)]'"
                class="px-3 py-1.5 rounded-md text-xs transition-colors"
                @click="setLayoutMode('force')"
              >
                Force
              </button>
              <button
                :class="layoutMode === 'hierarchical' ? 'bg-[var(--primary)] text-white' : 'text-[var(--text-secondary)]'"
                class="px-3 py-1.5 rounded-md text-xs transition-colors"
                @click="setLayoutMode('hierarchical')"
              >
                Layered
              </button>
            </div>
            <!-- Zoom -->
            <div class="flex items-center gap-1">
              <button class="p-2 rounded-lg bg-[var(--surface)] text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]" @click="zoomIn"><i class="fas fa-search-plus"></i></button>
              <button class="p-2 rounded-lg bg-[var(--surface)] text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]" @click="zoomOut"><i class="fas fa-search-minus"></i></button>
              <button class="p-2 rounded-lg bg-[var(--surface)] text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]" @click="resetZoom"><i class="fas fa-undo"></i></button>
              <button class="p-2 rounded-lg bg-[var(--surface)] text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]" @click="fitToView"><i class="fas fa-expand"></i></button>
            </div>
          </div>
        </div>

        <!-- Graph + Details -->
        <div class="grid grid-cols-1 lg:grid-cols-4 gap-6">
          <div class="lg:col-span-3">
            <div class="glass-card p-4 relative" style="min-height: 500px; height: calc(100vh - 420px); max-height: 700px;">
              <!-- Empty state -->
              <div v-if="!selectedPricerGraphId || nodes.length === 0" class="flex flex-col items-center justify-center h-full text-[var(--text-muted)]">
                <i class="fas fa-project-diagram text-5xl mb-4 opacity-30"></i>
                <p class="text-sm">Select a saved graph to visualise</p>
              </div>
              <!-- Graph container (shared) -->
              <div ref="containerRef" class="w-full h-full"></div>
              <!-- Legend -->
              <div v-if="nodes.length > 0 && selectedPricerGraphId" class="absolute bottom-4 left-4 glass-card p-3">
                <div class="text-xs font-medium text-[var(--text-muted)] mb-2">Node Types</div>
                <div class="grid grid-cols-3 gap-x-4 gap-y-1">
                  <div v-for="item in legendItems" :key="item.type" class="flex items-center gap-2">
                    <span class="w-3 h-3 rounded-full flex-shrink-0" :style="{ backgroundColor: item.color }"></span>
                    <span class="text-xs text-[var(--text-secondary)]">{{ item.type }}</span>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- Node Details -->
          <div class="space-y-6">
            <div class="glass-card p-6">
              <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Node Details</h3>
              <div v-if="!selectedNode" class="text-center py-8">
                <i class="fas fa-hand-pointer text-3xl text-[var(--text-muted)] mb-4 opacity-30"></i>
                <p class="text-sm text-[var(--text-muted)]">Click a node to see details</p>
              </div>
              <template v-else>
                <div class="space-y-3">
                  <div class="flex justify-between text-sm">
                    <span class="text-[var(--text-muted)]">ID</span>
                    <span class="text-[var(--text-primary)] font-mono text-xs">{{ selectedNode.id }}</span>
                  </div>
                  <div class="flex justify-between text-sm">
                    <span class="text-[var(--text-muted)]">Type</span>
                    <span class="px-2 py-0.5 rounded text-xs" :style="{ backgroundColor: `${nodeColours[selectedNode.type] || nodeColours.default}20`, color: nodeColours[selectedNode.type] || nodeColours.default }">
                      {{ selectedNode.type }}
                    </span>
                  </div>
                  <div class="flex justify-between text-sm">
                    <span class="text-[var(--text-muted)]">Label</span>
                    <span class="text-[var(--text-primary)]">{{ selectedNode.label }}</span>
                  </div>
                  <div class="flex justify-between text-sm">
                    <span class="text-[var(--text-muted)]">Value</span>
                    <span class="text-[var(--text-primary)] font-mono">
                      {{ selectedNode.value !== undefined ? selectedNode.value.toFixed(6) : '-' }}
                    </span>
                  </div>
                  <div class="flex justify-between text-sm">
                    <span class="text-[var(--text-muted)]">Edges</span>
                    <span class="text-[var(--text-primary)]">
                      {{ adjacencyMap.get(selectedNode.id)?.incoming.length ?? 0 }} in / {{ adjacencyMap.get(selectedNode.id)?.outgoing.length ?? 0 }} out
                    </span>
                  </div>
                </div>
              </template>
            </div>

            <!-- Graph Metadata -->
            <div v-if="metadata && selectedPricerGraphId" class="glass-card p-6">
              <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Graph Info</h3>
              <div class="space-y-2 text-sm">
                <div class="flex justify-between">
                  <span class="text-[var(--text-muted)]">Nodes</span>
                  <span class="text-[var(--text-primary)]">{{ metadata.node_count }}</span>
                </div>
                <div class="flex justify-between">
                  <span class="text-[var(--text-muted)]">Edges</span>
                  <span class="text-[var(--text-primary)]">{{ metadata.edge_count }}</span>
                </div>
                <div class="flex justify-between">
                  <span class="text-[var(--text-muted)]">Depth</span>
                  <span class="text-[var(--text-primary)]">{{ metadata.depth }}</span>
                </div>
                <div v-if="(metadata as any).source_locations" class="pt-2 border-t border-[var(--glass-border)]">
                  <p class="text-xs text-[var(--text-muted)] mb-1">Source Locations</p>
                  <div class="max-h-32 overflow-y-auto">
                    <div v-for="(loc, nodeId) in (metadata as any).source_locations" :key="nodeId" class="text-xs font-mono text-[var(--text-secondary)] truncate">
                      {{ nodeId }}: {{ loc }}
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </template>
    </template>
  </div>
</template>

<style scoped>
.glass-card {
  background: var(--glass-bg);
  backdrop-filter: blur(20px);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-lg);
  box-shadow: var(--glass-shadow);
}

/* Graph SVG */
:deep(.graph-svg) {
  cursor: grab;
}

:deep(.graph-svg:active) {
  cursor: grabbing;
}

/* Node interactions */
:deep(.graph-node) {
  cursor: pointer;
  transition: opacity 0.2s ease;
}

:deep(.node-circle) {
  transition: all 0.2s ease;
}

:deep(.node-circle:hover) {
  filter: brightness(1.2);
}

:deep(.node-circle.selected) {
  stroke: var(--primary);
  stroke-width: 3px;
}

:deep(.node-circle.search-highlight) {
  stroke: #fbbf24;
  stroke-width: 3px;
  animation: pulse 1s ease-in-out infinite;
}

/* Edge highlighting */
:deep(.graph-edge) {
  transition: stroke-opacity 0.2s ease, stroke-width 0.2s ease;
}

:deep(.node-dimmed) {
  opacity: 0.12;
}

:deep(.edge-dimmed) {
  stroke-opacity: 0.05 !important;
}

:deep(.edge-highlighted) {
  stroke: var(--primary, #3b82f6) !important;
  stroke-opacity: 1 !important;
  stroke-width: 2.5px !important;
}

/* Node type filter */
:deep(.node-filtered) {
  opacity: 0.08;
}

:deep(.edge-filtered) {
  stroke-opacity: 0.05 !important;
}

/* Analysis: critical path / path finder */
:deep(.node-on-path) .node-circle {
  stroke: #f59e0b;
  stroke-width: 3px;
  filter: drop-shadow(0 0 4px rgba(245, 158, 11, 0.5));
}

:deep(.edge-critical) {
  stroke: #f59e0b !important;
  stroke-opacity: 1 !important;
  stroke-width: 3px !important;
  filter: drop-shadow(0 0 3px rgba(245, 158, 11, 0.4));
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.6; }
}
</style>
