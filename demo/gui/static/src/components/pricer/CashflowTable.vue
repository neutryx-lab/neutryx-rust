<script setup lang="ts">
import { computed } from 'vue';
import { usePricerStore } from '@/stores/pricer';
import { useCashflowEditor } from '@/composables/useCashflowEditor';
import { formatNumberCompact, parseFormattedNumber } from '@/utils/format';

const store = usePricerStore();
const { updateCashflow, resetEdits } = useCashflowEditor();

interface CashflowRow {
  legIdx: number;
  cfIdx: number;
  paymentDate: string;
  accrualStart: string;
  accrualEnd: string;
  yearFraction: number;
  notional: number;
  rate: number | null;
  payoffType: string;
  discountFactor: number | null;
  pv: number | null;
  edited: boolean;
}

interface LegGroup {
  legIdx: number;
  direction: string;
  currency: string;
  legType: string;
  rateIndex?: string;
  rows: CashflowRow[];
}

const legGroups = computed<LegGroup[]>(() => {
  if (!store.expandedTrade) return [];
  return store.expandedTrade.legs.map((leg, legIdx) => ({
    legIdx,
    direction: leg.direction,
    currency: leg.currency,
    legType: leg.legType,
    rateIndex: leg.rateIndex,
    rows: leg.cashflows.map((cf, cfIdx) => {
      const key = `${legIdx}-${cfIdx}`;
      const edited = store.editedCashflows[key];
      const pricedCf = (store.pricingResult?.legs as any)?.[legIdx]?.cashflows?.[cfIdx];
      return {
        legIdx,
        cfIdx,
        paymentDate: cf.paymentDate,
        accrualStart: cf.accrualStart,
        accrualEnd: cf.accrualEnd,
        yearFraction: cf.yearFraction,
        notional: edited?.notional ?? cf.notional,
        rate: edited?.rate ?? cf.rate,
        payoffType: cf.payoffType,
        discountFactor: pricedCf?.discountFactor ?? null,
        pv: pricedCf?.pv ?? null,
        edited: !!edited,
      };
    }),
  }));
});

function onNotionalChange(legIdx: number, cfIdx: number, event: Event) {
  const value = parseFormattedNumber((event.target as HTMLInputElement).value);
  updateCashflow(legIdx, cfIdx, 'notional', value);
}

function onRateChange(legIdx: number, cfIdx: number, event: Event) {
  const value = parseFloat((event.target as HTMLInputElement).value) / 100;
  updateCashflow(legIdx, cfIdx, 'rate', value);
}

function fmtNotional(v: number): string {
  return formatNumberCompact(v);
}

function fmtRate(v: number | null): string {
  if (v == null) return '';
  return (v * 100).toFixed(4);
}

function fmtYf(v: number): string {
  return v.toFixed(4);
}

function fmtDf(v: number | null): string {
  if (v == null) return '-';
  return v.toFixed(6);
}

function fmtPv(v: number | null): string {
  if (v == null) return '-';
  return v.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 });
}
</script>

<template>
  <v-card variant="outlined">
    <v-card-text class="pa-0">
      <!-- Loading -->
      <div v-if="store.isExpanding" class="pa-4">
        <v-progress-linear indeterminate color="primary" />
      </div>

      <!-- Empty State -->
      <div v-else-if="!store.expandedTrade" class="text-center pa-6 text-medium-emphasis" style="font-size: 0.8rem">
        Expand to view cashflows
      </div>

      <!-- Spreadsheet -->
      <div v-else class="sheet-wrap">
        <!-- Toolbar -->
        <div v-if="store.hasEdits" class="sheet-toolbar">
          <button class="reset-btn" @click="resetEdits">Reset Edits</button>
        </div>

        <div class="sheet-scroll">
          <table class="sheet">
            <thead>
              <tr>
                <th class="col-date">Payment</th>
                <th class="col-date">Accrual Start</th>
                <th class="col-date">Accrual End</th>
                <th class="col-num">YF</th>
                <th class="col-num col-edit">Notional</th>
                <th class="col-num col-edit">Rate %</th>
                <th class="col-type">Type</th>
                <th class="col-num">DF</th>
                <th class="col-num">PV</th>
              </tr>
            </thead>
            <tbody v-for="group in legGroups" :key="group.legIdx">
              <!-- Leg separator row -->
              <tr class="leg-row">
                <td colspan="9">
                  Leg {{ group.legIdx }}
                  <span class="leg-tag" :class="group.direction === 'Payer' ? 'tag-pay' : 'tag-rec'">{{ group.direction }}</span>
                  <span class="leg-tag">{{ group.currency }}</span>
                  <span class="leg-tag">{{ group.legType }}</span>
                  <span v-if="group.rateIndex" class="leg-tag tag-idx">{{ group.rateIndex }}</span>
                </td>
              </tr>
              <!-- Data rows -->
              <tr v-for="row in group.rows" :key="`${row.legIdx}-${row.cfIdx}`" :class="{ 'row-edited': row.edited }">
                <td class="col-date">{{ row.paymentDate }}</td>
                <td class="col-date">{{ row.accrualStart }}</td>
                <td class="col-date">{{ row.accrualEnd }}</td>
                <td class="col-num">{{ fmtYf(row.yearFraction) }}</td>
                <td class="col-num col-edit">
                  <input
                    type="text"
                    :value="fmtNotional(row.notional)"
                    class="cell-input"
                    :class="{ edited: row.edited }"
                    @change="onNotionalChange(row.legIdx, row.cfIdx, $event)"
                  />
                </td>
                <td class="col-num col-edit">
                  <template v-if="row.rate != null">
                    <input
                      type="text"
                      :value="fmtRate(row.rate)"
                      class="cell-input"
                      :class="{ edited: row.edited }"
                      @change="onRateChange(row.legIdx, row.cfIdx, $event)"
                    />
                  </template>
                  <span v-else class="float-label">Float</span>
                </td>
                <td class="col-type">{{ row.payoffType }}</td>
                <td class="col-num">{{ fmtDf(row.discountFactor) }}</td>
                <td class="col-num" :class="row.pv != null ? (row.pv >= 0 ? 'pv-pos' : 'pv-neg') : ''">
                  {{ fmtPv(row.pv) }}
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <!-- Footer -->
        <div class="sheet-footer">
          {{ store.expandedTrade.metadata.totalLegs }} legs
          · {{ store.expandedTrade.metadata.totalCashflows }} cfs
          · {{ store.expandedTrade.metadata.processingTimeMs.toFixed(1) }}ms
        </div>
      </div>
    </v-card-text>
  </v-card>
