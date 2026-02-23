<script setup lang="ts">
import { ref, reactive, computed } from 'vue';
import {
  calibrateMfm,
  buildGaussianTree,
  priceBermudan,
  priceTarn,
} from '@/services/api';
import type {
  MfmCalibrateResponse,
  GaussianTreeResponse,
  BermudanPriceResponse,
  TarnPriceResponse,
} from '@/types';

// ── Active Tab ──
const activeTab = ref('calibration');

// ── Loading States ──
const loading = reactive({
  calibration: false,
  tree: false,
  bermudan: false,
  tarn: false,
});

// ── Error State ──
const error = ref<string | null>(null);

// ── Shared Model Parameters ──
const modelParams = reactive({
  meanReversion: 0.03,
  volatility: 0.01,
  numGridPoints: 41,
  numStdDevs: 5.0,
  volType: 'normal' as 'normal' | 'lognormal',
  fundingRate: 0.03,
  couponRate: 0.035,
  normalVolBp: 80.0,
});

// ── Schedule Parameters ──
const scheduleParams = reactive({
  numExercises: 5,
  swapTenor: 5.0,
  paymentFreq: 0.5,
});

// ── Bermudan Parameters ──
const bermudanParams = reactive({
  isCallable: true,
  flatCoupon: 0.01,
});

// ── TARN Parameters ──
const tarnParams = reactive({
  tarnAmount: 0.10,
  numCouponGridPoints: 10,
  excessCouponFlag: false,
  hasBermudanExercise: false,
  flatCoupon: 0.02,
});

// ── Gaussian Tree Parameters ──
const treeParams = reactive({
  numSteps: 5,
  maturity: 5.0,
  numGridPoints: 21,
});

// ── Results ──
const calibrationResult = ref<MfmCalibrateResponse | null>(null);
const treeResult = ref<GaussianTreeResponse | null>(null);
const bermudanResult = ref<BermudanPriceResponse | null>(null);
const tarnResult = ref<TarnPriceResponse | null>(null);

// ── Computed: exercise times from schedule ──
function generateExerciseTimes(): number[] {
  const times: number[] = [];
  for (let i = 1; i <= scheduleParams.numExercises; i++) {
    times.push(i);
  }
  return times;
}

function generateSwapTenors(): number[] {
  return new Array(scheduleParams.numExercises).fill(scheduleParams.swapTenor);
}

function generatePaymentFreqs(): number[] {
  return new Array(scheduleParams.numExercises).fill(scheduleParams.paymentFreq);
}

// ── Calibration Selected Slice ──
const selectedCalibSliceIdx = ref(0);
const selectedCalibRateIndex = ref<'funding' | 'couponSwap' | 'couponLibor'>('funding');

const selectedCalibration = computed(() => {
  if (!calibrationResult.value) return null;
  switch (selectedCalibRateIndex.value) {
    case 'funding': return calibrationResult.value.fundingCalibration;
    case 'couponSwap': return calibrationResult.value.couponSwapCalibration;
    case 'couponLibor': return calibrationResult.value.couponLiborCalibration;
  }
});

const selectedSlice = computed(() => {
  if (!selectedCalibration.value) return null;
  const slices = selectedCalibration.value.slices;
  if (selectedCalibSliceIdx.value >= slices.length) return null;
  return slices[selectedCalibSliceIdx.value];
});

// ── Actions ──

async function runCalibration() {
  error.value = null;
  loading.calibration = true;
  try {
    calibrationResult.value = await calibrateMfm({
      meanReversion: modelParams.meanReversion,
      volatility: modelParams.volatility,
      numGridPoints: modelParams.numGridPoints,
      numStdDevs: modelParams.numStdDevs,
      volType: modelParams.volType,
      exerciseTimes: generateExerciseTimes(),
      swapTenors: generateSwapTenors(),
      paymentFrequencies: generatePaymentFreqs(),
      fundingCurve: { rate: modelParams.fundingRate },
      couponCurve: { rate: modelParams.couponRate },
      volSurfaceType: 'flat',
      flatVol: { normalVolBp: modelParams.normalVolBp },
    });
    selectedCalibSliceIdx.value = 0;
  } catch (e: any) {
    error.value = e.message || String(e);
  } finally {
    loading.calibration = false;
  }
}

