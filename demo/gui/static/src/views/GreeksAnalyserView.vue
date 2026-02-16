<script setup lang="ts">
/**
 * GreeksAnalyserView — Advanced Greeks analysis using saved Pricer history.
 *
 * Sends legs + bump config to /api/pricer/advanced-greeks and displays
 * 7 Greeks (delta, gamma, vega, theta, rho, vanna, volga) mirroring
 * pricer_risk::GreeksResult.
 */
import { ref, computed } from 'vue';
import { usePricerStore } from '@/stores/pricer';
import { computeAdvancedGreeks } from '@/services/api';
import type { AdvancedGreeksResult, AdvancedGreeksMode } from '@/types/api';
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

interface GreekCard {
  label: string;
  key: keyof AdvancedGreeksResult;
  order: '1st' | '2nd';
  description: string;
}

const greekCards: GreekCard[] = [
  { label: 'Delta', key: 'delta', order: '1st', description: 'dV/dr' },
  { label: 'Gamma', key: 'gamma', order: '2nd', description: 'd\u00B2V/dr\u00B2' },
  { label: 'Vega', key: 'vega', order: '1st', description: 'dV/d\u03C3' },
  { label: 'Theta', key: 'theta', order: '1st', description: 'dV/dt' },
  { label: 'Rho', key: 'rho', order: '1st', description: 'dV/dr (rate)' },
  { label: 'Vanna', key: 'vanna', order: '2nd', description: 'd\u00B2V/dr d\u03C3' },
  { label: 'Volga', key: 'volga', order: '2nd', description: 'd\u00B2V/d\u03C3\u00B2' },
];

function greekValue(card: GreekCard): number | null {
  if (!result.value) return null;
  return (result.value[card.key] as number | null | undefined) ?? null;
}

function formatGreek(val: number | null): string {
  if (val === null || val === undefined) return 'N/A';
  return val.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 6 });
}

function greekColor(val: number | null): string {
  if (val === null || val === undefined) return '';
  if (val > 0) return 'text-success';
  if (val < 0) return 'text-error';
  return '';
}
</script>

