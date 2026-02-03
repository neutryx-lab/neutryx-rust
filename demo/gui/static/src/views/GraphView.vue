<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';
// D3 is loaded via CDN - using global namespace
// eslint-disable-next-line @typescript-eslint/no-explicit-any
declare const d3: any;

// Types
interface GraphNode {
  id: string;
  type: string;
  label: string;
  group: string;
  value?: number;
  is_sensitivity_target: boolean;
  trade_ids: string[];
}

interface GraphEdge {
  source: string;
  target: string;
  weight?: number;
}

interface GraphMetadata {
  node_count: number;
  edge_count: number;
  depth: number;
  generated_at: string;
}

interface TradeSummary {
  id: string;
  instrument_type: string;
  currency: string;
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

// State
const containerRef = ref<HTMLDivElement | null>(null);
const nodes = ref<GraphNode[]>([]);
const edges = ref<GraphEdge[]>([]);
const metadata = ref<GraphMetadata | null>(null);
const trades = ref<TradeSummary[]>([]);
const selectedTradeId = ref('');
const selectedNode = ref<GraphNode | null>(null);
const searchQuery = ref('');
const searchResults = ref<GraphNode[]>([]);
const searchIndex = ref(0);
const isLoading = ref(false);

// D3 references (any types due to CDN loading)
// eslint-disable-next-line @typescript-eslint/no-explicit-any
let svg: any = null;
// eslint-disable-next-line @typescript-eslint/no-explicit-any
let simulation: any = null;
// eslint-disable-next-line @typescript-eslint/no-explicit-any
let zoomBehavior: any = null;
// eslint-disable-next-line @typescript-eslint/no-explicit-any
let mainGroup: any = null;

// Node colours
const nodeColours: Record<string, string> = {
  Input: '#4ade80',
  Output: '#f87171',
  Mul: '#60a5fa',
  Add: '#a78bfa',
  Sub: '#fbbf24',
  Div: '#fb923c',
  Exp: '#2dd4bf',
  Log: '#e879f9',
  default: '#94a3b8',
};

// Legend items
const legendItems = [
  { type: 'Input', color: '#4ade80' },
  { type: 'Output', color: '#f87171' },
  { type: 'Mul', color: '#60a5fa' },
  { type: 'Add', color: '#a78bfa' },
  { type: 'Sub', color: '#fbbf24' },
  { type: 'Div', color: '#fb923c' },
];

// Computed
const summaryStats = computed(() => [
  { label: 'Nodes', value: metadata.value?.node_count ?? 0, icon: 'fa-circle', color: '#3b82f6' },
  { label: 'Edges', value: metadata.value?.edge_count ?? 0, icon: 'fa-arrow-right', color: '#10b981' },
  { label: 'Depth', value: metadata.value?.depth ?? 0, icon: 'fa-layer-group', color: '#8b5cf6' },
  { label: 'Trades', value: trades.value.length, icon: 'fa-file-contract', color: '#f59e0b' },
]);

// API calls
async function loadTrades() {
  try {
    const response = await fetch('/api/portfolio/trades');
    if (!response.ok) throw new Error('Failed to load trades');
    const data = await response.json();
    trades.value = data.trades || [];
  } catch (error) {
    console.error('Failed to load trades:', error);
  }
}

async function loadGraph() {
  isLoading.value = true;
  try {
    const url = selectedTradeId.value
      ? `/api/graph?trade_ids=${selectedTradeId.value}`
      : '/api/graph';
    const response = await fetch(url);
    if (!response.ok) throw new Error('Failed to load graph');
    const data = await response.json();

    nodes.value = data.nodes || [];
    edges.value = data.links || [];
    metadata.value = data.metadata || null;

    renderGraph();
  } catch (error) {
    console.error('Failed to load graph:', error);
  } finally {
    isLoading.value = false;
  }
}

// Graph rendering
function renderGraph() {
  if (!containerRef.value) return;

  // Clear existing
  containerRef.value.innerHTML = '';

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
  svg.append('defs').append('marker')
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

  // Prepare data
  const d3Nodes: D3Node[] = nodes.value.map(n => ({ ...n }));
  const d3Links: D3Link[] = edges.value.map(e => ({
    source: e.source,
    target: e.target,
    weight: e.weight,
  }));

  // Create simulation
  simulation = d3.forceSimulation(d3Nodes)
    .force('link', d3.forceLink(d3Links)
      .id((d: D3Node) => d.id)
      .distance(80))
    .force('charge', d3.forceManyBody().strength(-300))
    .force('center', d3.forceCenter(width / 2, height / 2))
    .force('collision', d3.forceCollide().radius(30));

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
    .attr('marker-end', 'url(#arrowhead)');

  // Draw nodes
  const node = mainGroup.append('g')
    .attr('class', 'nodes')
    .selectAll('g')
    .data(d3Nodes)
    .enter()
    .append('g')
    .attr('class', 'graph-node')
    .call(d3.drag()
      .on('start', dragStarted)
      .on('drag', dragged)
      .on('end', dragEnded))
    .on('click', (_event: Event, d: D3Node) => selectNodeHandler(d));

  // Node circles
  node.append('circle')
    .attr('r', (d: D3Node) => d.is_sensitivity_target ? 12 : 10)
    .attr('fill', (d: D3Node) => nodeColours[d.type] || nodeColours.default)
    .attr('stroke', (d: D3Node) => d.is_sensitivity_target ? '#fff' : 'none')
    .attr('stroke-width', 2)
    .attr('class', 'node-circle');

  // Node labels
  node.append('text')
    .attr('dx', 15)
    .attr('dy', 4)
    .attr('fill', '#e2e8f0')
    .attr('font-size', '11px')
    .text((d: D3Node) => d.label);

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

// Drag handlers
function dragStarted(event: { active: number; x: number; y: number }, d: D3Node) {
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

// Node selection
function selectNodeHandler(node: D3Node) {
  selectedNode.value = node;
  mainGroup?.selectAll('.node-circle').classed('selected', false);
  mainGroup?.selectAll('.graph-node')
    .filter((d: unknown) => (d as D3Node).id === node.id)
    .select('.node-circle')
    .classed('selected', true);
}

function clearSelection() {
  selectedNode.value = null;
  mainGroup?.selectAll('.node-circle').classed('selected', false);
}

// Zoom controls
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

// Search
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

  // Center on node
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

// Watch trade selection
watch(selectedTradeId, () => {
  loadGraph();
});

// Lifecycle
onMounted(() => {
  loadTrades();
  loadGraph();
});

onUnmounted(() => {
  simulation?.stop();
  simulation = null;
  svg = null;
  zoomBehavior = null;
  mainGroup = null;
});
</script>

<template>
  <div class="graph-view">
    <!-- Summary Stats -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
      <div v-for="stat in summaryStats" :key="stat.label" class="glass-card p-4">
        <div class="flex items-start justify-between">
          <div>
            <p class="text-sm text-[var(--text-muted)] mb-1">{{ stat.label }}</p>
            <p class="text-2xl font-semibold text-[var(--text-primary)]">{{ stat.value }}</p>
          </div>
          <div class="w-10 h-10 rounded-lg flex items-center justify-center flex-shrink-0" :style="{ backgroundColor: `${stat.color}1a` }">
            <i :class="['fas', stat.icon]" :style="{ color: stat.color }"></i>
          </div>
        </div>
      </div>
    </div>

    <!-- Controls -->
    <div class="flex flex-wrap items-center justify-between gap-4 mb-6">
      <div class="flex items-center gap-4">
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
            class="w-64 px-4 py-2 pl-10 rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-[var(--primary)]"
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

      <!-- Zoom Controls -->
      <div class="flex items-center gap-2">
        <button
          class="p-2 rounded-lg bg-[var(--surface)] text-[var(--text-secondary)] hover:bg-[var(--surface-hover)] transition-colors"
          title="Zoom In"
          @click="zoomIn"
        >
          <i class="fas fa-search-plus"></i>
        </button>
        <button
          class="p-2 rounded-lg bg-[var(--surface)] text-[var(--text-secondary)] hover:bg-[var(--surface-hover)] transition-colors"
          title="Zoom Out"
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
          title="Fit to View"
          @click="fitToView"
        >
          <i class="fas fa-expand"></i>
        </button>
        <button
          v-if="selectedNode"
          class="px-3 py-2 rounded-lg bg-[var(--surface)] text-[var(--text-secondary)] hover:bg-[var(--surface-hover)] transition-colors text-sm"
          @click="clearSelection"
        >
          Clear Selection
        </button>
      </div>
    </div>

    <div class="grid grid-cols-1 lg:grid-cols-4 gap-6">
      <!-- Graph Container -->
      <div class="lg:col-span-3">
        <div class="glass-card p-4 relative" style="height: 600px;">
          <!-- Loading -->
          <div v-if="isLoading" class="absolute inset-0 flex items-center justify-center bg-[var(--glass-bg)]/80 z-10">
            <div class="text-center">
              <i class="fas fa-spinner fa-spin text-3xl text-[var(--primary)] mb-4"></i>
              <p class="text-[var(--text-muted)]">Loading graph...</p>
            </div>
          </div>

          <!-- Graph -->
          <div ref="containerRef" class="w-full h-full"></div>

          <!-- Legend -->
          <div class="absolute bottom-4 left-4 glass-card p-3">
            <div class="text-xs font-medium text-[var(--text-muted)] mb-2">Node Types</div>
            <div class="grid grid-cols-2 gap-2">
              <div v-for="item in legendItems" :key="item.type" class="flex items-center gap-2">
                <span class="w-3 h-3 rounded-full" :style="{ backgroundColor: item.color }"></span>
                <span class="text-xs text-[var(--text-secondary)]">{{ item.type }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Node Info Panel -->
      <div>
        <div class="glass-card p-6">
          <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Node Details</h3>

          <div v-if="!selectedNode" class="text-center py-8">
            <i class="fas fa-hand-pointer text-3xl text-[var(--text-muted)] mb-4"></i>
            <p class="text-[var(--text-muted)]">Click a node to see details</p>
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
              <div v-if="selectedNode.trade_ids.length > 0" class="pt-2 border-t border-[var(--glass-border)]">
                <p class="text-sm text-[var(--text-muted)] mb-2">Associated Trades</p>
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
            </div>
          </template>
        </div>

        <!-- Graph Info -->
        <div v-if="metadata" class="glass-card p-6 mt-6">
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
              <span class="text-[var(--text-muted)]">Generated</span>
              <span class="text-[var(--text-primary)]">{{ new Date(metadata.generated_at).toLocaleTimeString() }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>
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

:deep(.graph-svg) {
  cursor: grab;
}

:deep(.graph-svg:active) {
  cursor: grabbing;
}

:deep(.graph-node) {
  cursor: pointer;
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

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.6; }
}
</style>
