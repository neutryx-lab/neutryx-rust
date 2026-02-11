<script setup lang="ts">
import { computed } from 'vue';
import { usePricerStore } from '@/stores/pricer';
import { useCashflowEditor } from '@/composables/useCashflowEditor';
import { formatNumberCompact, parseFormattedNumber, formatCurrency } from '@/utils/format';

const store = usePricerStore();
const { updateCashflow, resetEdits } = useCashflowEditor();

const headers = [
  { title: 'Payment', key: 'paymentDate', sortable: true },
  { title: 'Accrual Start', key: 'accrualStart', sortable: true },
  { title: 'Accrual End', key: 'accrualEnd', sortable: true },
  { title: 'YF', key: 'yearFraction', align: 'end' as const },
  { title: 'Notional', key: 'notional', align: 'end' as const },
  { title: 'Rate', key: 'rate', align: 'end' as const },
  { title: 'Type', key: 'payoffType', align: 'center' as const },
  { title: 'DF', key: 'discountFactor', align: 'end' as const },
  { title: 'PV', key: 'pv', align: 'end' as const },
];

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

function buildRows(legIdx: number): CashflowRow[] {
  const leg = store.expandedTrade?.legs[legIdx];
  if (!leg) return [];
  return leg.cashflows.map((cf, cfIdx) => {
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
  });
}

const legTables = computed(() => {
  if (!store.expandedTrade) return [];
  return store.expandedTrade.legs.map((leg, idx) => ({
    leg,
    legIdx: idx,
    rows: buildRows(idx),
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

function getNotionalDisplay(row: CashflowRow): string {
  return formatNumberCompact(row.notional);
}

function getRateDisplay(row: CashflowRow): string {
  const rate = row.rate ?? 0;
  return (rate * 100).toFixed(4);
}
</script>

<template>
  <v-card>
    <v-card-title class="d-flex align-center justify-space-between">
      <span>Cashflows</span>
      <v-btn
        v-if="store.hasEdits"
        size="small"
        variant="text"
        color="warning"
        prepend-icon="mdi-undo"
        @click="resetEdits"
      >
        Reset Edits
      </v-btn>
    </v-card-title>

    <v-card-text>
      <!-- Loading -->
      <div v-if="store.isExpanding">
        <v-skeleton-loader v-for="i in 4" :key="i" type="table-row" class="mb-2" />
      </div>

      <!-- Empty State -->
      <div v-else-if="!store.expandedTrade" class="text-center py-8">
        <v-icon icon="mdi-table-large" size="48" color="grey" class="mb-3" />
        <p class="text-medium-emphasis">Click "Expand Cashflows" to view cashflows</p>
      </div>

      <!-- Expanded Trade -->
      <template v-else>
        <!-- Trade ID Chips -->
        <div class="mb-4 d-flex ga-2">
          <v-chip color="primary" size="small" prepend-icon="mdi-pound">
            {{ store.expandedTrade.tradeId }}
          </v-chip>
          <v-chip size="small" variant="tonal">
            {{ store.expandedTrade.tradeType }}
          </v-chip>
        </div>

        <!-- Per-Leg Data Tables -->
        <div v-for="lt in legTables" :key="lt.legIdx" class="mb-6">
          <!-- Leg Header -->
          <div class="d-flex align-center ga-2 mb-2">
            <v-avatar color="primary" size="24">
              <span class="text-caption">{{ lt.legIdx + 1 }}</span>
            </v-avatar>
            <v-chip
              size="small"
              :color="lt.leg.direction === 'Payer' ? 'error' : 'success'"
              variant="tonal"
            >
              {{ lt.leg.direction }}
            </v-chip>
            <v-chip size="small" variant="outlined">{{ lt.leg.currency }}</v-chip>
            <v-chip size="small" variant="outlined">{{ lt.leg.legType }}</v-chip>
            <v-chip v-if="lt.leg.rateIndex" size="small" color="info" variant="tonal">
              {{ lt.leg.rateIndex }}
            </v-chip>
          </div>

          <!-- Data Table -->
          <v-data-table
            :headers="headers"
            :items="lt.rows"
            :items-per-page="-1"
            density="compact"
            hover
            class="elevation-0"
          >
            <template #item.yearFraction="{ item }">
              <span class="font-weight-medium text-body-2">{{ item.yearFraction.toFixed(4) }}</span>
            </template>

            <template #item.notional="{ item }">
              <input
                type="text"
                :value="getNotionalDisplay(item)"
                class="cf-input"
                :class="{ 'cf-edited': item.edited }"
                @change="onNotionalChange(item.legIdx, item.cfIdx, $event)"
              />
            </template>

            <template #item.rate="{ item }">
              <template v-if="item.rate !== null">
                <input
                  type="text"
                  :value="getRateDisplay(item)"
                  class="cf-input"
                  :class="{ 'cf-edited': item.edited }"
                  @change="onRateChange(item.legIdx, item.cfIdx, $event)"
                />
                <span class="text-caption text-medium-emphasis ml-1">%</span>
              </template>
              <span v-else class="text-medium-emphasis font-italic">Floating</span>
            </template>

            <template #item.payoffType="{ item }">
              <v-chip
                size="x-small"
                :color="item.payoffType === 'Fixed' ? 'info' : 'secondary'"
                variant="tonal"
              >
                {{ item.payoffType }}
              </v-chip>
            </template>

            <template #item.discountFactor="{ item }">
              <span v-if="item.discountFactor !== null" class="font-weight-medium text-body-2">
                {{ item.discountFactor.toFixed(6) }}
              </span>
              <span v-else class="text-medium-emphasis">-</span>
            </template>

            <template #item.pv="{ item }">
              <span
                v-if="item.pv !== null"
                :class="item.pv >= 0 ? 'text-success' : 'text-error'"
                class="font-weight-bold"
              >
                {{ formatCurrency(item.pv) }}
              </span>
              <span v-else class="text-medium-emphasis">-</span>
            </template>

            <template #bottom />
          </v-data-table>
        </div>

        <!-- Footer Metadata -->
        <v-divider class="mb-3" />
        <div class="d-flex ga-4 text-caption text-medium-emphasis">
          <span>
            <v-icon icon="mdi-layers-outline" size="14" class="mr-1" />
            {{ store.expandedTrade.metadata.totalLegs }} legs
          </span>
          <span>
            <v-icon icon="mdi-cash-multiple" size="14" class="mr-1" />
            {{ store.expandedTrade.metadata.totalCashflows }} cashflows
          </span>
          <span>
            <v-icon icon="mdi-timer-outline" size="14" class="mr-1" />
            {{ store.expandedTrade.metadata.processingTimeMs.toFixed(2) }}ms
          </span>
        </div>
      </template>
    </v-card-text>
  </v-card>
</template>

<style scoped>
.cf-input {
  width: 80px;
  padding: 2px 6px;
  text-align: right;
  font-size: 0.8125rem;
  font-family: 'JetBrains Mono', monospace;
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 4px;
  background: rgba(255, 255, 255, 0.05);
  color: inherit;
  outline: none;
}
.cf-input:focus {
  border-color: rgb(var(--v-theme-primary));
}
.cf-edited {
  border-color: rgb(var(--v-theme-warning));
  background: rgba(var(--v-theme-warning), 0.08);
}
</style>
