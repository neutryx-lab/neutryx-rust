<script setup lang="ts">
import { usePricerStore } from '@/stores/pricer';
import { useCashflowEditor } from '@/composables/useCashflowEditor';
import { formatNumberCompact, parseFormattedNumber, formatCurrency } from '@/utils/format';

const store = usePricerStore();
const { updateCashflow, resetEdits } = useCashflowEditor();

function onNotionalChange(legIdx: number, cfIdx: number, event: Event) {
  const value = parseFormattedNumber((event.target as HTMLInputElement).value);
  updateCashflow(legIdx, cfIdx, 'notional', value);
}

function onRateChange(legIdx: number, cfIdx: number, event: Event) {
  const value = parseFloat((event.target as HTMLInputElement).value) / 100;
  updateCashflow(legIdx, cfIdx, 'rate', value);
}

function getNotionalDisplay(legIdx: number, cfIdx: number, originalNotional: number): string {
  const key = `${legIdx}-${cfIdx}`;
  const edited = store.editedCashflows[key];
  return formatNumberCompact(edited?.notional ?? originalNotional);
}

function getRateDisplay(legIdx: number, cfIdx: number, originalRate: number | null): string {
  const key = `${legIdx}-${cfIdx}`;
  const edited = store.editedCashflows[key];
  const rate = edited?.rate ?? originalRate ?? 0;
  return (rate * 100).toFixed(4);
}

function isEdited(legIdx: number, cfIdx: number): boolean {
  return !!store.editedCashflows[`${legIdx}-${cfIdx}`];
}
</script>

