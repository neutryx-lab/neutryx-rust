<script setup lang="ts">
import { usePricerStore } from '@/stores/pricer';
import { usePricerHistory } from '@/composables/usePricerHistory';
import { formatCurrency } from '@/utils/format';

const store = usePricerStore();
const { restoreFromHistory, toggleCompareMode } = usePricerHistory();

function entryPv(entry: (typeof store.recentHistory)[number]): number {
  return entry.pricingResult.totalPv ?? entry.pricingResult.pv ?? 0;
}

const compareItems = (_idx: number) =>
  store.recentHistory.map((e, i) => ({
    title: `#${e.id} ${e.instrumentName}`,
    value: i,
  }));
</script>

<template>
  <v-card v-if="store.recentHistory.length > 0">
    <v-card-title class="d-flex align-center justify-space-between">
      <span>History</span>
      <v-btn
        v-if="store.resultHistory.length >= 2"
        size="small"
        :variant="store.compareMode ? 'flat' : 'text'"
        :color="store.compareMode ? 'primary' : undefined"
        prepend-icon="mdi-compare"
        @click="toggleCompareMode"
      >
        {{ store.compareMode ? 'Exit Compare' : 'Compare' }}
      </v-btn>
    </v-card-title>

    <v-card-text>
      <!-- History List -->
      <v-list density="compact" class="pa-0">
        <v-list-item
          v-for="entry in store.recentHistory"
          :key="entry.id"
          rounded="lg"
          @click="restoreFromHistory(entry)"
        >
          <v-list-item-title class="text-body-2">
            {{ entry.instrumentName }}
            <span class="text-caption text-medium-emphasis ml-2">
              {{ new Date(entry.timestamp).toLocaleTimeString() }}
            </span>
          </v-list-item-title>

          <template #append>
            <span
              class="text-body-2 font-weight-bold"
              :class="entryPv(entry) >= 0 ? 'text-success' : 'text-error'"
            >
              {{ formatCurrency(entryPv(entry)) }}
            </span>
          </template>
        </v-list-item>
      </v-list>

      <!-- Compare Mode -->
      <template v-if="store.compareMode && store.comparedResults">
        <v-divider class="my-3" />

        <v-row dense>
          <v-col cols="6">
            <v-select
              v-model.number="store.compareIndices[0]"
              :items="compareItems(0)"
              density="compact"
              label="Result A"
              hide-details
            />
          </v-col>
          <v-col cols="6">
            <v-select
              v-model.number="store.compareIndices[1]"
              :items="compareItems(1)"
              density="compact"
              label="Result B"
              hide-details
            />
          </v-col>
        </v-row>

        <!-- Side-by-side PV -->
        <v-row dense class="mt-2">
          <v-col cols="6">
            <v-sheet rounded="lg" class="pa-3 text-center" color="surface-variant">
              <div class="text-caption text-medium-emphasis">Result A</div>
              <div
                class="text-subtitle-1 font-weight-bold"
                :class="
                  (store.comparedResults.a.pricingResult.totalPv ?? 0) >= 0
                    ? 'text-success'
                    : 'text-error'
                "
              >
                {{
                  formatCurrency(
                    store.comparedResults.a.pricingResult.totalPv ??
                      store.comparedResults.a.pricingResult.pv ??
                      0,
                  )
                }}
              </div>
            </v-sheet>
          </v-col>
          <v-col cols="6">
            <v-sheet rounded="lg" class="pa-3 text-center" color="surface-variant">
              <div class="text-caption text-medium-emphasis">Result B</div>
              <div
                class="text-subtitle-1 font-weight-bold"
                :class="
                  (store.comparedResults.b.pricingResult.totalPv ?? 0) >= 0
                    ? 'text-success'
                    : 'text-error'
                "
              >
                {{
                  formatCurrency(
                    store.comparedResults.b.pricingResult.totalPv ??
                      store.comparedResults.b.pricingResult.pv ??
                      0,
                  )
                }}
              </div>
            </v-sheet>
          </v-col>
        </v-row>

        <!-- Changed Parameters -->
        <v-table v-if="store.changedParams.length > 0" density="compact" class="mt-3">
          <thead>
            <tr>
              <th class="text-caption">Parameter</th>
              <th class="text-caption text-right">A</th>
              <th class="text-caption text-center" style="width: 24px"></th>
              <th class="text-caption">B</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="cp in store.changedParams" :key="cp.name">
              <td class="text-caption text-medium-emphasis">{{ cp.name }}</td>
              <td class="text-caption text-right text-error">{{ cp.valueA }}</td>
              <td class="text-center">
                <v-icon icon="mdi-arrow-right" size="12" />
              </td>
              <td class="text-caption text-success">{{ cp.valueB }}</td>
            </tr>
          </tbody>
        </v-table>
      </template>
    </v-card-text>
  </v-card>
</template>
