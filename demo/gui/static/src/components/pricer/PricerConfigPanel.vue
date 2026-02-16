<script setup lang="ts">
/**
 * PricerConfigPanel — Flat property-grid mirroring CalcSetting + Trade inputs.
 *
 * Categories: Trade, CalcSetting, Monte Carlo, Tree, Bumps, Market Data, Model
 */
import { computed, ref, watch } from 'vue';
import { usePricerStore } from '@/stores/pricer';
import { useMarketEnvStore } from '@/stores/marketEnv';
import { usePricer } from '@/composables/usePricer';
import { resolveTenor, fetchPricerGraph } from '@/services/api';
import { STOCHASTIC_MODELS } from '@/constants/pricer';

const store = usePricerStore();
const marketEnv = useMarketEnvStore();
const { expandCashflows, calculateAll } = usePricer();

// ---------------------------------------------------------------------------
// Save Graph
// ---------------------------------------------------------------------------
const isSavingGraph = ref(false);
const graphSaveFeedback = ref(false);
const graphDetailLevel = ref<'operation' | 'scope'>('scope');

async function saveGraph() {
  const inst = store.selectedInstrument;
  if (!inst) return;
  isSavingGraph.value = true;
  try {
    const instrumentType = inst.instrumentType || inst.id || inst.type || '';
    const instrumentName = inst.displayName || inst.name || instrumentType;
    const response = await fetchPricerGraph({
      instrumentType,
      params: { ...store.instrumentParams },
      detailLevel: graphDetailLevel.value,
    });
    marketEnv.publishPricerGraph(instrumentType, instrumentName, response, graphDetailLevel.value);
    graphSaveFeedback.value = true;
    setTimeout(() => { graphSaveFeedback.value = false; }, 2000);
  } catch (err) {
    console.error('Failed to save pricer graph:', err);
  } finally {
    isSavingGraph.value = false;
  }
}

// ---------------------------------------------------------------------------
// Instruments
// ---------------------------------------------------------------------------
function getFieldError(fieldName: string): string | undefined {
  return store.validationErrors.find((e) => e.field === fieldName)?.message;
}

const instrumentItems = computed(() => {
  const items: { title: string; value: string }[] = [];
  for (const [, instruments] of Object.entries(store.groupedInstruments)) {
    for (const inst of instruments) {
      const id = inst.instrumentType || inst.id || inst.type || '';
      const name = inst.displayName || inst.name || id || '';
      items.push({ title: name, value: id });
    }
  }
  return items;
});

const allParams = computed(() => {
  const inst = store.selectedInstrument;
  if (!inst) return [];
  const required = (inst.requiredParams ?? []).map((p) => ({ ...p, required: true }));
  const optional = (inst.optionalParams ?? []).map((p) => ({ ...p, required: false }));
  return [...required, ...optional];
});

// ---------------------------------------------------------------------------
// CalcSetting dropdowns
// ---------------------------------------------------------------------------
const methodItems = [
  { title: 'Auto', value: 'auto' },
  { title: 'Analytical', value: 'analytical' },
  { title: 'Monte Carlo', value: 'monteCarlo' },
  { title: 'Tree', value: 'tree' },
];

const treeTypeItems = [
  { title: 'Binomial', value: 'binomial' },
  { title: 'Trinomial', value: 'trinomial' },
];

// ---------------------------------------------------------------------------
// Tenor-first date fields
// ---------------------------------------------------------------------------
const dateDisplay = ref<Record<string, string>>({});

function getDateDisplay(paramName: string): string {
  return dateDisplay.value[paramName] ?? (store.instrumentParams[paramName] as string) ?? '';
}

function onTenorInput(paramName: string, raw: string) {
  dateDisplay.value[paramName] = raw;
}

async function onTenorCommit(paramName: string) {
  const raw = dateDisplay.value[paramName] ?? '';
  if (!raw) {
    store.instrumentParams[paramName] = '';
    return;
  }
  try {
    const resolved = await resolveTenor(raw);
    store.instrumentParams[paramName] = resolved;
    dateDisplay.value[paramName] = resolved;
  } catch {
    dateDisplay.value[paramName] = (store.instrumentParams[paramName] as string) ?? '';
  }
}

function onCalendarPick(paramName: string, date: unknown) {
  const d = String(date);
  store.instrumentParams[paramName] = d;
  dateDisplay.value[paramName] = d;
}

async function applyTenorChip(paramName: string, tenor: string) {
  try {
    const resolved = await resolveTenor(tenor);
    store.instrumentParams[paramName] = resolved;
    dateDisplay.value[paramName] = resolved;
  } catch { /* ignore */ }
}

// Valuation date (separate from instrument params)
const valDateDisplay = ref(store.valuationDate);

function onValInput(raw: string) {
  valDateDisplay.value = raw;
}