async function runGaussianTree() {
  error.value = null;
  loading.tree = true;
  try {
    const times: number[] = [];
    const dt = treeParams.maturity / treeParams.numSteps;
    for (let i = 1; i <= treeParams.numSteps; i++) {
      times.push(i * dt);
    }
    treeResult.value = await buildGaussianTree({
      meanReversion: modelParams.meanReversion,
      volatility: modelParams.volatility,
      times,
      numStdDevs: modelParams.numStdDevs,
      numGridPoints: treeParams.numGridPoints,
    });
  } catch (e: any) {
    error.value = e.message || String(e);
  } finally {
    loading.tree = false;
  }
}

async function runBermudan() {
  error.value = null;
  loading.bermudan = true;
  try {
    bermudanResult.value = await priceBermudan({
      meanReversion: modelParams.meanReversion,
      volatility: modelParams.volatility,
      numGridPoints: modelParams.numGridPoints,
      numStdDevs: modelParams.numStdDevs,
      volType: modelParams.volType,
      exerciseTimes: generateExerciseTimes(),
      swapTenors: generateSwapTenors(),
      paymentFrequencies: generatePaymentFreqs(),
      fundingCurve: { rate: modelParams.fundingRate },
      couponCurve: { rate: modelParams.couponRate },
      volSurfaceType: 'flat',
      flatVol: { normalVolBp: modelParams.normalVolBp },
      isCallable: bermudanParams.isCallable,
      flatCoupon: bermudanParams.flatCoupon,
    });
  } catch (e: any) {
    error.value = e.message || String(e);
  } finally {
    loading.bermudan = false;
  }
}

async function runTarn() {
  error.value = null;
  loading.tarn = true;
  try {
    tarnResult.value = await priceTarn({
      meanReversion: modelParams.meanReversion,
      volatility: modelParams.volatility,
      numGridPoints: modelParams.numGridPoints,
      numStdDevs: modelParams.numStdDevs,
      volType: modelParams.volType,
      exerciseTimes: generateExerciseTimes(),
      swapTenors: generateSwapTenors(),
      paymentFrequencies: generatePaymentFreqs(),
      fundingCurve: { rate: modelParams.fundingRate },
      couponCurve: { rate: modelParams.couponRate },
      volSurfaceType: 'flat',
      flatVol: { normalVolBp: modelParams.normalVolBp },
      tarnAmount: tarnParams.tarnAmount,
      numCouponGridPoints: tarnParams.numCouponGridPoints,
      excessCouponFlag: tarnParams.excessCouponFlag,
      hasBermudanExercise: tarnParams.hasBermudanExercise,
      flatCoupon: tarnParams.flatCoupon,
    });
  } catch (e: any) {
    error.value = e.message || String(e);
  } finally {
    loading.tarn = false;
  }
}

function formatNumber(n: number | undefined | null, digits = 6): string {
  if (n === undefined || n === null) return '-';
  return n.toFixed(digits);
}

function formatBps(n: number): string {
  return (n * 10000).toFixed(2) + ' bp';
}

function formatPct(n: number): string {
  return (n * 100).toFixed(4) + '%';
}

function formatMs(n: number): string {
  return n.toFixed(1) + ' ms';
}
</script>

