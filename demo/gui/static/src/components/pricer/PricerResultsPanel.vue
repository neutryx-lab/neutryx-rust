<script setup lang="ts">
/**
 * PricerResultsPanel — Faithfully displays PricingResult from result.rs.
 *
 * Sections:
 *   1. Total PV + method + computation time
 *   2. Greeks (inline from PricingResult.greeks)
 *   3. Leg breakdown (per-leg PV, currency, FX rate)
 *   4. Cashflow-level PV (per-leg expandable)
 *   5. Path Distribution (MC only)
 */
import { computed, ref } from 'vue';
import { usePricerStore } from '@/stores/pricer';
import { formatCurrency } from '@/utils/format';

const store = usePricerStore();
const expandedLegs = ref<Set<number>>(new Set());

function toggleLeg(idx: number) {
  if (expandedLegs.value.has(idx)) {
    expandedLegs.value.delete(idx);
  } else {
    expandedLegs.value.add(idx);
  }
}

const result = computed(() => store.pricingResult);
const greeks = computed(() => result.value?.greeks);
const pathDist = computed(() => result.value?.pathDistribution);

function fmtNum(v: number | null | undefined, decimals = 2): string {
  if (v == null) return '-';
  return v.toLocaleString(undefined, { minimumFractionDigits: decimals, maximumFractionDigits: decimals });
}

function fmtCcy(v: number | null | undefined): string {
  if (v == null) return '-';
  return formatCurrency(v);
}

const greekItems = computed(() => {
  if (!greeks.value) return [];
  const g = greeks.value;
  const items: { label: string; value: number | null | undefined }[] = [
    { label: 'Delta', value: g.delta },
    { label: 'Gamma', value: g.gamma },
    { label: 'Vega', value: g.vega },
    { label: 'Theta', value: g.theta },
    { label: 'Rho', value: g.rho },
  ];
  return items.filter((i) => i.value != null);
});
</script>

<template>
  <div v-if="result" class="glass-card">
    <div class="p-3">
      <div class="result-grid">
        <!-- ═══ PricingResult ═══ -->
        <div class="section-header">PricingResult</div>

        <div class="grid-label">total_pv</div>
        <div class="grid-value" :class="result.totalPv >= 0 ? 'text-success' : 'text-error'">
          {{ fmtCcy(result.totalPv) }}
        </div>

        <div class="grid-label">reporting_currency</div>
        <div class="grid-value">{{ result.reportingCurrency }}</div>

        <div v-if="result.method" class="grid-label">method</div>
        <div v-if="result.method" class="grid-value">{{ result.method }}</div>

        <div v-if="result.computationTimeMs != null" class="grid-label">computation_time</div>
        <div v-if="result.computationTimeMs != null" class="grid-value">{{ fmtNum(result.computationTimeMs, 1) }} ms</div>

        <!-- ═══ Greeks (inline) ═══ -->
        <template v-if="greekItems.length > 0">
          <div class="section-header">Greeks</div>

          <template v-for="g in greekItems" :key="g.label">
            <div class="grid-label">{{ g.label }}</div>
            <div class="grid-value">{{ fmtCcy(g.value) }}</div>
          </template>
        </template>

        <!-- ═══ Legs ═══ -->
        <div class="section-header">Legs ({{ result.legs.length }})</div>

        <template v-for="(leg, idx) in result.legs" :key="idx">
          <div class="grid-label leg-header" @click="toggleLeg(idx)">
            <v-icon size="12" class="mr-1">{{ expandedLegs.has(idx) ? 'mdi-chevron-down' : 'mdi-chevron-right' }}</v-icon>
            Leg {{ idx }}
          </div>
          <div class="grid-value leg-header" :class="leg.pv >= 0 ? 'text-success' : 'text-error'" @click="toggleLeg(idx)">
            {{ fmtCcy(leg.pv) }}
          </div>

          <template v-if="expandedLegs.has(idx)">
            <div class="grid-label sub">direction</div>
            <div class="grid-value sub">{{ leg.direction }}</div>

            <div class="grid-label sub">currency</div>
            <div class="grid-value sub">{{ leg.currency }}</div>

            <div class="grid-label sub">pv</div>
            <div class="grid-value sub" :class="leg.pv >= 0 ? 'text-success' : 'text-error'">{{ fmtCcy(leg.pv) }}</div>

            <template v-if="leg.pvOriginal != null">
              <div class="grid-label sub">pv_original</div>
              <div class="grid-value sub">{{ fmtCcy(leg.pvOriginal) }}</div>
            </template>

            <template v-if="leg.fxRate != null">
              <div class="grid-label sub">fx_rate</div>
              <div class="grid-value sub">{{ fmtNum(leg.fxRate, 6) }}</div>
            </template>

            <!-- Cashflows -->
            <template v-if="leg.cashflows && leg.cashflows.length > 0">
              <div class="grid-label sub cf-header">cashflows ({{ leg.cashflows.length }})</div>
              <div class="grid-value sub cf-header"></div>

              <template v-for="(cf, cfIdx) in leg.cashflows" :key="cfIdx">
                <div class="grid-label sub2">{{ cf.paymentDate }}</div>
                <div class="grid-value sub2">
                  PV {{ fmtNum(cf.pv, 2) }} | DF {{ fmtNum(cf.discountFactor, 6) }}
                </div>
              </template>
            </template>
          </template>
        </template>

        <!-- ═══ PathDistribution (MC) ═══ -->
        <template v-if="pathDist">
          <div class="section-header">PathDistribution</div>

          <div class="grid-label">mean</div>
          <div class="grid-value">{{ fmtNum(pathDist.mean) }}</div>

          <div class="grid-label">std_dev</div>
          <div class="grid-value">{{ fmtNum(pathDist.stdDev) }}</div>

          <div class="grid-label">path_count</div>
          <div class="grid-value">{{ pathDist.pathCount.toLocaleString() }}</div>

          <template v-for="[pct, val] in pathDist.percentiles" :key="pct">
            <div class="grid-label sub">p{{ pct }}</div>
            <div class="grid-value sub">{{ fmtNum(val) }}</div>
          </template>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.result-grid {
  display: grid;
  grid-template-columns: 100px 1fr;
  align-items: baseline;
  gap: 2px 8px;
}

.result-grid .grid-label {
  font-size: 0.75rem;
  color: rgba(var(--v-theme-on-surface), 0.6);
  font-family: monospace;
}

.grid-value {
  font-size: 0.8rem;
  font-family: monospace;
}

.leg-header {
  cursor: pointer;
  font-weight: 600;
}

.sub {
  padding-left: 12px;
  font-size: 0.72rem;
}

.sub2 {
  padding-left: 24px;
  font-size: 0.7rem;
  color: rgba(var(--v-theme-on-surface), 0.55);
}

.cf-header {
  font-weight: 500;
  font-style: italic;
}
</style>