async function onValCommit() {
  const raw = valDateDisplay.value;
  if (!raw) return;
  try {
    const resolved = await resolveTenor(raw);
    store.valuationDate = resolved;
    valDateDisplay.value = resolved;
  } catch {
    valDateDisplay.value = store.valuationDate;
  }
}

async function applyValTenor(tenor: string) {
  try {
    const resolved = await resolveTenor(tenor);
    store.valuationDate = resolved;
    valDateDisplay.value = resolved;
  } catch { /* ignore */ }
}

function onValCalendarPick(date: unknown) {
  const d = String(date);
  store.valuationDate = d;
  valDateDisplay.value = d;
}

// ---------------------------------------------------------------------------
// Market data overrides
// ---------------------------------------------------------------------------
function toggleCurveOverride(id: string) {
  const idx = store.activeCurveOverrideIds.indexOf(id);
  if (idx >= 0) {
    store.activeCurveOverrideIds.splice(idx, 1);
  } else {
    store.activeCurveOverrideIds.push(id);
  }
}

function toggleVolOverride(id: string) {
  const idx = store.activeVolOverrideIds.indexOf(id);
  if (idx >= 0) {
    store.activeVolOverrideIds.splice(idx, 1);
  } else {
    store.activeVolOverrideIds.push(id);
  }
}

