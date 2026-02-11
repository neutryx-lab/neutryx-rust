<script setup lang="ts">
import { computed } from 'vue';
import type { JacobianData } from '@/composables/useCurveBuilder';

const props = defineProps<{
  jacobian: JacobianData;
}>();

const jacobianAbsMax = computed(() => {
  const vals = props.jacobian.matrix.flat().filter(v => v !== 0);
  if (vals.length === 0) return 1;
  return Math.max(...vals.map(Math.abs));
});

function jacobianHeatmapColour(value: number): string {
  const max = jacobianAbsMax.value;
  if (max === 0 || value === 0) return 'transparent';
  const t = Math.min(Math.abs(value) / max, 1);
  if (value < 0) {
    return `rgba(239, 68, 68, ${0.08 + t * 0.35})`;
  }
  return `rgba(59, 130, 246, ${0.08 + t * 0.35})`;
}

function jacobianTextColour(value: number): string {
  const max = jacobianAbsMax.value;
  if (max === 0 || value === 0) return 'var(--text-muted)';
  const t = Math.min(Math.abs(value) / max, 1);
  if (t > 0.4) return value < 0 ? '#f87171' : '#60a5fa';
  return 'var(--text-secondary)';
}
</script>

<template>
  <div class="glass-card p-6">
    <div class="flex items-center justify-between mb-4">
      <h3 class="text-lg font-semibold text-[var(--text-primary)]">
        <i class="fas fa-th text-sm mr-2 text-[var(--primary)]"></i>
        Jacobian <span class="text-sm font-normal text-[var(--text-muted)]">d(log DF)/T / dr &approx; &minus;dz/dr</span>
      </h3>
      <span class="text-xs text-[var(--text-muted)] font-mono">
        {{ jacobian.size }} &times; {{ jacobian.size }}
      </span>
    </div>

    <div class="overflow-x-auto">
      <table class="w-full border-collapse">
        <thead>
          <tr>
            <th class="sticky left-0 z-10 py-2 px-3 text-xs font-medium text-[var(--text-muted)] jacobian-sticky-cell border-b border-r border-[var(--glass-border)] text-left">
              &minus;dz \ Rate
            </th>
            <th
              v-for="label in jacobian.col_labels"
              :key="'jh-' + label"
              class="py-2 px-2 text-xs font-medium text-[var(--text-muted)] text-center border-b border-[var(--glass-border)] whitespace-nowrap"
            >
              {{ label }}
            </th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="(row, i) in jacobian.matrix"
            :key="'jr-' + i"
            class="hover:bg-[var(--surface-hover)] transition-colors"
          >
            <td class="sticky left-0 z-10 py-1.5 px-3 text-xs font-medium text-[var(--text-muted)] jacobian-sticky-cell border-r border-b border-[var(--glass-border)] whitespace-nowrap">
              {{ jacobian.row_labels[i] }}
            </td>
            <td
              v-for="(val, j) in row"
              :key="'jc-' + i + '-' + j"
              class="py-1.5 px-1 text-center text-xs font-mono border-b border-[var(--glass-border)]"
              :style="{ backgroundColor: jacobianHeatmapColour(val) }"
            >
              <span :style="{ color: jacobianTextColour(val) }">
                {{ val === 0 ? '--' : val.toPrecision(2) }}
              </span>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <p class="mt-3 text-xs text-[var(--text-muted)]">
      <i class="fas fa-info-circle mr-1"></i>
      Normalised by T<sub>i</sub>: diagonal &approx; &minus;1 (zero rate moves 1:1 with market rate). Lower-triangular in bootstrapping.
    </p>
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

.jacobian-sticky-cell {
  background: var(--glass-bg);
}
</style>
