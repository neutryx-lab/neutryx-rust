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
  updatePips: [index: number, value: string];
  updateCoupon: [index: number, value: string];
}>();
</script>

<template>
  <div class="glass-card px-3 py-4">
    <div class="section-header" style="margin-top: 0; display: flex; justify-content: space-between; align-items: center;">
      <span>Instruments</span>
      <span v-if="instruments.length > 0" class="toggle-group">
        <button class="toggle-btn" @click="emit('toggleAll', true)">All</button>
        <button class="toggle-btn" @click="emit('toggleAll', false)">None</button>
      </span>
    </div>

    <div v-if="isLoading" class="text-center py-8">
      <i class="fas fa-spinner fa-spin text-[var(--primary)]"></i>
    </div>

    <div v-else-if="instruments.length === 0" class="text-center py-8 text-[var(--text-muted)] text-sm">
      Select a curve
    </div>

    <div v-else class="instrument-list">
      <div
        v-for="(inst, idx) in instruments"
        :key="inst.id"
        :class="['instrument-row', { disabled: !inst.enabled }, inst.type]"
        style="display: flex; align-items: center; gap: 4px; flex-wrap: nowrap;"
      >
        <input
          type="checkbox"
          :checked="inst.enabled"
          class="checkbox"
          @change="emit('toggle', idx)"
        >
        <span class="instrument-id" style="flex: 1; min-width: 0;" :title="inst.id">{{ inst.id }}</span>
        <span v-if="inst.type === 'event'" class="type-badge event-badge">
          {{ inst.endDate ? 'TURN' : 'JUMP' }}
        </span>
        <span v-else-if="inst.type === 'bond'" class="type-badge bond-badge">BOND</span>
        <span v-else-if="inst.type === 'cds'" class="type-badge cds-badge">CDS</span>
        <span v-else-if="inst.type === 'fx_forward'" class="type-badge fwd-badge">FWD</span>
        <span v-else-if="inst.type === 'xccy_basis'" class="type-badge basis-badge">BASIS</span>

        <!-- Event fields -->
        <template v-if="inst.type === 'event'">
          <span class="field-label">Date</span>
          <span
            class="date-value"
            :title="inst.endDate ? `Turn: ${inst.eventDate} → ${inst.endDate}` : 'Event Date'"
          >{{ inst.eventDate }}</span>
          <span class="field-label">Spike</span>
          <input
            type="number"
            :value="(inst.rate * 10000).toFixed(1)"
            step="0.5"
            class="input-field event-field"
            :title="inst.endDate ? `Turn spike in bp (reverts ${inst.endDate})` : 'Expected rate spike in basis points'"
            @change="emit('updateSpike', idx, ($event.target as HTMLInputElement).value)"
          >
          <span class="field-unit">bp</span>
        </template>

        <!-- Bond fields -->
        <template v-else-if="inst.type === 'bond'">
          <span class="field-label">Coupon</span>
          <input
            type="number"
            :value="((inst.couponRate || 0) * 100).toFixed(2)"
            step="0.01"
            class="input-field bond-field"
            title="Coupon rate (%)"
            @change="emit('updateCoupon', idx, ($event.target as HTMLInputElement).value)"
          >
          <span class="field-unit">%</span>
          <span class="field-label">YTM</span>
          <input
            type="number"
            :value="(inst.rate * 100).toFixed(2)"
            step="0.01"
            class="input-field"
            title="Yield-to-maturity (%)"
            @change="emit('updateRate', idx, ($event.target as HTMLInputElement).value)"
          >
          <span class="field-unit">%</span>
        </template>

        <!-- CDS fields -->
        <template v-else-if="inst.type === 'cds'">
          <span class="field-label">Spread</span>
          <input
            type="number"
            :value="(inst.rate * 10000).toFixed(1)"
            step="0.5"
            class="input-field cds-field"
            title="CDS spread in basis points"
            @change="emit('updateSpike', idx, ($event.target as HTMLInputElement).value)"
          >
          <span class="field-unit">bp</span>
        </template>

        <!-- FX Forward fields -->
        <template v-else-if="inst.type === 'fx_forward'">
          <span class="field-label">Pips</span>
          <input
            type="number"
            :value="inst.rate.toFixed(2)"
            step="0.01"
            class="input-field fwd-field"
            title="Forward points (pips)"
            @change="emit('updatePips', idx, ($event.target as HTMLInputElement).value)"
          >
          <span class="field-unit">pips</span>
        </template>

        <!-- XCCY Basis fields -->
        <template v-else-if="inst.type === 'xccy_basis'">
          <span class="field-label">Spread</span>
          <input
            type="number"
            :value="inst.rate.toFixed(1)"
            step="0.5"
            class="input-field basis-field"
            title="Cross-currency basis spread (bps)"
            @change="emit('updatePips', idx, ($event.target as HTMLInputElement).value)"
          >
          <span class="field-unit">bps</span>
        </template>

        <!-- Regular fields -->
        <template v-else>
          <span class="field-label">Rate</span>
          <input
            type="number"
            :value="(inst.rate * 100).toFixed(2)"
            step="0.01"
            class="input-field"
            @change="emit('updateRate', idx, ($event.target as HTMLInputElement).value)"
          >
          <span class="field-unit">%</span>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* ─── Header toggle buttons ─── */