<template>
  <v-container fluid class="pa-4">
    <!-- Header -->
    <v-row class="mb-4">
      <v-col>
        <h1 class="text-h4 font-weight-bold">1F Markov Functional Model</h1>
        <p class="text-subtitle-1 text-medium-emphasis">
          Non-parametric calibration, Gaussian tree, Bermudan swaption & TARN pricing
        </p>
      </v-col>
    </v-row>

    <!-- Error Alert -->
    <v-alert v-if="error" type="error" closable class="mb-4" @click:close="error = null">
      {{ error }}
    </v-alert>

    <!-- Tabs -->
    <v-tabs v-model="activeTab" color="primary" class="mb-4">
      <v-tab value="calibration">MFM Calibration</v-tab>
      <v-tab value="tree">Gaussian Tree</v-tab>
      <v-tab value="bermudan">Bermudan Swaption</v-tab>
      <v-tab value="tarn">TARN</v-tab>
    </v-tabs>

    <v-window v-model="activeTab">
      <!-- ═══════════════════════════════════════════════════════════════ -->
      <!-- TAB 1: MFM CALIBRATION -->
      <!-- ═══════════════════════════════════════════════════════════════ -->
      <v-window-item value="calibration">
        <v-row>
          <!-- Left: Parameters -->
          <v-col cols="12" md="4">
            <v-card>
              <v-card-title>Model Parameters</v-card-title>
              <v-card-text>
                <v-text-field v-model.number="modelParams.meanReversion" label="Mean Reversion (a)" type="number" step="0.001" density="compact" class="mb-2" />
                <v-text-field v-model.number="modelParams.volatility" label="Gaussian Vol (σ)" type="number" step="0.001" density="compact" class="mb-2" />
                <v-text-field v-model.number="modelParams.numGridPoints" label="Grid Points" type="number" step="2" density="compact" class="mb-2" />
                <v-text-field v-model.number="modelParams.numStdDevs" label="Std Devs" type="number" step="0.5" density="compact" class="mb-2" />
                <v-select v-model="modelParams.volType" :items="['normal', 'lognormal']" label="Vol Type" density="compact" class="mb-2" />

                <v-divider class="my-3" />
                <div class="text-subtitle-2 mb-2">Curves</div>
                <v-text-field v-model.number="modelParams.fundingRate" label="Funding Rate (OIS)" type="number" step="0.005" density="compact" class="mb-2" />
                <v-text-field v-model.number="modelParams.couponRate" label="Coupon Rate (Libor)" type="number" step="0.005" density="compact" class="mb-2" />

                <v-divider class="my-3" />
                <div class="text-subtitle-2 mb-2">Volatility</div>
                <v-text-field v-model.number="modelParams.normalVolBp" label="Normal Vol (bp)" type="number" step="5" density="compact" class="mb-2" />

                <v-divider class="my-3" />
                <div class="text-subtitle-2 mb-2">Schedule</div>
                <v-text-field v-model.number="scheduleParams.numExercises" label="Exercise Dates" type="number" step="1" density="compact" class="mb-2" />
                <v-text-field v-model.number="scheduleParams.swapTenor" label="Swap Tenor (yrs)" type="number" step="1" density="compact" class="mb-2" />
                <v-text-field v-model.number="scheduleParams.paymentFreq" label="Payment Freq (yf)" type="number" step="0.25" density="compact" class="mb-2" />
              </v-card-text>
              <v-card-actions>
                <v-btn color="primary" block :loading="loading.calibration" @click="runCalibration">
                  Calibrate MFM
                </v-btn>
              </v-card-actions>
            </v-card>
          </v-col>

          <!-- Right: Results -->
          <v-col cols="12" md="8">
            <v-card v-if="calibrationResult">
              <v-card-title class="d-flex align-center">
                Calibration Results
                <v-spacer />
                <v-chip color="success" size="small" class="mr-2">
                  {{ formatMs(calibrationResult.computationTimeMs) }}
                </v-chip>
              </v-card-title>
              <v-card-text>
                <!-- Summary -->
                <v-row class="mb-3">
                  <v-col cols="4">
                    <v-card variant="outlined" class="pa-3 text-center">
                      <div class="text-caption text-medium-emphasis">Max NR Iterations</div>
                      <div class="text-h6">{{ calibrationResult.maxNrIterationsUsed }}</div>
                    </v-card>
                  </v-col>
                  <v-col cols="4">
                    <v-card variant="outlined" class="pa-3 text-center">
                      <div class="text-caption text-medium-emphasis">Max Calibration Error</div>
                      <div class="text-h6">{{ formatNumber(calibrationResult.maxCalibrationError, 2) }}</div>
                    </v-card>
                  </v-col>
                  <v-col cols="4">
                    <v-card variant="outlined" class="pa-3 text-center">
                      <div class="text-caption text-medium-emphasis">Exercise Dates</div>
                      <div class="text-h6">{{ calibrationResult.fundingCalibration.slices.length }}</div>
                    </v-card>
                  </v-col>
                </v-row>

                <!-- Rate Index Selector -->
                <v-btn-toggle v-model="selectedCalibRateIndex" mandatory color="primary" class="mb-3">
                  <v-btn value="funding" size="small">Funding</v-btn>
                  <v-btn value="couponSwap" size="small">Coupon Swap</v-btn>
                  <v-btn value="couponLibor" size="small">Coupon Libor</v-btn>
                </v-btn-toggle>

                <!-- Slice Selector -->
                <v-slider
                  v-if="selectedCalibration"
                  v-model="selectedCalibSliceIdx"
                  :min="0"
                  :max="Math.max(0, selectedCalibration.slices.length - 1)"
                  :step="1"
                  label="Exercise Date"
                  thumb-label
                  class="mb-3"
                />

                <!-- Slice Data Table -->
                <v-table v-if="selectedSlice" density="compact" fixed-header height="400">
                  <thead>
                    <tr>
                      <th>Node</th>
                      <th>x</th>
                      <th>Swap Rate</th>
                      <th>DF</th>
                      <th>Annuity</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="(x, j) in selectedSlice.xGrid" :key="j">
                      <td>{{ j }}</td>
                      <td>{{ formatNumber(x, 4) }}</td>
                      <td>{{ formatPct(selectedSlice.swapRates[j]) }}</td>
                      <td>{{ formatNumber(selectedSlice.discountFactors[j], 6) }}</td>
                      <td>{{ formatNumber(selectedSlice.annuities[j], 4) }}</td>
                    </tr>
                  </tbody>
                </v-table>

                <!-- Integral Adjuster -->
                <v-divider class="my-3" />
                <div class="text-subtitle-2 mb-2">Integral Adjuster (Moment Matching)</div>
                <v-table density="compact">
                  <thead>
                    <tr>
                      <th>Step</th>
                      <th>Additive Correction</th>
                      <th>Multiplicative Correction</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="(adder, i) in calibrationResult.adjuster.adders" :key="i">
                      <td>{{ i }}</td>
                      <td>{{ formatBps(adder) }}</td>
                      <td>{{ formatNumber(calibrationResult.adjuster.multipliers[i], 8) }}</td>
                    </tr>
                  </tbody>
                </v-table>
              </v-card-text>
            </v-card>
            <v-card v-else variant="outlined" class="d-flex align-center justify-center" min-height="400">
              <v-card-text class="text-center text-medium-emphasis">
                Configure model parameters and click "Calibrate MFM" to see results
              </v-card-text>
            </v-card>
          </v-col>
        </v-row>
      </v-window-item>

      <!-- ═══════════════════════════════════════════════════════════════ -->
      <!-- TAB 2: GAUSSIAN TREE -->
      <!-- ═══════════════════════════════════════════════════════════════ -->
      <v-window-item value="tree">
        <v-row>
          <v-col cols="12" md="4">
            <v-card>
              <v-card-title>Tree Parameters</v-card-title>
              <v-card-text>
                <v-text-field v-model.number="modelParams.meanReversion" label="Mean Reversion (a)" type="number" step="0.001" density="compact" class="mb-2" />
                <v-text-field v-model.number="modelParams.volatility" label="Gaussian Vol (σ)" type="number" step="0.001" density="compact" class="mb-2" />
                <v-text-field v-model.number="treeParams.numGridPoints" label="Grid Points" type="number" step="2" density="compact" class="mb-2" />
                <v-text-field v-model.number="modelParams.numStdDevs" label="Std Devs" type="number" step="0.5" density="compact" class="mb-2" />
                <v-text-field v-model.number="treeParams.numSteps" label="Time Steps" type="number" step="1" density="compact" class="mb-2" />
                <v-text-field v-model.number="treeParams.maturity" label="Maturity (years)" type="number" step="1" density="compact" class="mb-2" />
              </v-card-text>
              <v-card-actions>
                <v-btn color="primary" block :loading="loading.tree" @click="runGaussianTree">
                  Build Tree
                </v-btn>
              </v-card-actions>
            </v-card>
          </v-col>

          <v-col cols="12" md="8">
            <v-card v-if="treeResult">
              <v-card-title class="d-flex align-center">
                Gaussian Tree Structure
                <v-spacer />
                <v-chip color="success" size="small" class="mr-2">
                  {{ formatMs(treeResult.computationTimeMs) }}
                </v-chip>
              </v-card-title>
              <v-card-text>
                <v-row class="mb-3">
                  <v-col cols="4">
                    <v-card variant="outlined" class="pa-3 text-center">
                      <div class="text-caption text-medium-emphasis">Time Steps</div>
                      <div class="text-h6">{{ treeResult.numSteps }}</div>
                    </v-card>
                  </v-col>
                  <v-col cols="4">
                    <v-card variant="outlined" class="pa-3 text-center">
                      <div class="text-caption text-medium-emphasis">Nodes per Step</div>
                      <div class="text-h6">{{ treeResult.numNodes }}</div>
                    </v-card>
                  </v-col>
                  <v-col cols="4">
                    <v-card variant="outlined" class="pa-3 text-center">
                      <div class="text-caption text-medium-emphasis">Grid Spacing (dx)</div>
                      <div class="text-h6">{{ treeResult.slices.length > 0 ? formatNumber(treeResult.slices[0].dx, 6) : '-' }}</div>
                    </v-card>
                  </v-col>
                </v-row>

                <!-- Slice details -->
                <v-table density="compact" fixed-header height="300">
                  <thead>
                    <tr>
                      <th>Step</th>
                      <th>Time (yf)</th>
                      <th>Cond. Variance</th>
                      <th>dx</th>
                      <th>x_min</th>
                      <th>x_max</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="(slice, i) in treeResult.slices" :key="i">
                      <td>{{ i }}</td>
                      <td>{{ formatNumber(slice.time, 4) }}</td>
                      <td>{{ formatNumber(slice.conditionalVariance, 8) }}</td>
                      <td>{{ formatNumber(slice.dx, 6) }}</td>
                      <td>{{ formatNumber(slice.xGrid[0], 6) }}</td>
                      <td>{{ formatNumber(slice.xGrid[slice.xGrid.length - 1], 6) }}</td>
                    </tr>
                  </tbody>
                </v-table>

                <!-- Arrow-Debreu Prices -->
                <v-divider class="my-3" />
                <div class="text-subtitle-2 mb-2">Arrow-Debreu Prices (sum per step)</div>
                <v-table density="compact">
                  <thead>
                    <tr>
                      <th>Step</th>
                      <th>AD Sum</th>
                      <th>AD Center</th>
                      <th>AD Min</th>
                      <th>AD Max</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="(prices, i) in treeResult.arrowDebreuPrices" :key="i">
                      <td>{{ i }}</td>
                      <td>{{ formatNumber(prices.reduce((a: number, b: number) => a + b, 0), 8) }}</td>
                      <td>{{ formatNumber(prices[Math.floor(prices.length / 2)], 8) }}</td>
                      <td>{{ formatNumber(Math.min(...prices), 10) }}</td>
                      <td>{{ formatNumber(Math.max(...prices), 8) }}</td>
                    </tr>
                  </tbody>
                </v-table>
              </v-card-text>
            </v-card>
            <v-card v-else variant="outlined" class="d-flex align-center justify-center" min-height="400">
              <v-card-text class="text-center text-medium-emphasis">
                Configure tree parameters and click "Build Tree" to visualise the Gaussian lattice
              </v-card-text>
            </v-card>
          </v-col>
        </v-row>
      </v-window-item>

      <!-- ═══════════════════════════════════════════════════════════════ -->
      <!-- TAB 3: BERMUDAN SWAPTION -->
      <!-- ═══════════════════════════════════════════════════════════════ -->
      <v-window-item value="bermudan">
        <v-row>
          <v-col cols="12" md="4">
            <v-card>
              <v-card-title>Bermudan Parameters</v-card-title>
              <v-card-text>
                <div class="text-subtitle-2 mb-2">Model</div>
                <v-text-field v-model.number="modelParams.meanReversion" label="Mean Reversion" type="number" step="0.001" density="compact" class="mb-2" />
                <v-text-field v-model.number="modelParams.volatility" label="Gaussian Vol" type="number" step="0.001" density="compact" class="mb-2" />
                <v-text-field v-model.number="modelParams.numGridPoints" label="Grid Points" type="number" step="2" density="compact" class="mb-2" />

                <v-divider class="my-3" />
                <div class="text-subtitle-2 mb-2">Curves</div>
                <v-text-field v-model.number="modelParams.fundingRate" label="Funding Rate" type="number" step="0.005" density="compact" class="mb-2" />
                <v-text-field v-model.number="modelParams.couponRate" label="Coupon Rate" type="number" step="0.005" density="compact" class="mb-2" />
                <v-text-field v-model.number="modelParams.normalVolBp" label="Normal Vol (bp)" type="number" step="5" density="compact" class="mb-2" />

                <v-divider class="my-3" />
                <div class="text-subtitle-2 mb-2">Schedule</div>
                <v-text-field v-model.number="scheduleParams.numExercises" label="Exercise Dates" type="number" step="1" density="compact" class="mb-2" />
                <v-text-field v-model.number="scheduleParams.swapTenor" label="Swap Tenor" type="number" step="1" density="compact" class="mb-2" />

                <v-divider class="my-3" />
                <div class="text-subtitle-2 mb-2">Exercise</div>
                <v-switch v-model="bermudanParams.isCallable" :label="bermudanParams.isCallable ? 'Callable' : 'Puttable'" color="primary" density="compact" class="mb-2" />
                <v-text-field v-model.number="bermudanParams.flatCoupon" label="Flat Coupon" type="number" step="0.005" density="compact" class="mb-2" />
              </v-card-text>
              <v-card-actions>
                <v-btn color="primary" block :loading="loading.bermudan" @click="runBermudan">
                  Price Bermudan
                </v-btn>
              </v-card-actions>
            </v-card>
          </v-col>

          <v-col cols="12" md="8">
            <v-card v-if="bermudanResult">
              <v-card-title class="d-flex align-center">
                Bermudan Swaption Result
                <v-spacer />
                <v-chip color="success" size="small">
                  {{ formatMs(bermudanResult.computationTimeMs) }}
                </v-chip>
              </v-card-title>
              <v-card-text>
                <v-row class="mb-4">
                  <v-col cols="3">
                    <v-card variant="tonal" color="primary" class="pa-4 text-center">
                      <div class="text-caption">Present Value</div>
                      <div class="text-h5 font-weight-bold">{{ formatNumber(bermudanResult.pv, 6) }}</div>
                    </v-card>
                  </v-col>
                  <v-col cols="3">
                    <v-card variant="outlined" class="pa-4 text-center">
                      <div class="text-caption">Continuation Value</div>
                      <div class="text-h5">{{ formatNumber(bermudanResult.continuationValue, 6) }}</div>
                    </v-card>
                  </v-col>
                  <v-col cols="3">
                    <v-card variant="outlined" class="pa-4 text-center">
                      <div class="text-caption">Option Value</div>
                      <div class="text-h5">{{ formatNumber(bermudanResult.optionValue, 6) }}</div>
                    </v-card>
                  </v-col>
                  <v-col cols="3">
                    <v-card variant="outlined" class="pa-4 text-center">
                      <div class="text-caption">Exercise Boundary Points</div>
                      <div class="text-h5">{{ bermudanResult.exerciseBoundary.length }}</div>
                    </v-card>
                  </v-col>
                </v-row>

                <!-- Exercise Boundary -->
                <div v-if="bermudanResult.exerciseBoundary.length > 0">
                  <div class="text-subtitle-2 mb-2">Exercise Boundary</div>
                  <v-table density="compact">
                    <thead>
                      <tr>
                        <th>Exercise Date</th>
                        <th>Boundary (x)</th>
                      </tr>
                    </thead>
                    <tbody>
                      <tr v-for="(b, i) in bermudanResult.exerciseBoundary" :key="i">
                        <td>{{ i + 1 }}</td>
                        <td>{{ formatNumber(b, 6) }}</td>
                      </tr>
                    </tbody>
                  </v-table>
                </div>
              </v-card-text>
            </v-card>
            <v-card v-else variant="outlined" class="d-flex align-center justify-center" min-height="400">
              <v-card-text class="text-center text-medium-emphasis">
                Configure parameters and click "Price Bermudan" to compute
              </v-card-text>
            </v-card>
          </v-col>
        </v-row>
      </v-window-item>

      <!-- ═══════════════════════════════════════════════════════════════ -->
      <!-- TAB 4: TARN -->
      <!-- ═══════════════════════════════════════════════════════════════ -->
      <v-window-item value="tarn">
        <v-row>
          <v-col cols="12" md="4">
            <v-card>
              <v-card-title>TARN Parameters</v-card-title>
              <v-card-text>
                <div class="text-subtitle-2 mb-2">Model</div>
                <v-text-field v-model.number="modelParams.meanReversion" label="Mean Reversion" type="number" step="0.001" density="compact" class="mb-2" />
                <v-text-field v-model.number="modelParams.volatility" label="Gaussian Vol" type="number" step="0.001" density="compact" class="mb-2" />
                <v-text-field v-model.number="modelParams.numGridPoints" label="Grid Points" type="number" step="2" density="compact" class="mb-2" />

                <v-divider class="my-3" />
                <div class="text-subtitle-2 mb-2">Curves</div>
                <v-text-field v-model.number="modelParams.fundingRate" label="Funding Rate" type="number" step="0.005" density="compact" class="mb-2" />
                <v-text-field v-model.number="modelParams.couponRate" label="Coupon Rate" type="number" step="0.005" density="compact" class="mb-2" />
                <v-text-field v-model.number="modelParams.normalVolBp" label="Normal Vol (bp)" type="number" step="5" density="compact" class="mb-2" />

                <v-divider class="my-3" />
                <div class="text-subtitle-2 mb-2">Schedule</div>
                <v-text-field v-model.number="scheduleParams.numExercises" label="Coupon Dates" type="number" step="1" density="compact" class="mb-2" />
                <v-text-field v-model.number="scheduleParams.swapTenor" label="Swap Tenor" type="number" step="1" density="compact" class="mb-2" />

                <v-divider class="my-3" />
                <div class="text-subtitle-2 mb-2">TARN Config</div>
                <v-text-field v-model.number="tarnParams.tarnAmount" label="Target Amount" type="number" step="0.01" density="compact" class="mb-2" />
                <v-text-field v-model.number="tarnParams.numCouponGridPoints" label="Coupon Grid Points" type="number" step="1" density="compact" class="mb-2" />
                <v-switch v-model="tarnParams.excessCouponFlag" label="Pay Excess Coupon" color="primary" density="compact" class="mb-2" />
                <v-switch v-model="tarnParams.hasBermudanExercise" label="Bermudan Exercise" color="primary" density="compact" class="mb-2" />
                <v-text-field v-model.number="tarnParams.flatCoupon" label="Flat Coupon" type="number" step="0.005" density="compact" class="mb-2" />
              </v-card-text>
              <v-card-actions>
                <v-btn color="primary" block :loading="loading.tarn" @click="runTarn">
                  Price TARN
                </v-btn>
              </v-card-actions>
            </v-card>
          </v-col>

          <v-col cols="12" md="8">
            <v-card v-if="tarnResult">
              <v-card-title class="d-flex align-center">
                TARN Pricing Result
                <v-spacer />
                <v-chip color="success" size="small">
                  {{ formatMs(tarnResult.computationTimeMs) }}
                </v-chip>
              </v-card-title>
              <v-card-text>
                <v-row>
                  <v-col cols="4">
                    <v-card variant="tonal" color="primary" class="pa-4 text-center">
                      <div class="text-caption">Present Value</div>
                      <div class="text-h5 font-weight-bold">{{ formatNumber(tarnResult.pv, 6) }}</div>
                    </v-card>
                  </v-col>
                  <v-col cols="4">
                    <v-card variant="outlined" class="pa-4 text-center">
                      <div class="text-caption">Auto-Redemption Prob</div>
                      <div class="text-h5">{{ formatPct(tarnResult.autoRedemptionProbability) }}</div>
                    </v-card>
                  </v-col>
                  <v-col cols="4">
                    <v-card variant="outlined" class="pa-4 text-center">
                      <div class="text-caption">Expected Redemption Time</div>
                      <div class="text-h5">{{ formatNumber(tarnResult.expectedRedemptionTime, 2) }} yrs</div>
                    </v-card>
                  </v-col>
                </v-row>
              </v-card-text>
            </v-card>
            <v-card v-else variant="outlined" class="d-flex align-center justify-center" min-height="400">
              <v-card-text class="text-center text-medium-emphasis">
                Configure TARN parameters and click "Price TARN" to compute with 2D state space expansion
              </v-card-text>
            </v-card>
          </v-col>
        </v-row>
      </v-window-item>
    </v-window>
  </v-container>
</template>
