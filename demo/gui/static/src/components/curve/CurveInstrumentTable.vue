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
    <div class="flex items-center justify-between mb-4">
      <h3 class="section-title">Instruments</h3>
      <div v-if="instruments.length > 0" class="flex gap-1">
        <button
          class="toggle-btn"
          @click="emit('toggleAll', true)"
        >All</button>
        <button
          class="toggle-btn"
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

    <div v-else class="instrument-list">
      <div
        v-for="(inst, idx) in instruments"
        :key="inst.id"
        :class="['instrument-row', { disabled: !inst.enabled }, inst.type]"
      >
        <!-- Row header: checkbox + ID + type badge -->
        <div class="row-header">
          <input
            type="checkbox"
            :checked="inst.enabled"
            class="checkbox"
            @change="emit('toggle', idx)"
          >
          <span class="instrument-id" :title="inst.id">{{ inst.id }}</span>
          <span v-if="inst.type === 'event'" class="type-badge event-badge">
            {{ inst.endDate ? 'TURN' : 'JUMP' }}
          </span>
          <span v-else-if="inst.type === 'bond'" class="type-badge bond-badge">BOND</span>
          <span v-else-if="inst.type === 'cds'" class="type-badge cds-badge">CDS</span>
        </div>

        <!-- Fields grid -->
        <div class="row-fields">
          <!-- Event instruments -->
          <template v-if="inst.type === 'event'">
            <div class="field-label">Date</div>
            <div class="field-input">
              <span
                class="date-value"
                :title="inst.endDate ? `Turn: ${inst.eventDate} → ${inst.endDate}` : 'Event Date'"
              >{{ inst.eventDate }}</span>
            </div>
            <div class="field-label">Spike</div>
            <div class="field-input">
              <input
                type="number"
                :value="(inst.rate * 10000).toFixed(1)"
                step="0.5"
                class="input-field event-field"
                :title="inst.endDate ? `Turn spike in bp (reverts ${inst.endDate})` : 'Expected rate spike in basis points'"
                @change="emit('updateSpike', idx, ($event.target as HTMLInputElement).value)"
              >
              <span class="field-unit">bp</span>
            </div>
          </template>

          <!-- Bond instruments -->
          <template v-else-if="inst.type === 'bond'">
            <div class="field-label">Coupon</div>
            <div class="field-input">
              <input
                type="number"
                :value="((inst.couponRate || 0) * 100).toFixed(2)"
                step="0.01"
                class="input-field bond-field"
                title="Coupon rate (%)"
                @change="emit('updateCoupon', idx, ($event.target as HTMLInputElement).value)"
              >
              <span class="field-unit">%</span>
            </div>
            <div class="field-label">YTM</div>
            <div class="field-input">
              <input
                type="number"
                :value="(inst.rate * 100).toFixed(2)"
                step="0.01"
                class="input-field"
                title="Yield-to-maturity (%)"
                @change="emit('updateRate', idx, ($event.target as HTMLInputElement).value)"
              >
              <span class="field-unit">%</span>
            </div>
          </template>

          <!-- CDS instruments -->
          <template v-else-if="inst.type === 'cds'">
            <div class="field-label">Spread</div>
            <div class="field-input">
              <input
                type="number"
                :value="(inst.rate * 10000).toFixed(1)"
                step="0.5"
                class="input-field cds-field"
                title="CDS spread in basis points"
                @change="emit('updateSpike', idx, ($event.target as HTMLInputElement).value)"
              >
              <span class="field-unit">bp</span>
            </div>
          </template>

          <!-- Regular instruments -->
          <template v-else>
            <div class="field-label">Rate</div>
            <div class="field-input">
              <input
                type="number"
                :value="(inst.rate * 100).toFixed(2)"
                step="0.01"
                class="input-field"
                @change="emit('updateRate', idx, ($event.target as HTMLInputElement).value)"
              >
              <span class="field-unit">%</span>
            </div>
          </template>
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

.section-title {
  font-size: 0.95rem;
  font-weight: 600;
  color: var(--text-primary);
}

.toggle-btn {
  padding: 3px 10px;
  font-size: 0.7rem;
  font-weight: 500;
  border-radius: 4px;
  background: var(--surface);
  color: var(--text-muted);
  border: 1px solid var(--glass-border);
  cursor: pointer;
  transition: background 0.15s;
}
.toggle-btn:hover {
  background: var(--surface-hover);
}

/* ─── Instrument list ─── */
.instrument-list {
  max-height: 26rem;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

/* ─── Instrument row ─── */
.instrument-row {
  border-radius: 6px;
  background: var(--surface);
  padding: 8px 10px;
  transition: opacity 0.15s;
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

/* ─── Row header ─── */
.row-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 6px;
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
  flex: 1;
  font-family: monospace;
  font-size: 0.8rem;
  font-weight: 500;
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
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

/* ─── Fields grid (Pricer-style property grid) ─── */
.row-fields {
  display: grid;
  grid-template-columns: 50px 1fr;
  align-items: center;
  gap: 3px 8px;
  padding-left: 22px;
}

.field-label {
  font-size: 0.75rem;
  color: var(--text-muted);
  text-align: right;
  padding-right: 2px;
  white-space: nowrap;
  line-height: 1.2;
}

.field-input {
  display: flex;
  align-items: center;
  gap: 4px;
  min-width: 0;
}

.input-field {
  width: 100%;
  max-width: 100px;
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
