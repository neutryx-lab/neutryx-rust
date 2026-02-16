<script setup lang="ts">
/**
 * GreeksAnalyserView — Advanced Greeks analysis using saved Pricer history.
 *
 * Sends legs + bump config to /api/pricer/advanced-greeks and displays
 * per-factor Greeks in a table, mirroring pricer_risk::GreeksResultByFactor.
 */
import { ref, computed } from 'vue';
import { usePricerStore } from '@/stores/pricer';
import { computeAdvancedGreeks } from '@/services/api';
import type { AdvancedGreeksResult, AdvancedGreeksMode, FactorGreeks } from '@/types/api';
import type { HistoryEntry } from '@/constants/pricer';
import { formatCurrency } from '@/utils/format';

const store = usePricerStore();

// Selected history entry
const selectedEntryId = ref<string | null>(null);
const selectedEntry = computed<HistoryEntry | null>(() =>
  store.resultHistory.find((e) => e.id === selectedEntryId.value) ?? null,
);

const historyItems = computed(() =>
  store.resultHistory.map((e) => ({
    title: `${e.instrumentName} — ${formatCurrency(e.totalPv, e.reportingCcy)} (${e.valuationDate})`,
    value: e.id,
  })),
);

// Bump configuration
const rateBump = ref(0.0001);
const volBump = ref(0.01);
const timeBump = ref(1.0 / 365.0);
const spotBump = ref(0.01);
const greeksMode = ref<AdvancedGreeksMode>('bumpRevalue');

// Result
const result = ref<AdvancedGreeksResult | null>(null);
const isComputing = ref(false);
const error = ref<string | null>(null);

async function compute() {
  const entry = selectedEntry.value;
  if (!entry) return;

  isComputing.value = true;
  error.value = null;

  try {
    result.value = await computeAdvancedGreeks({
      valuationDate: entry.valuationDate,
      reportingCurrency: entry.reportingCcy,
      legs: entry.legs,
      config: {
        rateBumpAbsolute: rateBump.value,
        volBumpAbsolute: volBump.value,
        timeBumpYears: timeBump.value,
        spotBumpRelative: spotBump.value,
        mode: greeksMode.value,
      },
    });
  } catch (e) {
    error.value = (e as Error).message;
    result.value = null;
  } finally {
    isComputing.value = false;
  }
}

// Greek column definitions for the table.
const greekColumns = [
  { key: 'delta' as const, label: 'Delta', order: '1st' },
  { key: 'gamma' as const, label: 'Gamma', order: '2nd' },
  { key: 'vega' as const, label: 'Vega', order: '1st' },
  { key: 'theta' as const, label: 'Theta', order: '1st' },
  { key: 'rho' as const, label: 'Rho', order: '1st' },
  { key: 'vanna' as const, label: 'Vanna', order: '2nd' },
  { key: 'volga' as const, label: 'Volga', order: '2nd' },
];

// Table rows: factors + totals.
const tableRows = computed(() => {
  if (!result.value) return [];
  return result.value.factors.map((entry) => ({
    label: `${entry.factor.factorType}:${entry.factor.name}`,
    factorType: entry.factor.factorType,
    greeks: entry.greeks,
    isTotals: false,
  }));
});

const totalsRow = computed(() => {
  if (!result.value) return null;
  return {
    label: 'Totals',
    factorType: '',
    greeks: result.value.totals,
    isTotals: true,
  };
});

function formatGreek(val: number | null | undefined): string {
  if (val === null || val === undefined) return 'N/A';
  return val.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 6 });
}

function greekColor(val: number | null | undefined): string {
  if (val === null || val === undefined) return 'text-medium-emphasis';
  if (val > 0) return 'text-success';
  if (val < 0) return 'text-error';
  return '';
}

function getGreekValue(greeks: FactorGreeks, key: keyof FactorGreeks): number | null {
  return (greeks[key] as number | null | undefined) ?? null;
}

