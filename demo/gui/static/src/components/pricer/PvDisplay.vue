<script setup lang="ts">
import { usePricerStore } from '@/stores/pricer';
import { formatCurrency } from '@/utils/format';

const store = usePricerStore();

function pvValue(): number {
  return store.pricingResult?.totalPv ?? store.pricingResult?.pv ?? 0;
}
</script>

<template>
  <div v-if="store.pricingResult" class="glass-card p-6">
    <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-4">Present Value</h3>

    <!-- Total PV -->
    <div
      :class="[
        'text-3xl font-bold text-center py-4',
        pvValue() >= 0 ? 'text-[var(--success)]' : 'text-[var(--danger)]',
      ]"
    >
      {{ formatCurrency(pvValue()) }}
    </div>

    <!-- PV Diff from Previous -->
    <div v-if="store.pvDiff" class="mt-2 text-center text-sm">
      <span
        :class="store.pvDiff.absolute >= 0 ? 'text-[var(--success)]' : 'text-[var(--danger)]'"
      >
        {{ store.pvDiff.absolute >= 0 ? '+' : '' }}{{ formatCurrency(store.pvDiff.absolute) }}
        ({{ store.pvDiff.percent >= 0 ? '+' : '' }}{{ store.pvDiff.percent.toFixed(2) }}%)
      </span>
      <span class="text-[var(--text-muted)]"> vs previous</span>
    </div>

    <!-- Leg-level PV -->
    <div v-if="store.pricingResult.legs" class="mt-4 space-y-2">
      <div
        v-for="(leg, idx) in store.pricingResult.legs"
        :key="idx"
        class="flex justify-between text-sm"
      >
        <span class="text-[var(--text-muted)]">
          Leg {{ idx + 1 }} ({{ leg.direction }})
          <span
            v-if="(leg as Record<string, unknown>).currency"
            class="ml-1 px-1.5 py-0.5 rounded bg-[var(--surface)] text-xs"
          >
            {{ (leg as Record<string, unknown>).currency }}
          </span>
        </span>
        <span :class="leg.pv >= 0 ? 'text-[var(--success)]' : 'text-[var(--danger)]'">
          {{ formatCurrency(leg.pv) }}
        </span>
      </div>

      <!-- Currency Aggregation -->
      <div
        v-if="store.currencyAggregation.length > 1"
        class="pt-2 border-t border-[var(--glass-border)]"
      >
        <p class="text-xs text-[var(--text-muted)] mb-1">By Currency</p>
        <div
          v-for="agg in store.currencyAggregation"
          :key="agg.ccy"
          class="flex justify-between text-sm"
        >
          <span class="text-[var(--text-muted)]">{{ agg.ccy }}</span>
          <span :class="agg.pv >= 0 ? 'text-[var(--success)]' : 'text-[var(--danger)]'">
            {{ formatCurrency(agg.pv) }}
          </span>
        </div>
      </div>
    </div>
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
