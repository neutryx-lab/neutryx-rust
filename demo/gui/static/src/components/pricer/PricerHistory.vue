<script setup lang="ts">
import { usePricerStore } from '@/stores/pricer';
import { usePricerHistory } from '@/composables/usePricerHistory';
import { formatCurrency } from '@/utils/format';

const store = usePricerStore();
const { restoreFromHistory, toggleCompareMode } = usePricerHistory();

function entryPv(entry: (typeof store.recentHistory)[number]): number {
  return entry.pricingResult.totalPv ?? entry.pricingResult.pv ?? 0;
}
</script>

<template>
  <div v-if="store.recentHistory.length > 0" class="glass-card p-6">
    <div class="flex items-center justify-between mb-4">
      <h3 class="text-lg font-semibold text-[var(--text-primary)]">History</h3>
      <button
        v-if="store.resultHistory.length >= 2"
        class="px-3 py-1.5 text-xs rounded-lg transition-colors"
        :class="
          store.compareMode
            ? 'bg-[var(--primary)] text-white'
            : 'bg-[var(--surface)] text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]'
        "
        @click="toggleCompareMode"
      >
        <i class="fas fa-columns mr-1"></i>{{ store.compareMode ? 'Exit Compare' : 'Compare' }}
      </button>
    </div>

    <!-- History List -->
    <div class="space-y-2">
      <div
        v-for="entry in store.recentHistory"
        :key="entry.id"
        class="p-3 rounded-lg bg-[var(--surface)] cursor-pointer hover:bg-[var(--surface-hover)] transition-colors"
        @click="restoreFromHistory(entry)"
      >
        <div class="flex justify-between items-center">
          <div>
            <span class="text-sm font-medium text-[var(--text-primary)]">
              {{ entry.instrumentName }}
            </span>
            <span class="text-xs text-[var(--text-muted)] ml-2">
              {{ new Date(entry.timestamp).toLocaleTimeString() }}
            </span>
          </div>
          <span
            :class="[
              'text-sm font-semibold',
              entryPv(entry) >= 0 ? 'text-[var(--success)]' : 'text-[var(--danger)]',
            ]"
          >
            {{ formatCurrency(entryPv(entry)) }}
          </span>
        </div>
      </div>
    </div>

    <!-- Compare Mode View -->
    <div
      v-if="store.compareMode && store.comparedResults"
      class="mt-4 border-t border-[var(--glass-border)] pt-4"
    >
      <div class="grid grid-cols-2 gap-4 mb-3">
        <select
          v-model.number="store.compareIndices[0]"
          class="px-3 py-2 text-sm rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)]"
        >
          <option v-for="(e, i) in store.recentHistory" :key="i" :value="i">
            #{{ e.id }} {{ e.instrumentName }}
          </option>
        </select>
        <select
          v-model.number="store.compareIndices[1]"
          class="px-3 py-2 text-sm rounded-lg bg-[var(--surface)] border border-[var(--glass-border)] text-[var(--text-primary)]"
        >
          <option v-for="(e, i) in store.recentHistory" :key="i" :value="i">
            #{{ e.id }} {{ e.instrumentName }}
          </option>
        </select>
      </div>

      <!-- Side-by-side PV -->
      <div class="grid grid-cols-2 gap-4 text-center">
        <div class="p-3 rounded-lg bg-[var(--surface)]">
          <p class="text-xs text-[var(--text-muted)] mb-1">Result A</p>
          <p
            :class="[
              'text-lg font-bold',
              (store.comparedResults.a.pricingResult.totalPv ?? 0) >= 0
                ? 'text-[var(--success)]'
                : 'text-[var(--danger)]',
            ]"
          >
            {{
              formatCurrency(
                store.comparedResults.a.pricingResult.totalPv ??
                  store.comparedResults.a.pricingResult.pv ??
                  0,
              )
            }}
          </p>
        </div>
        <div class="p-3 rounded-lg bg-[var(--surface)]">
          <p class="text-xs text-[var(--text-muted)] mb-1">Result B</p>
          <p
            :class="[
              'text-lg font-bold',
              (store.comparedResults.b.pricingResult.totalPv ?? 0) >= 0
                ? 'text-[var(--success)]'
                : 'text-[var(--danger)]',
            ]"
          >
            {{
              formatCurrency(
                store.comparedResults.b.pricingResult.totalPv ??
                  store.comparedResults.b.pricingResult.pv ??
                  0,
              )
            }}
          </p>
        </div>
      </div>

      <!-- Changed Parameters -->
      <div v-if="store.changedParams.length > 0" class="mt-3">
        <p class="text-xs text-[var(--text-muted)] mb-2">Changed Parameters</p>
        <div
          v-for="cp in store.changedParams"
          :key="cp.name"
          class="flex justify-between text-xs py-1 border-b border-[var(--glass-border)]"
        >
          <span class="text-[var(--text-muted)]">{{ cp.name }}</span>
          <span>
            <span class="text-[var(--danger)]">{{ cp.valueA }}</span>
            <i class="fas fa-arrow-right mx-1 text-[var(--text-muted)]"></i>
            <span class="text-[var(--success)]">{{ cp.valueB }}</span>
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
