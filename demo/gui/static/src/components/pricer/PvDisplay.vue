<script setup lang="ts">
import { usePricerStore } from '@/stores/pricer';
import { formatCurrency } from '@/utils/format';

const store = usePricerStore();

function pvValue(): number {
  return store.pricingResult?.totalPv ?? 0;
}
</script>

<template>
  <v-card v-if="store.pricingResult">
    <v-card-title>Present Value</v-card-title>
    <v-card-text>
      <!-- Total PV -->
      <div
        class="text-h4 font-weight-bold text-center py-3"
        :class="pvValue() >= 0 ? 'text-success' : 'text-error'"
      >
        {{ formatCurrency(pvValue()) }}
      </div>

      <!-- PV Diff -->
      <div v-if="store.pvDiff" class="text-center text-body-2 mb-3">
        <span :class="store.pvDiff.absolute >= 0 ? 'text-success' : 'text-error'">
          {{ store.pvDiff.absolute >= 0 ? '+' : '' }}{{ formatCurrency(store.pvDiff.absolute) }}
          ({{ store.pvDiff.percent >= 0 ? '+' : '' }}{{ store.pvDiff.percent.toFixed(2) }}%)
        </span>
        <span class="text-medium-emphasis"> vs previous</span>
      </div>

      <!-- Leg PV Breakdown -->
      <v-table v-if="store.pricingResult.legs" density="compact">
        <tbody>
          <tr v-for="(leg, idx) in store.pricingResult.legs" :key="idx">
            <td class="text-medium-emphasis">
              Leg {{ idx + 1 }} ({{ leg.direction }})
              <v-chip
                v-if="(leg as Record<string, unknown>).currency"
                size="x-small"
                variant="tonal"
                class="ml-1"
              >
                {{ (leg as Record<string, unknown>).currency }}
              </v-chip>
            </td>
            <td class="text-right" :class="leg.pv >= 0 ? 'text-success' : 'text-error'">
              {{ formatCurrency(leg.pv) }}
            </td>
          </tr>
        </tbody>
      </v-table>

      <!-- Currency Aggregation -->
      <template v-if="store.currencyAggregation.length > 1">
        <v-divider class="my-2" />
        <div class="text-caption text-medium-emphasis mb-1">By Currency</div>
        <v-table density="compact">
          <tbody>
            <tr v-for="agg in store.currencyAggregation" :key="agg.ccy">
              <td class="text-medium-emphasis">{{ agg.ccy }}</td>
              <td class="text-right" :class="agg.pv >= 0 ? 'text-success' : 'text-error'">
                {{ formatCurrency(agg.pv) }}
              </td>
            </tr>
          </tbody>
        </v-table>
      </template>
    </v-card-text>
  </v-card>
</template>
