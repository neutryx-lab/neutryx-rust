<script setup lang="ts">
/**
 * PricerView — Unified pricer layout.
 *
 * Standard pricer at top, with Exotic Products and Markov Functional
 * available as collapsible expansion panels below.
 */
import { onMounted, ref, computed, reactive } from 'vue';
import { usePricerStore } from '@/stores/pricer';
import { useInstruments } from '@/composables/useInstruments';
import { usePricer } from '@/composables/usePricer';

import PricerSummaryBar from '@/components/pricer/PricerSummaryBar.vue';
import PricerConfigPanel from '@/components/pricer/PricerConfigPanel.vue';
import PricerResultsPanel from '@/components/pricer/PricerResultsPanel.vue';
import CashflowTable from '@/components/pricer/CashflowTable.vue';

import DynamicParamField from '@/components/pricer/DynamicParamField.vue';

import MfmView from '@/views/MfmView.vue';
import JyInstrumentPanel from '@/components/jy/JyInstrumentPanel.vue';
import JyPricingPanel from '@/components/jy/JyPricingPanel.vue';
import JyXvaPanel from '@/components/jy/JyXvaPanel.vue';

import type { ExoticProductDef, ExoticPricingResponse } from '@/types';
import { fetchExoticProducts, priceExotic } from '@/services/api';
import { useJyInflationStore } from '@/stores/jyInflation';
import { useJYInflation } from '@/composables/useJYInflation';

const store = usePricerStore();
const { loadInstruments } = useInstruments();
usePricer();

// ---------------------------------------------------------------------------
// Exotic Products state
// ---------------------------------------------------------------------------
const exoticProducts = ref<ExoticProductDef[]>([]);
const selectedProductType = ref<string | null>(null);
const exoticParams = reactive<Record<string, any>>({});
const exoticResult = ref<ExoticPricingResponse | null>(null);
const exoticLoading = ref(false);
const exoticError = ref<string | null>(null);
const productsLoading = ref(false);
const exoticExpanded = ref(false);
const mfmExpanded = ref(false);
const inflationExpanded = ref(false);

// ---------------------------------------------------------------------------
// JY Inflation state
// ---------------------------------------------------------------------------
const jyStore = useJyInflationStore();
const { generateCashflows: jyGenerateCashflows, runPricing: jyRunPricing, runXva: jyRunXva } = useJYInflation();

const selectedProduct = computed(() =>
  exoticProducts.value.find((p) => p.productType === selectedProductType.value) ?? null,
);

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------
onMounted(() => {
  loadInstruments();
});

// ---------------------------------------------------------------------------
// Exotic helpers
// ---------------------------------------------------------------------------
async function loadExoticProducts() {
  if (exoticProducts.value.length > 0) return;
  productsLoading.value = true;
  try {
    exoticProducts.value = await fetchExoticProducts();
  } catch (err: any) {
    exoticError.value = err.message ?? 'Failed to load exotic products';
  } finally {
    productsLoading.value = false;
  }
}

function selectProduct(productType: string) {
  selectedProductType.value = productType;
  exoticResult.value = null;
  exoticError.value = null;

  // Reset params and apply defaults
  Object.keys(exoticParams).forEach((key) => delete exoticParams[key]);
  const product = exoticProducts.value.find((p) => p.productType === productType);
  if (product) {
    for (const param of product.parameters) {
      if (param.defaultValue !== undefined) {
        exoticParams[param.name] = param.defaultValue;
      }
    }
  }
}

async function submitExoticPricing() {
  if (!selectedProductType.value) return;
  exoticLoading.value = true;
  exoticError.value = null;
  exoticResult.value = null;
  try {
    const request = {
      productType: selectedProductType.value,
      ...exoticParams,
    };
    exoticResult.value = await priceExotic(request);
  } catch (err: any) {
    exoticError.value = err.message ?? 'Pricing failed';
  } finally {
    exoticLoading.value = false;
  }
}

function onExoticToggle(expanded: boolean) {
  if (expanded) loadExoticProducts();
}
</script>

