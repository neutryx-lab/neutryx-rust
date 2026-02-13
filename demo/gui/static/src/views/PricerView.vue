<script setup lang="ts">
/**
 * PricerView — Orchestrator (Vuetify Material UI)
 *
 * Flexible table-based pricer layout using Vuetify v-data-table,
 * v-card, v-expansion-panels, and Material Design components.
 *
 * Tabs: "Standard" (original pricer) | "Exotic Products" (dynamic exotic pricing)
 */
import { onMounted, ref, computed, reactive } from 'vue';
import { usePricerStore } from '@/stores/pricer';
import { useInstruments } from '@/composables/useInstruments';
import { usePricer } from '@/composables/usePricer';

import PricerSummaryBar from '@/components/pricer/PricerSummaryBar.vue';
import PricerConfigPanel from '@/components/pricer/PricerConfigPanel.vue';
import PricerResultsPanel from '@/components/pricer/PricerResultsPanel.vue';
import CashflowTable from '@/components/pricer/CashflowTable.vue';
import PricerHistory from '@/components/pricer/PricerHistory.vue';
import DynamicParamField from '@/components/pricer/DynamicParamField.vue';

import type { ExoticProductDef, ExoticPricingResponse } from '@/types';
import { fetchExoticProducts, priceExotic } from '@/services/api';

const store = usePricerStore();
const { loadInstruments } = useInstruments();
const { expandCashflows } = usePricer();

// ---------------------------------------------------------------------------
// Tab state
// ---------------------------------------------------------------------------
const activeTab = ref<'standard' | 'exotic'>('standard');

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

const selectedProduct = computed(() =>
  exoticProducts.value.find((p) => p.productType === selectedProductType.value) ?? null,
);

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------
onMounted(async () => {
  await loadInstruments();
  if (store.selectedInstrumentId && store.instruments.length > 0) {
    expandCashflows();
  }
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

function onTabChange(tab: 'standard' | 'exotic') {
  activeTab.value = tab;
  if (tab === 'exotic') {
    loadExoticProducts();
  }
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
      <!-- Tabs -->
      <v-tabs
        :model-value="activeTab"
        color="primary"
        class="mb-4"
        @update:model-value="onTabChange"
      >
        <v-tab value="standard">Standard</v-tab>
        <v-tab value="exotic">Exotic Products</v-tab>
      </v-tabs>

      <!-- Standard Tab -->
      <div v-if="activeTab === 'standard'">
        <v-row>
          <!-- Left Panel: Config + Results (4 cols) -->
          <v-col cols="12" lg="4">
            <div class="d-flex flex-column" style="gap: 16px">
              <PricerConfigPanel />
              <PricerResultsPanel />
              <PricerHistory />
            </div>
          </v-col>

          <!-- Right Panel: Cashflows (8 cols) -->
          <v-col cols="12" lg="8">
            <CashflowTable />
          </v-col>
        </v-row>
      </div>

      <!-- Exotic Products Tab -->
      <div v-if="activeTab === 'exotic'">
        <v-row>
          <!-- Left Panel: Product selection + parameters -->
          <v-col cols="12" lg="4">
            <v-card variant="outlined" class="mb-4">
              <v-card-title class="text-subtitle-1">Select Product</v-card-title>
              <v-card-text>
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
                  class="text-body-2 text-medium-emphasis mt-2"
                >
                  {{ selectedProduct.description }}
                </p>
              </v-card-text>
            </v-card>

            <!-- Parameter Form -->
            <v-card
              v-if="selectedProduct && selectedProduct.parameters.length > 0"
              variant="outlined"
              class="mb-4"
            >
              <v-card-title class="text-subtitle-1">Parameters</v-card-title>
              <v-card-text>
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
              </v-card-text>
            </v-card>
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
            <v-card v-if="exoticResult" variant="outlined">
              <v-card-title class="text-subtitle-1">Pricing Result</v-card-title>
              <v-card-text>
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
              </v-card-text>
            </v-card>

            <!-- Empty state -->
            <v-card
              v-else-if="!exoticError"
              variant="outlined"
              class="d-flex align-center justify-center"
              style="min-height: 200px"
            >
              <v-card-text class="text-center text-medium-emphasis">
                <v-icon icon="mdi-chart-bell-curve-cumulative" size="48" class="mb-2" />
                <p class="text-body-1">
                  Select an exotic product and configure its parameters to begin pricing.
                </p>
              </v-card-text>
            </v-card>
          </v-col>
        </v-row>
      </div>
    </template>
  </div>
</template>