.toggle-group {
  display: inline-flex;
  gap: 4px;
}

.toggle-btn {
  padding: 2px 8px;
  font-size: 0.65rem;
  font-weight: 500;
  border-radius: 3px;
  background: var(--surface);
  color: var(--text-muted);
  border: 1px solid var(--glass-border);
  cursor: pointer;
  text-transform: none;
  letter-spacing: normal;
  transition: background 0.15s;
}
.toggle-btn:hover {
  background: var(--surface-hover);
}

/* ─── Instrument list ─── */
.instrument-list {
  max-height: 26rem;
  overflow-y: auto;
  overflow-x: hidden;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

/* ─── Instrument row ─── */
.instrument-row {
  display: flex !important;
  align-items: center;
  gap: 4px;
  border-radius: 6px;
  background: var(--surface);
  padding: 5px 6px;
  transition: opacity 0.15s;
  flex-wrap: nowrap;
}
.instrument-row.disabled {
  opacity: 0.35;
}
.instrument-row.event {
  border-left: 2px solid #f59e0b;
}
.instrument-row.bond {
  border-left: 2px solid #3b82f6;
}
.instrument-row.cds {
  border-left: 2px solid #a855f7;
}
.instrument-row.fx_forward {
  border-left: 2px solid #06b6d4;
}
.instrument-row.xccy_basis {
  border-left: 2px solid #f59e0b;
}

.checkbox {
  width: 14px;
  height: 14px;
  border-radius: 3px;
  border: 1px solid var(--glass-border);
  cursor: pointer;
  flex-shrink: 0;
}

.instrument-id {
  font-family: monospace;
  font-size: 0.8rem;
  font-weight: 500;
  color: var(--text-secondary);
  white-space: nowrap;
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
}

.field-label {
  font-size: 0.7rem;
  color: var(--text-muted);
  white-space: nowrap;
  flex-shrink: 0;
}

.type-badge {
  padding: 1px 6px;
  font-size: 0.6rem;
  font-weight: 600;
  letter-spacing: 0.05em;
  border-radius: 3px;
  flex-shrink: 0;
}
.event-badge {
  background: rgba(245, 158, 11, 0.15);
  color: #f59e0b;
}
.bond-badge {
  background: rgba(59, 130, 246, 0.15);
  color: #3b82f6;
}
.cds-badge {
  background: rgba(168, 85, 247, 0.15);
  color: #a855f7;
}
.fwd-badge {
  background: rgba(6, 182, 212, 0.15);
  color: #06b6d4;
}
.basis-badge {
  background: rgba(245, 158, 11, 0.15);
  color: #f59e0b;
}

/* ─── Input fields (component-specific) ─── */
.input-field {
  width: 64px;
  flex-shrink: 1;
  padding: 4px 8px;
  font-size: 0.8rem;
  text-align: right;
  border-radius: 4px;
  background: var(--glass-bg);
  border: 1px solid var(--glass-border);
  color: var(--text-primary);
  outline: none;
  transition: border-color 0.15s;
}
.input-field:focus {
  border-color: var(--primary);
}

.event-field {
  background: rgba(245, 158, 11, 0.08);
  border-color: rgba(245, 158, 11, 0.25);
  color: #f59e0b;
}
.event-field:focus {
  border-color: #f59e0b;
}

.bond-field {
  background: rgba(59, 130, 246, 0.08);
  border-color: rgba(59, 130, 246, 0.25);
  color: #3b82f6;
}
.bond-field:focus {
  border-color: #3b82f6;
}

.cds-field {
  background: rgba(168, 85, 247, 0.08);
  border-color: rgba(168, 85, 247, 0.25);
  color: #a855f7;
}
.cds-field:focus {
  border-color: #a855f7;
}

.fwd-field {
  background: rgba(6, 182, 212, 0.08);
  border-color: rgba(6, 182, 212, 0.25);
  color: #06b6d4;
}
.fwd-field:focus {
  border-color: #06b6d4;
}
.basis-field {
  background: rgba(245, 158, 11, 0.08);
  border-color: rgba(245, 158, 11, 0.25);
  color: #f59e0b;
}
.basis-field:focus {
  border-color: #f59e0b;
}

.field-unit {
  font-size: 0.7rem;
  color: var(--text-muted);
  flex-shrink: 0;
  min-width: 16px;
}

.date-value {
  padding: 3px 8px;
  font-size: 0.75rem;
  font-family: monospace;
  border-radius: 4px;
  background: rgba(245, 158, 11, 0.12);
  color: #f59e0b;
}

/* Scrollbar styling */
.instrument-list::-webkit-scrollbar {
  width: 4px;
}
.instrument-list::-webkit-scrollbar-track {
  background: transparent;
}
.instrument-list::-webkit-scrollbar-thumb {
  background: var(--glass-border);
  border-radius: 2px;
}
</style>