<template>
  <div class="pricer-view">
    <!-- Summary Bar -->
    <PricerSummaryBar />

    <!-- API Not Available Fallback -->
    <v-alert v-if="!store.apiAvailable" type="info" variant="tonal" class="mb-6">
      Pricer API is not available in this build configuration.
    </v-alert>

    <!-- Main Layout -->
    <template v-else>
      <!-- Standard Pricer -->
      <v-row>
        <!-- Left Panel: Config (4 cols) -->
        <v-col cols="12" lg="4">
          <PricerConfigPanel />
        </v-col>

        <!-- Right Panel: Cashflows (8 cols) -->
        <v-col cols="12" lg="8">
          <CashflowTable />
        </v-col>
      </v-row>

      <!-- Results (below, full width) -->
      <v-row class="mt-2">
        <v-col cols="12">
          <PricerResultsPanel />
        </v-col>
      </v-row>

      <!-- Exotic Products (collapsible) -->
      <v-row class="mt-4">
        <v-col cols="12">
          <v-expansion-panels v-model="exoticExpanded" @update:model-value="onExoticToggle(!!exoticExpanded)">
            <v-expansion-panel value="exotic">
              <v-expansion-panel-title>
                <div class="d-flex align-center gap-2">
                  <v-icon icon="mdi-chart-bell-curve-cumulative" size="20" />
                  <span class="text-subtitle-1 font-weight-medium">Exotic Products</span>
                </div>
              </v-expansion-panel-title>
              <v-expansion-panel-text>
                <v-row>
                  <!-- Left Panel: Product selection + parameters -->
                  <v-col cols="12" lg="4">
                    <div class="glass-card p-4 mb-4">
                      <div class="text-xs font-semibold uppercase tracking-wider text-text-muted mb-2">Select Product</div>
                        <v-progress-linear
                          v-if="productsLoading"
                          indeterminate
                          color="primary"
                          class="mb-4"
                        />

                        <v-alert
                          v-if="exoticError && !selectedProductType"
                          type="error"
                          variant="tonal"
                          density="compact"
                          class="mb-3"
                        >
                          {{ exoticError }}
                        </v-alert>

                        <v-select
                          v-if="exoticProducts.length > 0"
                          :model-value="selectedProductType"
                          :items="exoticProducts"
                          item-title="displayName"
                          item-value="productType"
                          label="Product Type"
                          variant="outlined"
                          density="compact"
                          @update:model-value="selectProduct"
                        />

                        <p
                          v-if="selectedProduct"
                          class="text-body-2 mt-2"
                          style="color: var(--text-muted)"
                        >
                          {{ selectedProduct.description }}
                        </p>
                    </div>

                    <!-- Parameter Form -->
                    <div
                      v-if="selectedProduct && selectedProduct.parameters.length > 0"
                      class="glass-card p-4 mb-4"
                    >
                      <div class="text-xs font-semibold uppercase tracking-wider text-text-muted mb-2">Parameters</div>
                        <DynamicParamField
                          v-for="param in selectedProduct.parameters"
                          :key="param.name"
                          :param="param"
                          :model-value="exoticParams[param.name]"
                          @update:model-value="exoticParams[param.name] = $event"
                        />

                        <v-btn
                          color="primary"
                          block
                          :loading="exoticLoading"
                          :disabled="!selectedProductType || exoticLoading"
                          class="mt-4"
                          @click="submitExoticPricing"
                        >
                          Price
                        </v-btn>
                    </div>
                  </v-col>

                  <!-- Right Panel: Results -->
                  <v-col cols="12" lg="8">
                    <!-- Error Alert -->
                    <v-alert
                      v-if="exoticError && selectedProductType"
                      type="error"
                      variant="tonal"
                      closable
                      class="mb-4"
                      @click:close="exoticError = null"
                    >
                      {{ exoticError }}
                    </v-alert>

                    <!-- Results Card -->
                    <div v-if="exoticResult" class="glass-card p-4">
                      <div class="text-xs font-semibold uppercase tracking-wider text-text-muted mb-2">Pricing Result</div>
                        <v-table density="compact">
                          <tbody>
                            <tr>
                              <td class="font-weight-medium">Product</td>
                              <td>{{ exoticResult.productType }}</td>
                            </tr>
                            <tr>
                              <td class="font-weight-medium">Price</td>
                              <td class="text-h6">
                                {{ exoticResult.price.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 6 }) }}
                                {{ exoticResult.currency }}
                              </td>
                            </tr>
                            <tr>
                              <td class="font-weight-medium">Calculation Time</td>
                              <td>{{ exoticResult.calculationTimeMs.toFixed(1) }} ms</td>
                            </tr>
                          </tbody>
                        </v-table>

                        <!-- Monte Carlo Statistics -->
                        <template v-if="exoticResult.mcStats">
                          <v-divider class="my-3" />
                          <p class="text-subtitle-2 mb-2">Monte Carlo Statistics</p>
                          <v-table density="compact">
                            <tbody>
                              <tr>
                                <td class="font-weight-medium">Paths</td>
                                <td>{{ exoticResult.mcStats.numPaths.toLocaleString() }}</td>
                              </tr>
                              <tr>
                                <td class="font-weight-medium">Std Error</td>
                                <td>{{ exoticResult.mcStats.stdError.toFixed(6) }}</td>
                              </tr>
                              <tr>
                                <td class="font-weight-medium">95% Confidence</td>
                                <td>
                                  [{{ exoticResult.mcStats.confidence95[0].toFixed(4) }},
                                  {{ exoticResult.mcStats.confidence95[1].toFixed(4) }}]
                                </td>
                              </tr>
                            </tbody>
                          </v-table>
                        </template>
                    </div>

                    <!-- Empty state -->
                    <div
                      v-else-if="!exoticError"
                      class="glass-card d-flex align-center justify-center"
                      style="min-height: 200px"
                    >
                      <div class="text-center" style="color: var(--text-muted)">
                        <v-icon icon="mdi-chart-bell-curve-cumulative" size="48" class="mb-2" />
                        <p class="text-body-1">
                          Select an exotic product and configure its parameters to begin pricing.
                        </p>
                      </div>
                    </div>
                  </v-col>
                </v-row>
              </v-expansion-panel-text>
            </v-expansion-panel>
          </v-expansion-panels>
        </v-col>
      </v-row>

      <!-- Markov Functional Model (collapsible) -->
      <v-row class="mt-2">
        <v-col cols="12">
          <v-expansion-panels v-model="mfmExpanded">
            <v-expansion-panel value="mfm">
              <v-expansion-panel-title>
                <div class="d-flex align-center gap-2">
                  <v-icon icon="mdi-atom" size="20" />
                  <span class="text-subtitle-1 font-weight-medium">Markov Functional Model</span>
                </div>
              </v-expansion-panel-title>
              <v-expansion-panel-text>
                <MfmView />
              </v-expansion-panel-text>
            </v-expansion-panel>
          </v-expansion-panels>
        </v-col>
      </v-row>

      <!-- Inflation Swaps (collapsible) -->
      <v-row class="mt-2">
        <v-col cols="12">
          <v-expansion-panels v-model="inflationExpanded">
            <v-expansion-panel value="inflation">
              <v-expansion-panel-title>
                <div class="d-flex align-center gap-2">
                  <v-icon icon="mdi-chart-bar" size="20" />
                  <span class="text-subtitle-1 font-weight-medium">Inflation Swaps (JY Model)</span>
                </div>
              </v-expansion-panel-title>
              <v-expansion-panel-text>
                <!-- Instrument Configuration + Cashflows -->
                <JyInstrumentPanel @generate="jyGenerateCashflows" />

                <!-- Pricing -->
                <div class="mt-6">
                  <div class="d-flex align-center gap-2 mb-4">
                    <v-btn
                      color="primary"
                      :loading="jyStore.loading"
                      :disabled="jyStore.loading"
                      @click="jyRunPricing"
                    >
                      <v-icon start>mdi-calculator</v-icon>
                      Price
                    </v-btn>
                    <v-btn
                      color="secondary"
                      variant="outlined"
                      :loading="jyStore.loading"
                      :disabled="jyStore.loading"
                      @click="jyRunXva"
                    >
                      <v-icon start>mdi-shield-half-full</v-icon>
                      Compute XVA
                    </v-btn>
                  </div>
                  <JyPricingPanel :result="jyStore.pricingResult" />
                </div>

                <!-- XVA -->
                <div v-if="jyStore.xvaResult" class="mt-6">
                  <JyXvaPanel :result="jyStore.xvaResult" />
                </div>
              </v-expansion-panel-text>
            </v-expansion-panel>
          </v-expansion-panels>
        </v-col>
      </v-row>
    </template>
  </div>
</template>