function formatTime(ts: number): string {
  return new Date(ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

// ---------------------------------------------------------------------------
// Market data & model
// ---------------------------------------------------------------------------
const ccyItems = [
  { title: 'USD', value: 'USD' },
  { title: 'EUR', value: 'EUR' },
  { title: 'GBP', value: 'GBP' },
  { title: 'JPY', value: 'JPY' },
];

const modelItems = STOCHASTIC_MODELS.map((m) => ({ title: m.label, value: m.type }));

watch(
  () => store.modelType,
  () => {
    const config = STOCHASTIC_MODELS.find((m) => m.type === store.modelType);
    if (config) {
      const defaults: Record<string, number> = {};
      config.params.forEach((p) => { defaults[p.name] = p.defaultValue; });
      store.modelParams = defaults;
    }
  },
);
</script>

<template>
  <div class="glass-card config-panel">
    <div class="p-3">
      <div class="config-grid">
        <!-- ═══ TRADE ═══ -->
        <div class="section-header">Trade</div>

        <div class="grid-label">Type</div>
        <div class="grid-input">
          <v-select
            v-model="store.selectedInstrumentId"
            :items="instrumentItems"
            placeholder="Select..."
            density="compact"
            variant="outlined"
            hide-details="auto"
            :error-messages="getFieldError('instrumentType')"
          />
        </div>

        <template v-for="param in allParams" :key="param.name">
          <div class="grid-label" :class="{ required: param.required }">
            {{ param.label || param.name }}
          </div>
          <div class="grid-input">
            <v-text-field
              v-if="param.fieldType === 'number'"
              v-model.number="store.instrumentParams[param.name]"
              type="number"
              density="compact"
              variant="outlined"
              hide-details="auto"
              :error-messages="getFieldError(param.name)"
            />

            <v-text-field
              v-else-if="param.fieldType === 'date'"
              :model-value="getDateDisplay(param.name)"
              density="compact"
              variant="outlined"
              hide-details="auto"
              placeholder="5Y, 3M, TD"
              :error-messages="getFieldError(param.name)"
              @update:model-value="(v: string) => onTenorInput(param.name, v)"
              @blur="onTenorCommit(param.name)"
              @keydown.enter="onTenorCommit(param.name)"
            >
              <template #prepend-inner>
                <div class="d-flex" style="gap: 1px">
                  <v-btn v-for="t in ['TD', '1Y', '5Y']" :key="t" size="x-small" variant="text" density="compact" class="tenor-chip" @click="applyTenorChip(param.name, t)">{{ t }}</v-btn>
                </div>
              </template>
              <template #append-inner>
                <v-menu :close-on-content-click="false" location="bottom end">
                  <template #activator="{ props: menuProps }">
                    <v-btn v-bind="menuProps" icon density="compact" variant="text" size="x-small" tabindex="-1">
                      <v-icon size="14">mdi-calendar</v-icon>
                    </v-btn>
                  </template>
                  <v-date-picker :model-value="(store.instrumentParams[param.name] as string) ?? undefined" @update:model-value="(v: any) => onCalendarPick(param.name, v)" />
                </v-menu>
              </template>
            </v-text-field>

            <v-select
              v-else-if="param.fieldType === 'select'"
              v-model="store.instrumentParams[param.name]"
              :items="(param.options || []).map((o: any) => ({ title: o.label, value: o.value }))"
              density="compact"
              variant="outlined"
              hide-details="auto"
              :error-messages="getFieldError(param.name)"
            />

            <v-text-field
              v-else
              v-model="store.instrumentParams[param.name]"
              density="compact"
              variant="outlined"
              hide-details="auto"
              :error-messages="getFieldError(param.name)"
            />
          </div>
        </template>

        <!-- ═══ CALC SETTING ═══ -->
        <div class="section-header">CalcSetting</div>

        <div class="grid-label">Method</div>
        <div class="grid-input">
          <v-select v-model="store.pricingMethod" :items="methodItems" density="compact" variant="outlined" hide-details />
        </div>

        <div class="grid-label">Greeks</div>
        <div class="grid-input">
          <v-switch v-model="store.computeGreeks" color="primary" density="compact" hide-details />
        </div>

        <div class="grid-label">Rpt Ccy</div>
        <div class="grid-input">
          <v-select v-model="store.reportingCcy" :items="ccyItems" density="compact" variant="outlined" hide-details />
        </div>

        <div class="grid-label">Val Date</div>
        <div class="grid-input">
          <v-text-field
            :model-value="valDateDisplay"
            density="compact"
            variant="outlined"
            hide-details
            placeholder="TD, 1D"
            @update:model-value="onValInput"
            @blur="onValCommit"
            @keydown.enter="onValCommit"
          >
            <template #prepend-inner>
              <div class="d-flex" style="gap: 1px">
                <v-btn v-for="t in ['TD', '1D']" :key="t" size="x-small" variant="text" density="compact" class="tenor-chip" @click="applyValTenor(t)">{{ t }}</v-btn>
              </div>
            </template>
            <template #append-inner>
              <v-menu :close-on-content-click="false" location="bottom end">
                <template #activator="{ props: menuProps }">
                  <v-btn v-bind="menuProps" icon density="compact" variant="text" size="x-small" tabindex="-1">
                    <v-icon size="14">mdi-calendar</v-icon>
                  </v-btn>
                </template>
                <v-date-picker :model-value="store.valuationDate || undefined" @update:model-value="onValCalendarPick" />
              </v-menu>
            </template>
          </v-text-field>
        </div>

        <!-- ═══ MONTE CARLO (shown when method=monteCarlo) ═══ -->
        <template v-if="store.pricingMethod === 'monteCarlo'">
          <div class="section-header">MonteCarloSetting</div>

          <div class="grid-label">Paths</div>
          <div class="grid-input">
            <v-text-field v-model.number="store.mcNumPaths" type="number" density="compact" variant="outlined" hide-details />
          </div>
          <div class="grid-label">Steps</div>
          <div class="grid-input">
            <v-text-field v-model.number="store.mcNumSteps" type="number" density="compact" variant="outlined" hide-details />
          </div>
          <div class="grid-label">Seed</div>
          <div class="grid-input">
            <v-text-field v-model.number="store.mcSeed" type="number" density="compact" variant="outlined" hide-details placeholder="(random)" />
          </div>
        </template>

        <!-- ═══ TREE (shown when method=tree) ═══ -->
        <template v-if="store.pricingMethod === 'tree'">
          <div class="section-header">TreeSetting</div>

          <div class="grid-label">Steps</div>
          <div class="grid-input">
            <v-text-field v-model.number="store.treeNumSteps" type="number" density="compact" variant="outlined" hide-details />
          </div>
          <div class="grid-label">Type</div>
          <div class="grid-input">
            <v-select v-model="store.treeType" :items="treeTypeItems" density="compact" variant="outlined" hide-details />
          </div>
        </template>

        <!-- ═══ BUMPS (only when Greeks is on) ═══ -->
        <template v-if="store.computeGreeks">
          <div class="section-header">Bumps</div>

          <div class="grid-label">Rate bp</div>
          <div class="grid-input">
            <v-text-field v-model.number="store.rateBump" type="number" step="0.1" density="compact" variant="outlined" hide-details />
          </div>
          <div class="grid-label">FX %</div>
          <div class="grid-input">
            <v-text-field v-model.number="store.fxBump" type="number" step="0.1" density="compact" variant="outlined" hide-details />
          </div>
          <div class="grid-label">Vol %</div>
          <div class="grid-input">
            <v-text-field v-model.number="store.volBump" type="number" step="0.1" density="compact" variant="outlined" hide-details />
          </div>
        </template>

        <!-- ═══ MARKET DATA ═══ -->
        <div class="section-header">Market Data</div>

        <template v-if="marketEnv.curves.length === 0 && marketEnv.volSurfaces.length === 0">
          <div class="grid-span text-caption text-medium-emphasis" style="font-style: italic">
            Server defaults. Publish from CurveBuilder / VolSurface to override.
          </div>
        </template>

        <template v-if="marketEnv.curves.length > 0">
          <div class="grid-label">Curves</div>
          <div class="grid-input">
            <div class="override-list">
              <v-chip
                v-for="c in marketEnv.curves"
                :key="c.id"
                size="small"
                :variant="store.activeCurveOverrideIds.includes(c.id) ? 'flat' : 'outlined'"
                :color="store.activeCurveOverrideIds.includes(c.id) ? 'primary' : undefined"
                :prepend-icon="store.activeCurveOverrideIds.includes(c.id) ? 'mdi-check-circle' : 'mdi-circle-outline'"
                closable
                @click="toggleCurveOverride(c.id)"
                @click:close="marketEnv.removeCurve(c.id)"
              >
                {{ c.curveName }} <span class="text-caption ml-1 text-medium-emphasis">{{ formatTime(c.publishedAt) }}</span>
              </v-chip>
            </div>
          </div>
        </template>

        <template v-if="marketEnv.volSurfaces.length > 0">
          <div class="grid-label">Vol Surf</div>
          <div class="grid-input">
            <div class="override-list">
              <v-chip
                v-for="v in marketEnv.volSurfaces"
                :key="v.id"
                size="small"
                :variant="store.activeVolOverrideIds.includes(v.id) ? 'flat' : 'outlined'"
                :color="store.activeVolOverrideIds.includes(v.id) ? 'teal' : undefined"
                :prepend-icon="store.activeVolOverrideIds.includes(v.id) ? 'mdi-check-circle' : 'mdi-circle-outline'"
                closable
                @click="toggleVolOverride(v.id)"
                @click:close="marketEnv.removeVolSurface(v.id)"
              >
                {{ v.indexOrPair }} <span class="text-caption ml-1 text-medium-emphasis">{{ v.model }}</span>
              </v-chip>
            </div>
          </div>
        </template>

        <!-- ═══ MODEL (shown when method=monteCarlo) ═══ -->
        <template v-if="store.pricingMethod === 'monteCarlo'">
          <div class="section-header">Stochastic Model</div>

          <div class="grid-label">Type</div>
          <div class="grid-input">
            <v-select v-model="store.modelType" :items="modelItems" density="compact" variant="outlined" hide-details />
          </div>

          <template v-for="param in store.selectedModelConfig.params" :key="param.name">
            <div class="grid-label">{{ param.label }}</div>
            <div class="grid-input">
              <v-text-field
                v-model.number="store.modelParams[param.name]"
                type="number"
                :min="param.min"
                :max="param.max"
                :step="param.step"
                density="compact"
                variant="outlined"
                hide-details
              />
            </div>
          </template>
        </template>
      </div>

      <!-- ═══ ACTIONS ═══ -->
      <div class="d-flex mt-3" style="gap: 6px">
        <v-btn
          variant="tonal"
          size="small"
          :disabled="!store.selectedInstrumentId || store.isExpanding"
          :loading="store.isExpanding"
          prepend-icon="mdi-arrow-expand-all"
          @click="expandCashflows"
        >
          Expand
        </v-btn>
        <v-btn
          color="primary"
          size="small"
          :disabled="!store.expandedTrade || store.isCalculating"
          :loading="store.isCalculating"
          prepend-icon="mdi-play"
          @click="calculateAll"
        >
          Price
        </v-btn>
        <v-btn
          variant="tonal"
          size="small"
          color="teal"
          :disabled="!store.expandedTrade || isSavingGraph || graphSaveFeedback"
          :loading="isSavingGraph"
          :prepend-icon="graphSaveFeedback ? 'mdi-check' : 'mdi-graph-outline'"
          @click="saveGraph"
        >
          {{ graphSaveFeedback ? 'Saved!' : 'Save Graph' }}
        </v-btn>
      </div>
    </div>
  </div>
</template>

<style scoped>
.config-grid {
  display: grid;
  grid-template-columns: 90px 1fr;
  align-items: center;
  gap: 4px 8px;
}

.section-header {
  grid-column: 1 / -1;
  font-size: 0.7rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: rgba(var(--v-theme-on-surface), 0.5);
  border-bottom: 1px solid rgba(var(--v-theme-on-surface), 0.08);
  padding: 6px 0 2px;
  margin-top: 4px;
}

.section-header:first-child {
  margin-top: 0;
}

.grid-label {
  font-size: 0.8rem;
  color: rgba(var(--v-theme-on-surface), 0.7);
  text-align: right;
  padding-right: 4px;
  white-space: nowrap;
  line-height: 1.2;
}

.grid-label.required::after {
  content: ' *';
  color: rgb(var(--v-theme-error));
}

.grid-input {
  min-width: 0;
}

.tenor-chip {
  min-width: 0 !important;
  padding: 0 4px !important;
  font-size: 0.7rem !important;
}

.grid-span {
  grid-column: 1 / -1;
  padding: 2px 0;
}

.override-list {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}
</style>
