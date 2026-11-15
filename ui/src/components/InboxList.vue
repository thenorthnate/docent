<script setup lang="ts">
import { computed, nextTick, onMounted, watch } from "vue";
import { storeToRefs } from "pinia";
import { Button } from "@/components/ui/button";
import { formatAge } from "@/lib/age";
import { CI_LABELS, REVIEW_LABELS, formatSize, pullKey } from "@/lib/pull";
import { SECTION_EMPTY, SECTION_LABELS } from "@/lib/sections";
import { useInboxStore, type PullSummary } from "@/stores/inbox";

/** The selected PR's key, from `pullKey`; `null` when none is. */
const selectedPull = defineModel<string | null>("selectedPull", { default: null });
const emit = defineEmits<{ open: [pull: PullSummary] }>();

const inbox = useInboxStore();
const { selectedSection, selected, hasWatchedRepos, loading } = storeToRefs(inbox);
const pulls = computed(() => selectedSection.value.pulls);
const failures = computed(() => selectedSection.value.failures);
const incomplete = computed(() => selectedSection.value.incomplete);
const error = computed(() => selectedSection.value.error);

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
          class="flex cursor-default items-baseline gap-3 px-3 py-2 text-sm select-none aria-selected:bg-accent"
          @click="selectedPull = pullKey(pr)"
          @dblclick="emit('open', pr)"
        >
          <span class="min-w-0 flex-1 truncate font-medium">{{ pr.title }}</span>
          <span class="shrink-0 text-muted-foreground">
            <template v-if="showHost">{{ pr.host }}:</template>{{ pr.owner }}/{{ pr.repo }}
          </span>
          <span class="shrink-0 text-muted-foreground">{{ pr.author }}</span>
          <template v-if="pr.details">
            <span
              data-testid="row-ci"
              class="shrink-0"
              :class="pr.details.ci === 'failing' ? 'text-destructive' : 'text-muted-foreground'"
            >{{ CI_LABELS[pr.details.ci] }}</span>
            <span
              data-testid="row-review"
              class="shrink-0"
              :class="pr.details.review === 'changes_requested' ? 'text-destructive' : 'text-muted-foreground'"
            >{{ REVIEW_LABELS[pr.details.review] }}</span>
            <span
              data-testid="row-size"
              class="shrink-0 text-muted-foreground tabular-nums"
            >{{ formatSize(pr.details) }}</span>
          </template>
          <time
            class="w-16 shrink-0 text-right text-muted-foreground tabular-nums"
            :datetime="pr.created_at"
            :title="pr.created_at"
          >{{ formatAge(pr.created_at) }}</time>
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
