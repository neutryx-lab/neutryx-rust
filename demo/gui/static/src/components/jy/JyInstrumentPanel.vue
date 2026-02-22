<script setup lang="ts">
import { useJyInflationStore } from '@/stores/jyInflation';

const store = useJyInflationStore();
const emit = defineEmits<{ generate: [] }>();
</script>

<template>
  <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
    <!-- Instrument Config -->
    <div class="space-y-4">
      <h4 class="text-sm font-semibold text-[var(--text-primary)] flex items-center gap-2">
        <i class="fas fa-cog text-[var(--primary)]"></i>
        Instrument Configuration
      </h4>

      <div class="space-y-3">
        <div>
          <label class="text-xs text-[var(--text-muted)] mb-1 block">Type</label>
          <select v-model="store.instrumentType" class="input-field">
            <option value="ZCIS">Zero-Coupon Inflation Swap (ZCIS)</option>
            <option value="YoYIS">Year-on-Year Inflation Swap (YoYIS)</option>
          </select>
        </div>
        <div>
          <label class="text-xs text-[var(--text-muted)] mb-1 block">Notional</label>
          <input v-model.number="store.notional" type="number" step="100000" min="1" class="input-field" />
        </div>
        <div>
          <label class="text-xs text-[var(--text-muted)] mb-1 block">Fixed Rate (%)</label>
          <input v-model.number="store.fixedRate" type="number" step="0.001" class="input-field" />
        </div>
        <div>
          <label class="text-xs text-[var(--text-muted)] mb-1 block">Start Date</label>
          <input v-model="store.startDate" type="date" class="input-field" />
        </div>
        <div>
          <label class="text-xs text-[var(--text-muted)] mb-1 block">Maturity (years)</label>
          <input v-model.number="store.maturityYears" type="number" step="1" min="1" max="50" class="input-field" />
        </div>
        <div>
          <label class="text-xs text-[var(--text-muted)] mb-1 block">Maturity Date</label>
          <input :value="store.maturityDate" type="date" disabled class="input-field opacity-60" />
        </div>
        <div v-if="store.instrumentType === 'YoYIS'">
          <label class="text-xs text-[var(--text-muted)] mb-1 block">Payment Frequency</label>
          <select v-model="store.paymentFrequency" class="input-field">
            <option value="annual">Annual</option>
            <option value="semiannual">Semi-Annual</option>
            <option value="quarterly">Quarterly</option>
          </select>
        </div>
      </div>

      <button
        class="w-full mt-4 px-4 py-2 rounded-lg text-sm font-medium bg-[var(--primary)] text-white hover:opacity-90 transition-all"
        :disabled="store.loading"
        @click="emit('generate')"
      >
        <i :class="['fas mr-2', store.loading ? 'fa-spinner fa-spin' : 'fa-cogs']"></i>
        Generate Cashflows
      </button>
    </div>

    <!-- Cashflow Table -->
    <div class="lg:col-span-2 space-y-4">
      <h4 class="text-sm font-semibold text-[var(--text-primary)] flex items-center gap-2">
        <i class="fas fa-list text-green-500"></i>
        Cashflow Schedule
        <span v-if="store.instrumentResult" class="text-xs text-[var(--text-muted)]">
          ({{ store.instrumentResult.cashflows.length }} flows)
        </span>
      </h4>

      <div v-if="store.instrumentResult" class="overflow-auto max-h-80">
        <table class="w-full text-xs">
          <thead>
            <tr class="text-[var(--text-muted)] border-b border-[var(--border)]">
              <th class="text-left py-2 px-2">#</th>
              <th class="text-left py-2 px-2">Date</th>
              <th class="text-right py-2 px-2">Year Frac</th>
              <th class="text-right py-2 px-2">Fixed</th>
              <th class="text-right py-2 px-2">Inflation</th>
              <th class="text-right py-2 px-2">DF</th>
              <th class="text-right py-2 px-2">PV</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(cf, i) in store.instrumentResult.cashflows" :key="i"
              class="border-b border-[var(--border)] border-opacity-50">
              <td class="py-1.5 px-2 text-[var(--text-muted)]">{{ i + 1 }}</td>
              <td class="py-1.5 px-2">{{ cf.date }}</td>
              <td class="py-1.5 px-2 text-right">{{ cf.yearFraction.toFixed(4) }}</td>
              <td class="py-1.5 px-2 text-right">{{ formatNum(cf.nominalAmount) }}</td>
              <td class="py-1.5 px-2 text-right">{{ cf.realAmount != null ? formatNum(cf.realAmount) : '-' }}</td>
              <td class="py-1.5 px-2 text-right">{{ cf.discountFactor.toFixed(6) }}</td>
              <td class="py-1.5 px-2 text-right font-medium" :class="cf.presentValue >= 0 ? 'text-green-400' : 'text-red-400'">
                {{ formatNum(cf.presentValue) }}
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <!-- Summary -->
      <div v-if="store.instrumentResult" class="grid grid-cols-3 gap-3 mt-4">
        <div class="p-3 rounded-lg bg-[var(--surface-hover)]">
          <div class="text-xs text-[var(--text-muted)]">Fixed Leg PV</div>
          <div class="text-sm font-semibold text-red-400">{{ formatNum(store.instrumentResult.summary.totalFixedPv) }}</div>
        </div>
        <div class="p-3 rounded-lg bg-[var(--surface-hover)]">
          <div class="text-xs text-[var(--text-muted)]">Inflation Leg PV</div>
          <div class="text-sm font-semibold text-green-400">{{ formatNum(store.instrumentResult.summary.totalInflationPv) }}</div>
        </div>
        <div class="p-3 rounded-lg bg-[var(--surface-hover)]">
          <div class="text-xs text-[var(--text-muted)]">Net PV</div>
          <div class="text-sm font-semibold" :class="store.instrumentResult.summary.netPv >= 0 ? 'text-green-400' : 'text-red-400'">
            {{ formatNum(store.instrumentResult.summary.netPv) }}
          </div>
        </div>
      </div>

      <div v-else class="flex items-center justify-center h-40 text-[var(--text-muted)] text-sm">
        <i class="fas fa-info-circle mr-2"></i>Configure instrument and click "Generate Cashflows"
      </div>
    </div>
  </div>
</template>

<script lang="ts">
function formatNum(v: number): string {
  const abs = Math.abs(v);
  const sign = v < 0 ? '-' : '';
  if (abs >= 1e6) return `${sign}${(abs / 1e6).toFixed(2)}M`;
  if (abs >= 1e3) return `${sign}${(abs / 1e3).toFixed(1)}K`;
  return `${sign}${abs.toFixed(2)}`;
}
</script>

<style scoped>
.input-field {
  width: 100%;
  padding: 0.4rem 0.6rem;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--surface);
  color: var(--text-primary);
  font-size: 0.8rem;
}
.input-field:focus {
  outline: none;
  border-color: var(--primary);
}
</style>
