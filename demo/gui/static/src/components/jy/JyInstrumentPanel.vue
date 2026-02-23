<script setup lang="ts">
import { useJyInflationStore } from '@/stores/jyInflation';

const store = useJyInflationStore();

function formatNum(v: number): string {
  const abs = Math.abs(v);
  const sign = v < 0 ? '-' : '';
  if (abs >= 1e6) return `${sign}${(abs / 1e6).toFixed(2)}M`;
  if (abs >= 1e3) return `${sign}${(abs / 1e3).toFixed(1)}K`;
  return `${sign}${abs.toFixed(2)}`;
}
</script>

<template>
  <div class="glass-card">
    <div>
      <!-- Loading -->
      <div v-if="store.loading" class="pa-4">
        <v-progress-linear indeterminate color="primary" />
      </div>

      <!-- Empty State -->
      <div v-else-if="!store.instrumentResult" class="text-center pa-6 text-medium-emphasis" style="font-size: 0.8rem">
        Expand to view cashflows
      </div>

      <!-- Cashflow Table -->
      <div v-else class="sheet-wrap">
        <div class="sheet-scroll">
          <table class="sheet">
            <thead>
              <tr>
                <th class="col-num">#</th>
                <th class="col-date">Date</th>
                <th class="col-num">Year Frac</th>
                <th class="col-num">Fixed</th>
                <th class="col-num">Inflation</th>
                <th class="col-num">DF</th>
                <th class="col-num">PV</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(cf, i) in store.instrumentResult.cashflows" :key="i">
                <td class="col-num" style="color: rgba(var(--v-theme-on-surface), 0.4)">{{ i + 1 }}</td>
                <td class="col-date">{{ cf.date }}</td>
                <td class="col-num">{{ cf.yearFraction.toFixed(4) }}</td>
                <td class="col-num">{{ formatNum(cf.nominalAmount) }}</td>
                <td class="col-num">{{ cf.realAmount != null ? formatNum(cf.realAmount) : '-' }}</td>
                <td class="col-num">{{ cf.discountFactor.toFixed(6) }}</td>
                <td class="col-num" :class="cf.presentValue >= 0 ? 'pv-pos' : 'pv-neg'">
                  {{ formatNum(cf.presentValue) }}
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <!-- Summary -->
        <div class="sheet-summary">
          <span>Fixed Leg PV: <strong class="pv-neg">{{ formatNum(store.instrumentResult.summary.totalFixedPv) }}</strong></span>
          <span>Inflation Leg PV: <strong class="pv-pos">{{ formatNum(store.instrumentResult.summary.totalInflationPv) }}</strong></span>
          <span>Net PV: <strong :class="store.instrumentResult.summary.netPv >= 0 ? 'pv-pos' : 'pv-neg'">{{ formatNum(store.instrumentResult.summary.netPv) }}</strong></span>
        </div>

        <!-- Footer -->
        <div class="sheet-footer">
          {{ store.instrumentResult.cashflows.length }} cashflows
          · {{ store.instrumentType }}
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.sheet-wrap {
  font-family: 'JetBrains Mono', 'Consolas', monospace;
  font-size: 0.82rem;
  line-height: 1.4;
}

.sheet-scroll {
  overflow-x: auto;
}

.sheet {
  border-collapse: collapse;
  white-space: nowrap;
  width: 100%;
}

.sheet thead {
  position: sticky;
  top: 0;
  z-index: 1;
}

.sheet th {
  padding: 6px 8px;
  font-weight: 600;
  font-size: 0.75rem;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: rgba(var(--v-theme-on-surface), 0.5);
  border-bottom: 2px solid rgba(var(--v-theme-on-surface), 0.12);
  background: rgb(var(--v-theme-surface));
  text-align: left;
}

.sheet td {
  padding: 4px 8px;
  border-bottom: 1px solid rgba(var(--v-theme-on-surface), 0.04);
  color: rgba(var(--v-theme-on-surface), 0.8);
}

.sheet tr:hover td {
  background: rgba(var(--v-theme-primary), 0.04);
}

.col-num {
  text-align: right;
}

.col-date {
  font-variant-numeric: tabular-nums;
}

.pv-pos { color: rgb(var(--v-theme-success)); font-weight: 600; }
.pv-neg { color: rgb(var(--v-theme-error)); font-weight: 600; }

.sheet-summary {
  display: flex;
  gap: 16px;
  padding: 8px;
  font-size: 0.8rem;
  color: rgba(var(--v-theme-on-surface), 0.6);
  border-top: 1px solid rgba(var(--v-theme-on-surface), 0.08);
}

.sheet-footer {
  padding: 4px 8px;
  font-size: 0.75rem;
  color: rgba(var(--v-theme-on-surface), 0.4);
  border-top: 1px solid rgba(var(--v-theme-on-surface), 0.06);
}
</style>