</template>

<style scoped>
.sheet-wrap {
  font-family: 'JetBrains Mono', 'Consolas', monospace;
  font-size: 0.82rem;
  line-height: 1.4;
}

.sheet-toolbar {
  padding: 4px 8px;
  border-bottom: 1px solid rgba(var(--v-theme-on-surface), 0.08);
}

.reset-btn {
  font-size: 0.8rem;
  color: rgb(var(--v-theme-warning));
  cursor: pointer;
  background: none;
  border: none;
  text-decoration: underline;
}

.sheet-scroll {
  overflow-x: auto;
}

.sheet {
  border-collapse: collapse;
  white-space: nowrap;
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

.col-num.col-edit {
  padding: 2px 4px;
  width: 100px;
  max-width: 100px;
}

.col-type {
  text-align: center;
  font-size: 0.75rem;
  color: rgba(var(--v-theme-on-surface), 0.5);
}

.col-date {
  font-variant-numeric: tabular-nums;
}

/* Leg separator */
.leg-row td {
  padding: 6px 8px 4px;
  font-weight: 700;
  font-size: 0.8rem;
  color: rgba(var(--v-theme-on-surface), 0.6);
  border-bottom: 1px solid rgba(var(--v-theme-on-surface), 0.10);
  background: rgba(var(--v-theme-on-surface), 0.03);
}

.leg-tag {
  display: inline-block;
  margin-left: 6px;
  padding: 0 4px;
  font-weight: 500;
  font-size: 0.72rem;
  border-radius: 2px;
  background: rgba(var(--v-theme-on-surface), 0.06);
  color: rgba(var(--v-theme-on-surface), 0.5);
}

.tag-pay { color: rgb(var(--v-theme-error)); background: rgba(var(--v-theme-error), 0.08); }
.tag-rec { color: rgb(var(--v-theme-success)); background: rgba(var(--v-theme-success), 0.08); }
.tag-idx { color: rgb(var(--v-theme-info)); background: rgba(var(--v-theme-info), 0.08); }

/* Editable cell input */
.cell-input {
  width: 90px;
  padding: 1px 4px;
  text-align: right;
  font-family: inherit;
  font-size: inherit;
  color: inherit;
  border: 1px solid transparent;
  border-radius: 2px;
  background: transparent;
  outline: none;
}

.cell-input:hover {
  border-color: rgba(var(--v-theme-on-surface), 0.12);
}

.cell-input:focus {
  border-color: rgb(var(--v-theme-primary));
  background: rgba(var(--v-theme-primary), 0.04);
}

.cell-input.edited {
  border-color: rgba(var(--v-theme-warning), 0.5);
  background: rgba(var(--v-theme-warning), 0.06);
}

.float-label {
  font-style: italic;
  color: rgba(var(--v-theme-on-surface), 0.35);
  font-size: 0.75rem;
}

.row-edited td {
  background: rgba(var(--v-theme-warning), 0.03);
}

.pv-pos { color: rgb(var(--v-theme-success)); font-weight: 600; }
.pv-neg { color: rgb(var(--v-theme-error)); font-weight: 600; }

.sheet-footer {
  padding: 4px 8px;
  font-size: 0.75rem;
  color: rgba(var(--v-theme-on-surface), 0.4);
  border-top: 1px solid rgba(var(--v-theme-on-surface), 0.06);
}
</style>