<template>
  <div class="glass-card p-6">
    <div class="flex items-center justify-between mb-4">
      <h3 class="text-lg font-semibold text-[var(--text-primary)]">Cashflows</h3>
      <div v-if="store.hasEdits" class="flex items-center gap-2">
        <span class="text-sm text-[var(--warning)] flex items-center gap-1">
          <i class="fas fa-edit"></i> Modified
        </span>
        <button
          class="px-3 py-1.5 text-xs rounded-lg bg-[var(--surface)] text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]"
          @click="resetEdits"
        >
          Reset Edits
        </button>
      </div>
    </div>

    <!-- Loading Skeleton -->
    <div v-if="store.isExpanding" class="space-y-3">
      <div v-for="i in 6" :key="i" class="h-10 rounded-lg bg-[var(--surface)] animate-pulse" />
    </div>

    <!-- Empty State -->
    <div v-else-if="!store.expandedTrade" class="text-center py-12">
      <i class="fas fa-stream text-4xl text-[var(--text-muted)] mb-4"></i>
      <p class="text-[var(--text-muted)]">Click "Expand Cashflows" to view cashflows</p>
    </div>

    <!-- Expanded Trade -->
    <template v-else>
      <!-- Trade ID Badge -->
      <div class="mb-4 flex items-center gap-3">
        <span
          class="px-3 py-1 rounded-lg bg-[var(--primary)]/10 text-[var(--primary)] text-sm font-medium"
        >
          <i class="fas fa-hashtag mr-1"></i>{{ store.expandedTrade.tradeId }}
        </span>
        <span class="px-3 py-1 rounded-lg bg-[var(--surface)] text-[var(--text-secondary)] text-sm">
          {{ store.expandedTrade.tradeType }}
        </span>
      </div>

      <!-- Per-Leg Tables -->
      <div v-for="(leg, legIdx) in store.expandedTrade.legs" :key="legIdx" class="mb-6">
        <!-- Leg Header Badges -->
        <div class="flex items-center gap-2 mb-3">
          <span
            class="w-6 h-6 rounded-full bg-[var(--primary)] text-white text-xs flex items-center justify-center font-medium"
          >
            {{ legIdx + 1 }}
          </span>
          <span
            :class="[
              'px-2 py-0.5 rounded text-xs font-medium',
              leg.direction === 'Payer'
                ? 'bg-red-500/10 text-red-400'
                : 'bg-green-500/10 text-green-400',
            ]"
          >
            {{ leg.direction }}
          </span>
          <span class="px-2 py-0.5 rounded bg-[var(--surface)] text-[var(--text-secondary)] text-xs">
            {{ leg.currency }}
          </span>
          <span class="px-2 py-0.5 rounded bg-[var(--surface)] text-[var(--text-secondary)] text-xs">
            {{ leg.legType }}
          </span>
          <span
            v-if="leg.rateIndex"
            class="px-2 py-0.5 rounded bg-blue-500/10 text-blue-400 text-xs"
          >
            {{ leg.rateIndex }}
          </span>
        </div>

        <!-- Cashflow Table -->
        <div class="overflow-x-auto">
          <table class="w-full text-sm">
            <thead>
              <tr class="border-b border-[var(--glass-border)]">
                <th class="text-left py-2 px-3 text-xs font-medium text-[var(--text-muted)]">Payment</th>
                <th class="text-left py-2 px-3 text-xs font-medium text-[var(--text-muted)]">Accrual</th>
                <th class="text-right py-2 px-3 text-xs font-medium text-[var(--text-muted)]">YF</th>
                <th class="text-right py-2 px-3 text-xs font-medium text-[var(--text-muted)]">Notional</th>
                <th class="text-right py-2 px-3 text-xs font-medium text-[var(--text-muted)]">Rate</th>
                <th class="text-center py-2 px-3 text-xs font-medium text-[var(--text-muted)]">Type</th>
                <th class="text-right py-2 px-3 text-xs font-medium text-[var(--text-muted)]">DF</th>
                <th class="text-right py-2 px-3 text-xs font-medium text-[var(--text-muted)]">PV</th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="(cf, cfIdx) in leg.cashflows"
                :key="cfIdx"
                :class="[
                  'border-b border-[var(--glass-border)]',
                  isEdited(legIdx, cfIdx)
                    ? 'bg-[var(--warning)]/5'
                    : 'hover:bg-[var(--surface-hover)]',
                ]"
              >
                <td class="py-2 px-3 text-[var(--text-primary)]">{{ cf.paymentDate }}</td>
                <td class="py-2 px-3 text-[var(--text-secondary)]">
                  {{ cf.accrualStart }}
                  <span class="text-[var(--text-muted)]">-</span>
                  {{ cf.accrualEnd }}
                </td>
                <td class="py-2 px-3 text-right text-[var(--text-secondary)] font-mono">
                  {{ cf.yearFraction.toFixed(4) }}
                </td>

                <!-- Editable Notional -->
                <td class="py-2 px-3 text-right">
                  <input
                    type="text"
                    :value="getNotionalDisplay(legIdx, cfIdx, cf.notional)"
                    class="w-20 px-2 py-1 text-right text-sm rounded bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] font-mono"
                    @change="onNotionalChange(legIdx, cfIdx, $event)"
                  />
                </td>

                <!-- Editable Rate -->
                <td class="py-2 px-3 text-right">
                  <template v-if="cf.rate !== null">
                    <input
                      type="text"
                      :value="getRateDisplay(legIdx, cfIdx, cf.rate)"
                      class="w-20 px-2 py-1 text-right text-sm rounded bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)] font-mono"
                      @change="onRateChange(legIdx, cfIdx, $event)"
                    />
                    <span class="text-[var(--text-muted)] ml-1">%</span>
                  </template>
                  <span v-else class="text-[var(--text-muted)] italic">Floating</span>
                </td>

                <!-- Payoff Type -->
                <td class="py-2 px-3 text-center">
                  <span
                    :class="[
                      'px-2 py-0.5 rounded text-xs',
                      cf.payoffType === 'Fixed'
                        ? 'bg-blue-500/10 text-blue-400'
                        : 'bg-purple-500/10 text-purple-400',
                    ]"
                  >
                    {{ cf.payoffType }}
                  </span>
                </td>

                <!-- DF (after pricing) -->
                <td class="py-2 px-3 text-right font-mono text-[var(--text-secondary)]">
                  <template
                    v-if="(store.pricingResult?.legs as any)?.[legIdx]?.cashflows?.[cfIdx]"
                  >
                    {{
                      (store.pricingResult!.legs as any)[legIdx].cashflows[cfIdx].discountFactor.toFixed(6)
                    }}
                  </template>
                  <span v-else class="text-[var(--text-muted)]">-</span>
                </td>

                <!-- PV (after pricing) -->
                <td class="py-2 px-3 text-right font-mono">
                  <template
                    v-if="(store.pricingResult?.legs as any)?.[legIdx]?.cashflows?.[cfIdx]"
                  >
                    <span
                      :class="
                        (store.pricingResult!.legs as any)[legIdx].cashflows[cfIdx].pv >= 0
                          ? 'text-[var(--success)]'
                          : 'text-[var(--danger)]'
                      "
                    >
                      {{
                        formatCurrency(
                          (store.pricingResult!.legs as any)[legIdx].cashflows[cfIdx].pv,
                        )
                      }}
                    </span>
                  </template>
                  <span v-else class="text-[var(--text-muted)]">-</span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <!-- Footer Metadata -->
      <div
        class="flex items-center gap-4 text-sm text-[var(--text-muted)] pt-4 border-t border-[var(--glass-border)]"
      >
        <span>
          <i class="fas fa-layer-group mr-1"></i>{{ store.expandedTrade.metadata.totalLegs }} legs
        </span>
        <span>
          <i class="fas fa-coins mr-1"></i>{{ store.expandedTrade.metadata.totalCashflows }}
          cashflows
        </span>
        <span>
          <i class="fas fa-clock mr-1"></i
          >{{ store.expandedTrade.metadata.processingTimeMs.toFixed(2) }}ms
        </span>
      </div>
    </template>
  </div>
</template>

<style scoped>
.glass-card {
  background: var(--glass-bg);
  backdrop-filter: blur(20px);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-lg);
  box-shadow: var(--glass-shadow);
}
</style>
