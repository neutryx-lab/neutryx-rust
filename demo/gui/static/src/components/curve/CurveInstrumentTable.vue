<script setup lang="ts">
import type { DisplayInstrument } from '@/composables/useCurveBuilder';

defineProps<{
  instruments: DisplayInstrument[];
  isLoading: boolean;
}>();

const emit = defineEmits<{
  toggle: [index: number];
  toggleAll: [enabled: boolean];
  updateRate: [index: number, value: string];
  updateSpike: [index: number, value: string];
  updateCoupon: [index: number, value: string];
}>();
</script>

<template>
  <div class="glass-card p-5">
    <div class="flex items-center justify-between mb-3">
      <h3 class="text-base font-semibold text-[var(--text-primary)]">Instruments</h3>
      <div v-if="instruments.length > 0" class="flex gap-1">
        <button
          class="px-2 py-1 text-xs rounded bg-[var(--surface)] text-[var(--text-muted)] hover:bg-[var(--surface-hover)]"
          @click="emit('toggleAll', true)"
        >All</button>
        <button
          class="px-2 py-1 text-xs rounded bg-[var(--surface)] text-[var(--text-muted)] hover:bg-[var(--surface-hover)]"
          @click="emit('toggleAll', false)"
        >None</button>
      </div>
    </div>

    <div v-if="isLoading" class="text-center py-8">
      <i class="fas fa-spinner fa-spin text-[var(--primary)]"></i>
    </div>

    <div v-else-if="instruments.length === 0" class="text-center py-8 text-[var(--text-muted)] text-sm">
      Select a curve
    </div>

    <div v-else class="max-h-64 overflow-y-auto space-y-1">
      <div
        v-for="(inst, idx) in instruments"
        :key="inst.id"
        :class="[
          'flex items-center gap-2 px-2 py-1.5 rounded text-sm',
          inst.enabled ? 'bg-[var(--surface)]' : 'opacity-40',
          inst.type === 'event' ? 'border-l-2 border-amber-500' : '',
          inst.type === 'bond' ? 'border-l-2 border-blue-500' : '',
          inst.type === 'cds' ? 'border-l-2 border-purple-500' : ''
        ]"
      >
        <input
          type="checkbox"
          :checked="inst.enabled"
          class="w-3.5 h-3.5 rounded border-[var(--glass-border)]"
          @change="emit('toggle', idx)"
        >
        <span class="flex-1 font-mono text-xs text-[var(--text-secondary)] truncate" :title="inst.id">{{ inst.id }}</span>
        <!-- Event instruments show date and expected spike input -->
        <template v-if="inst.type === 'event'">
          <span
            v-if="inst.endDate"
            class="px-1 py-0.5 text-[10px] rounded bg-cyan-500/20 text-cyan-400"
            title="Turn event (temporary spike)"
          >TURN</span>
          <span
            v-else
            class="px-1 py-0.5 text-[10px] rounded bg-amber-500/20 text-amber-400"
            title="Jump event (permanent shift)"
          >JUMP</span>
          <span
            class="px-1.5 py-0.5 text-xs rounded bg-amber-500/20 text-amber-400 font-mono"
            :title="inst.endDate ? `Turn: ${inst.eventDate} → ${inst.endDate}` : 'Event Date'"
          >{{ inst.eventDate }}</span>
          <input
            type="number"
            :value="(inst.rate * 10000).toFixed(1)"
            step="0.5"
            class="w-14 px-1.5 py-0.5 text-right text-xs rounded bg-amber-500/10 border border-amber-500/30 text-amber-400"
            :title="inst.endDate ? `Turn spike in bp (reverts ${inst.endDate})` : 'Expected rate spike in basis points'"
            @change="emit('updateSpike', idx, ($event.target as HTMLInputElement).value)"
          >
          <span class="text-xs text-amber-400/60">bp</span>
        </template>
        <!-- Bond instruments show coupon rate + YTM inputs -->
        <template v-else-if="inst.type === 'bond'">
          <span
            class="px-1 py-0.5 text-[10px] rounded bg-blue-500/20 text-blue-400"
            title="Fixed-coupon bond"
          >BOND</span>
          <span class="text-[10px] text-[var(--text-muted)]">Cpn</span>
          <input
            type="number"
            :value="((inst.couponRate || 0) * 100).toFixed(2)"
            step="0.01"
            class="w-14 px-1.5 py-0.5 text-right text-xs rounded bg-blue-500/10 border border-blue-500/30 text-blue-400"
            title="Coupon rate (%)"
            @change="emit('updateCoupon', idx, ($event.target as HTMLInputElement).value)"
          >
          <span class="text-[10px] text-[var(--text-muted)]">YTM</span>
          <input
            type="number"
            :value="(inst.rate * 100).toFixed(2)"
            step="0.01"
            class="w-14 px-1.5 py-0.5 text-right text-xs rounded bg-[var(--glass-bg)] border border-[var(--glass-border)] text-[var(--text-primary)]"
            title="Yield-to-maturity (%)"
            @change="emit('updateRate', idx, ($event.target as HTMLInputElement).value)"
          >
        </template>
        <!-- CDS instruments show spread in bp -->
        <template v-else-if="inst.type === 'cds'">
          <span
            class="px-1 py-0.5 text-[10px] rounded bg-purple-500/20 text-purple-400"
            title="Credit Default Swap"
          >CDS</span>
          <input
            type="number"
            :value="(inst.rate * 10000).toFixed(1)"
            step="0.5"
            class="w-16 px-1.5 py-0.5 text-right text-xs rounded bg-purple-500/10 border border-purple-500/30 text-purple-400"
            title="CDS spread in basis points"
            @change="emit('updateSpike', idx, ($event.target as HTMLInputElement).value)"
          >
          <span class="text-xs text-purple-400/60">bp</span>
        </template>
        <!-- Regular instruments show rate input -->
        <input
          v-else
          type="number"
          :value="(inst.rate * 100).toFixed(2)"
          step="0.01"
          class="w-16 px-1.5 py-0.5 text-right text-xs rounded bg-[var(--glass-bg)] border border-[var(--glass-border)] text-[var(--text-primary)]"
          @change="emit('updateRate', idx, ($event.target as HTMLInputElement).value)"
        >
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