<template>
  <v-container fluid class="pa-4">
    <v-row>
      <!-- Left Panel -->
      <v-col cols="4">
        <v-card variant="outlined" class="mb-4">
          <v-card-title class="text-subtitle-1">History</v-card-title>
          <v-card-text>
            <v-select
              v-model="selectedEntryId"
              :items="historyItems"
              placeholder="Select a pricing result..."
              density="compact"
              variant="outlined"
              hide-details
              :disabled="store.resultHistory.length === 0"
            />
            <div v-if="store.resultHistory.length === 0" class="text-caption text-medium-emphasis mt-2">
              No pricing history. Run the Pricer first.
            </div>
          </v-card-text>
        </v-card>

        <!-- Selected entry summary -->
        <v-card v-if="selectedEntry" variant="outlined" class="mb-4">
          <v-card-title class="text-subtitle-1">Summary</v-card-title>
          <v-card-text>
            <div class="summary-grid">
              <span class="text-medium-emphasis">Instrument</span>
              <span>{{ selectedEntry.instrumentName }}</span>
              <span class="text-medium-emphasis">PV</span>
              <span>{{ formatCurrency(selectedEntry.totalPv, selectedEntry.reportingCcy) }}</span>
              <span class="text-medium-emphasis">Val Date</span>
              <span>{{ selectedEntry.valuationDate }}</span>
              <span class="text-medium-emphasis">Currency</span>
              <span>{{ selectedEntry.reportingCcy }}</span>
              <span class="text-medium-emphasis">Legs</span>
              <span>{{ selectedEntry.legs.length }}</span>
            </div>
          </v-card-text>
        </v-card>

        <!-- Bump configuration -->
        <v-card variant="outlined" class="mb-4">
          <v-card-title class="text-subtitle-1">Bump Configuration</v-card-title>
          <v-card-text>
            <div class="config-grid">
              <div class="config-label">Rate (abs)</div>
              <v-text-field
                v-model.number="rateBump"
                type="number"
                step="0.0001"
                density="compact"
                variant="outlined"
                hide-details
                suffix="(1bp = 0.0001)"
              />
              <div class="config-label">Vol (abs)</div>
              <v-text-field
                v-model.number="volBump"
                type="number"
                step="0.01"
                density="compact"
                variant="outlined"
                hide-details
              />
              <div class="config-label">Time (yrs)</div>
              <v-text-field
                v-model.number="timeBump"
                type="number"
                step="0.001"
                density="compact"
                variant="outlined"
                hide-details
                suffix="(1d ~ 0.00274)"
              />
              <div class="config-label">Spot (rel)</div>
              <v-text-field
                v-model.number="spotBump"
                type="number"
                step="0.01"
                density="compact"
                variant="outlined"
                hide-details
                suffix="(1% = 0.01)"
              />
              <div class="config-label">Mode</div>
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
          </v-card-text>
        </v-card>

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
          <v-card variant="outlined" class="mb-4">
            <v-card-text>
              <div class="d-flex flex-wrap ga-6">
                <div>
                  <span class="text-caption text-medium-emphasis">Price</span>
                  <div class="text-h6">{{ formatCurrency(result.price, result.currency) }}</div>
                </div>
                <div>
                  <span class="text-caption text-medium-emphasis">Currency</span>
                  <div class="text-h6">{{ result.currency }}</div>
                </div>
                <div>
                  <span class="text-caption text-medium-emphasis">Mode</span>
                  <div class="text-h6">{{ result.mode }}</div>
                </div>
                <div>
                  <span class="text-caption text-medium-emphasis">Time</span>
                  <div class="text-h6">{{ result.computationTimeMs.toFixed(2) }} ms</div>
                </div>
                <div v-if="result.stdError != null">
                  <span class="text-caption text-medium-emphasis">Std Error</span>
                  <div class="text-h6">{{ result.stdError.toFixed(6) }}</div>
                </div>
              </div>
            </v-card-text>
          </v-card>

          <!-- Greeks cards -->
          <v-row>
            <v-col v-for="card in greekCards" :key="card.label" cols="12" sm="6" md="4" lg="3">
              <v-card variant="outlined" class="greek-card">
                <v-card-text class="text-center">
                  <div class="d-flex align-center justify-center ga-2 mb-2">
                    <span class="text-subtitle-1 font-weight-medium">{{ card.label }}</span>
                    <v-chip
                      size="x-small"
                      :color="card.order === '1st' ? 'primary' : 'secondary'"
                      variant="tonal"
                    >
                      {{ card.order }}
                    </v-chip>
                  </div>
                  <div
                    class="text-h5 font-weight-bold"
                    :class="greekColor(greekValue(card))"
                  >
                    {{ formatGreek(greekValue(card)) }}
                  </div>
                  <div class="text-caption text-medium-emphasis mt-1">
                    {{ card.description }}
                  </div>
                </v-card-text>
              </v-card>
            </v-col>
          </v-row>

          <!-- Confidence interval -->
          <v-card v-if="result.confidence95" variant="outlined" class="mt-4">
            <v-card-text>
              <span class="text-caption text-medium-emphasis">95% Confidence Interval: </span>
              <span>[{{ result.confidence95[0].toFixed(4) }}, {{ result.confidence95[1].toFixed(4) }}]</span>
            </v-card-text>
          </v-card>
        </template>
      </v-col>
    </v-row>
  </v-container>
</template>

<style scoped>
.summary-grid {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 4px 12px;
  font-size: 0.85rem;
}

.config-grid {
  display: grid;
  grid-template-columns: 80px 1fr;
  align-items: center;
  gap: 8px;
}

.config-label {
  font-size: 0.8rem;
  color: rgba(var(--v-theme-on-surface), 0.7);
  text-align: right;
  white-space: nowrap;
}

.greek-card {
  transition: border-color 0.2s;
}

.greek-card:hover {
  border-color: rgb(var(--v-theme-primary));
}
</style>
