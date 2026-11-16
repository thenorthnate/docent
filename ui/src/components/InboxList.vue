<script setup lang="ts">
import { computed, nextTick, onMounted, watch, type Component } from "vue";
import { CircleCheck, CircleDashed, CircleX, Clock } from "@lucide/vue";
import { storeToRefs } from "pinia";
import { Button } from "@/components/ui/button";
import { formatAge } from "@/lib/age";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { CI_LABELS, REVIEW_LABELS, pullKey, sizeBreakdown, sizeLabel } from "@/lib/pull";
import { SECTION_EMPTY, SECTION_LABELS } from "@/lib/sections";
import { useInboxStore, type CiStatus, type PullSummary, type ReviewState } from "@/stores/inbox";

/** The selected PR's key, from `pullKey`; `null` when none is. */
const selectedPull = defineModel<string | null>("selectedPull", { default: null });
const emit = defineEmits<{ open: [pull: PullSummary] }>();

const inbox = useInboxStore();
const { selectedSection, selected, hasWatchedRepos, loading } = storeToRefs(inbox);
const pulls = computed(() => selectedSection.value.pulls);
const failures = computed(() => selectedSection.value.failures);
const incomplete = computed(() => selectedSection.value.incomplete);
const error = computed(() => selectedSection.value.error);

/** Each CI status's icon and colour. */
const CI_ICONS: Record<CiStatus, { icon: Component; tone: string }> = {
  passing: { icon: CircleCheck, tone: "text-emerald-600 dark:text-emerald-400" },
  failing: { icon: CircleX, tone: "text-destructive" },
  pending: { icon: Clock, tone: "text-amber-600 dark:text-amber-400" },
  no_checks: { icon: CircleDashed, tone: "text-muted-foreground" },
};

/** Each review state's chip colour. */
const REVIEW_TONES: Record<ReviewState, string> = {
  approved: "text-emerald-600 dark:text-emerald-400",
  changes_requested: "text-destructive",
  review_required: "text-muted-foreground",
  reviewed: "text-muted-foreground",
  no_reviews: "text-muted-foreground",
};

/** Rows name their host only when the PRs span more than one. */
const showHost = computed(() => new Set(pulls.value.map((pr) => pr.host)).size > 1);

/** Each row's element id, so the listbox can point at the selected one. */
const rowId = (index: number) => `inbox-row-${index}`;
const selectedIndex = computed(() => pulls.value.findIndex((pr) => pullKey(pr) === selectedPull.value));

// Keep the selected row on screen as j/k move it.
watch(selectedIndex, async (index) => {
  if (index === -1) return;
  await nextTick();
  document.getElementById(rowId(index))?.scrollIntoView({ block: "nearest" });
});

onMounted(inbox.load);
</script>

<template>
  <section
    aria-labelledby="inbox-list-heading"
    class="space-y-2"
  >
    <div class="flex items-center justify-between gap-2">
      <h2
        id="inbox-list-heading"
        class="text-sm font-semibold"
      >
        {{ SECTION_LABELS[selected] }}
      </h2>
      <Button
        variant="ghost"
        size="xs"
        aria-label="Refresh inbox"
        :disabled="loading"
        @click="inbox.load"
      >
        {{ loading ? "Refreshing…" : "Refresh" }}
      </Button>
    </div>
    <p
      v-if="hasWatchedRepos === false"
      class="text-sm text-muted-foreground"
    >
      Watch a repo in the drawer to see its PRs here.
    </p>
    <template v-else-if="hasWatchedRepos">
      <!-- Every field below came from the forge: text interpolation only, never v-html. -->
      <ul
        v-if="pulls.length"
        role="listbox"
        tabindex="0"
        aria-labelledby="inbox-list-heading"
        :aria-activedescendant="selectedIndex === -1 ? undefined : rowId(selectedIndex)"
        class="divide-y rounded-md border"
      >
        <li
          v-for="(pr, index) in pulls"
          :id="rowId(index)"
          :key="pullKey(pr)"
          role="option"
          :aria-selected="index === selectedIndex"
          data-testid="inbox-row"
          class="cursor-default space-y-1 px-3 py-2 text-sm select-none aria-selected:bg-accent"
          @click="selectedPull = pullKey(pr)"
          @dblclick="emit('open', pr)"
        >
          <div class="flex items-center gap-1.5 text-xs text-muted-foreground">
            <Tooltip v-if="pr.details">
              <TooltipTrigger as-child>
                <span
                  data-testid="row-ci"
                  class="shrink-0"
                  :class="CI_ICONS[pr.details.ci].tone"
                >
                  <component
                    :is="CI_ICONS[pr.details.ci].icon"
                    class="size-3.5"
                    aria-hidden="true"
                  />
                  <span class="sr-only">{{ CI_LABELS[pr.details.ci] }}</span>
                </span>
              </TooltipTrigger>
              <TooltipContent>{{ CI_LABELS[pr.details.ci] }}</TooltipContent>
            </Tooltip>
            <span class="min-w-0 truncate">
              <template v-if="showHost">{{ pr.host }}:</template>{{ pr.owner }}/{{ pr.repo }} · {{ pr.author }}
            </span>
            <time
              class="ml-auto shrink-0 tabular-nums"
              :datetime="pr.created_at"
              :title="pr.created_at"
            >{{ formatAge(pr.created_at) }}</time>
          </div>
          <p class="truncate font-medium">
            {{ pr.title }}
          </p>
          <div
            v-if="pr.details"
            class="flex flex-wrap gap-1.5 text-xs"
          >
            <span
              data-testid="row-review"
              class="rounded-sm border px-1.5"
              :class="REVIEW_TONES[pr.details.review]"
            >{{ REVIEW_LABELS[pr.details.review] }}</span>
            <Tooltip>
              <TooltipTrigger as-child>
                <span
                  data-testid="row-size"
                  class="rounded-sm bg-muted px-1.5 tabular-nums"
                >{{ sizeLabel(pr.details) }}</span>
              </TooltipTrigger>
              <TooltipContent>{{ sizeBreakdown(pr.details) }}</TooltipContent>
            </Tooltip>
          </div>
        </li>
      </ul>
      <!-- Only claim there's nothing when every host answered in full. -->
      <p
        v-else-if="!failures.length && !incomplete.length && !error"
        class="text-sm text-muted-foreground"
      >
        {{ SECTION_EMPTY[selected] }}
      </p>
      <p
        v-for="host in incomplete"
        :key="host"
        data-testid="inbox-incomplete"
        class="text-xs text-muted-foreground"
      >
        {{ host }} didn't return all its results, so some PRs may be missing.
      </p>
    </template>
    <div
      v-if="error || failures.length"
      role="alert"
      class="space-y-1 text-sm text-destructive"
    >
      <p v-if="error">
        {{ error }}
      </p>
      <p
        v-for="failure in failures"
        :key="failure.host"
      >
        {{ failure.error.message }}
      </p>
    </div>
  </section>
</template>