function factorTypeChipColor(factorType: string): string {
  switch (factorType) {
    case 'Curve': return 'primary';
    case 'Underlying': return 'warning';
    case 'VolSurface': return 'secondary';
    case 'Time': return 'info';
    default: return 'default';
  }
}
</script>

<template>
  <v-container fluid class="pa-4">
    <v-row>
      <!-- Left Panel -->
      <v-col cols="4">
        <div class="glass-card p-4 mb-4">
          <div class="section-header" style="margin-top: 0">
            History
            <v-chip v-if="store.resultHistory.length > 0" size="x-small" class="ml-2">
              {{ store.resultHistory.length }}
            </v-chip>
          </div>
          <div class="config-grid">
            <div class="grid-label">Entry</div>
            <div class="grid-input">
              <v-select
                v-model="selectedEntryId"
                :items="historyItems"
                item-title="title"
                item-value="value"
                placeholder="Select a pricing result..."
                no-data-text="No pricing history"
                density="compact"
                variant="outlined"
                hide-details
                :disabled="historyItems.length === 0"
              />
            </div>
          </div>
          <div v-if="store.resultHistory.length === 0" class="text-caption text-text-muted mt-2">
            No pricing history. Run the Pricer first.
          </div>
        </div>

        <!-- Selected entry summary -->
        <div v-if="selectedEntry" class="glass-card p-4 mb-4">
          <div class="section-header" style="margin-top: 0">Summary</div>
          <div class="config-grid">
            <div class="grid-label">Instrument</div>
            <div class="grid-input">{{ selectedEntry.instrumentName }}</div>
            <div class="grid-label">PV</div>
            <div class="grid-input">{{ formatCurrency(selectedEntry.totalPv, selectedEntry.reportingCcy) }}</div>
            <div class="grid-label">Val Date</div>
            <div class="grid-input">{{ selectedEntry.valuationDate }}</div>
            <div class="grid-label">Currency</div>
            <div class="grid-input">{{ selectedEntry.reportingCcy }}</div>
            <div class="grid-label">Legs</div>
            <div class="grid-input">{{ selectedEntry.legs.length }}</div>
          </div>
        </div>

        <!-- Bump configuration -->
        <div class="glass-card p-4 mb-4">
          <div class="section-header" style="margin-top: 0">Bump Configuration</div>
            <div class="config-grid">
              <div class="grid-label">Rate (abs)</div>
              <div class="grid-input">
                <v-text-field
                  v-model.number="rateBump"
                  type="number"
                  step="0.0001"
                  density="compact"
                  variant="outlined"
                  hide-details
                  suffix="(1bp = 0.0001)"
                />
              </div>
              <div class="grid-label">Vol (abs)</div>
              <div class="grid-input">
                <v-text-field
                  v-model.number="volBump"
                  type="number"
                  step="0.01"
                  density="compact"
                  variant="outlined"
                  hide-details
                />
              </div>
              <div class="grid-label">Time (yrs)</div>
              <div class="grid-input">
                <v-text-field
                  v-model.number="timeBump"
                  type="number"
                  step="0.001"
                  density="compact"
                  variant="outlined"
                  hide-details
                  suffix="(1d ~ 0.00274)"
                />
              </div>
              <div class="grid-label">Spot (rel)</div>
              <div class="grid-input">
                <v-text-field
                  v-model.number="spotBump"
                  type="number"
                  step="0.01"
                  density="compact"
                  variant="outlined"
                  hide-details
                  suffix="(1% = 0.01)"
                />
              </div>
              <div class="grid-label">Mode</div>
              <div class="grid-input">
                <v-select
                  v-model="greeksMode"
                  :items="[
                    { title: 'Bump & Revalue', value: 'bumpRevalue' },
                    { title: 'Enzyme AAD', value: 'enzymeAad' },
                  ]"
                  density="compact"
                  variant="outlined"
                  hide-details
                />
              </div>
            </div>
        </div>

        <v-btn
          color="primary"
          block
          :loading="isComputing"
          :disabled="!selectedEntry || isComputing"
          @click="compute"
        >
          Compute Greeks
        </v-btn>

        <v-alert v-if="error" type="error" variant="tonal" density="compact" class="mt-3">
          {{ error }}
        </v-alert>
      </v-col>

      <!-- Right Panel -->
      <v-col cols="8">
        <!-- Empty state -->
        <div v-if="!result" class="d-flex align-center justify-center" style="min-height: 400px">
          <div class="text-center text-medium-emphasis">
            <v-icon size="64" class="mb-4">mdi-chart-bell-curve-cumulative</v-icon>
            <div class="text-h6">Greeks Analyser</div>
            <div class="text-body-2 mt-1">
              Select a pricing result from history and click Compute.
            </div>
          </div>
        </div>

        <!-- Results -->
        <template v-else>
          <!-- Summary row -->
          <div class="glass-card p-4 mb-4">
              <div class="d-flex flex-wrap ga-6">
                <div>
                  <span class="stat-label">Price</span>
                  <div class="stat-value">{{ formatCurrency(result.price, result.currency) }}</div>
                </div>
                <div>
                  <span class="stat-label">Currency</span>
                  <div class="stat-value">{{ result.currency }}</div>
                </div>
                <div>
                  <span class="stat-label">Mode</span>
                  <div class="stat-value">{{ result.mode }}</div>
                </div>
                <div>
                  <span class="stat-label">Time</span>
                  <div class="stat-value">{{ result.computationTimeMs.toFixed(2) }} ms</div>
                </div>
                <div>
                  <span class="stat-label">Factors</span>
                  <div class="stat-value">{{ result.factors.length }}</div>
                </div>
              </div>
          </div>

          <!-- Factor x Greek table -->
          <div class="glass-card p-4">
            <div class="text-xs font-semibold uppercase tracking-wider text-text-muted mb-2">Greeks by Risk Factor</div>
            <v-table density="compact" class="greeks-table">
              <thead>
                <tr>
                  <th class="text-left">Factor</th>
                  <th
                    v-for="col in greekColumns"
                    :key="col.key"
                    class="text-right"
                  >
                    <span>{{ col.label }}</span>
                    <v-chip
                      size="x-small"
                      :color="col.order === '1st' ? 'primary' : 'secondary'"
                      variant="tonal"
                      class="ml-1"
                    >
                      {{ col.order }}
                    </v-chip>
                  </th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="row in tableRows" :key="row.label">
                  <td>
                    <v-chip
                      size="small"
                      :color="factorTypeChipColor(row.factorType)"
                      variant="tonal"
                    >
                      {{ row.label }}
                    </v-chip>
                  </td>
                  <td
                    v-for="col in greekColumns"
                    :key="col.key"
                    class="text-right font-weight-medium"
                    :class="greekColor(getGreekValue(row.greeks, col.key))"
                  >
                    {{ formatGreek(getGreekValue(row.greeks, col.key)) }}
                  </td>
                </tr>
                <!-- Totals row -->
                <tr v-if="totalsRow" class="totals-row">
                  <td class="font-weight-bold">Totals</td>
                  <td
                    v-for="col in greekColumns"
                    :key="col.key"
                    class="text-right font-weight-bold"
                    :class="greekColor(getGreekValue(totalsRow.greeks, col.key))"
                  >
                    {{ formatGreek(getGreekValue(totalsRow.greeks, col.key)) }}
                  </td>
                </tr>
              </tbody>
            </v-table>
          </div>
        </template>
      </v-col>
    </v-row>
  </v-container>
</template>

<style scoped>
.greeks-table th {
  white-space: nowrap;
}

.totals-row {
  border-top: 2px solid var(--border);
}
</style>
