<script setup lang="ts">
import { computed, onMounted } from "vue";
import { storeToRefs } from "pinia";
import { Button } from "@/components/ui/button";
import { formatAge } from "@/lib/age";
import { pullKey } from "@/lib/pull";
import { useInboxStore, type PullSummary } from "@/stores/inbox";

/** The selected row's key, from `pullKey`; `null` when none is. */
const selected = defineModel<string | null>("selected", { default: null });
const emit = defineEmits<{ open: [pull: PullSummary] }>();

const inbox = useInboxStore();
const { pulls, failures, error, hasWatchedRepos, loading } = storeToRefs(inbox);

/** Rows name their host only when the PRs span more than one. */
const showHost = computed(() => new Set(pulls.value.map((pr) => pr.host)).size > 1);

onMounted(inbox.load);
</script>

<template>
  <section
    aria-labelledby="review-requested-heading"
    class="space-y-2"
  >
    <div class="flex items-center justify-between gap-2">
      <h2
        id="review-requested-heading"
        class="text-sm font-semibold"
      >
        Review requested
      </h2>
      <Button
        variant="ghost"
        size="xs"
        aria-label="Refresh review requests"
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
      Watch a repo in the drawer to see its review requests here.
    </p>
    <template v-else-if="hasWatchedRepos">
      <!-- Every field below came from the forge: text interpolation only, never v-html. -->
      <ul
        v-if="pulls.length"
        role="listbox"
        aria-labelledby="review-requested-heading"
        class="divide-y rounded-md border"
      >
        <li
          v-for="pr in pulls"
          :key="pullKey(pr)"
          role="option"
          :aria-selected="pullKey(pr) === selected"
          data-testid="review-requested-row"
          class="flex cursor-default items-baseline gap-3 px-3 py-2 text-sm select-none aria-selected:bg-accent"
          @click="selected = pullKey(pr)"
          @dblclick="emit('open', pr)"
        >
          <span class="min-w-0 flex-1 truncate font-medium">{{ pr.title }}</span>
          <span class="shrink-0 text-muted-foreground">
            <template v-if="showHost">{{ pr.host }}:</template>{{ pr.owner }}/{{ pr.repo }}
          </span>
          <span class="shrink-0 text-muted-foreground">{{ pr.author }}</span>
          <time
            class="w-16 shrink-0 text-right text-muted-foreground tabular-nums"
            :datetime="pr.created_at"
            :title="pr.created_at"
          >{{ formatAge(pr.created_at) }}</time>
        </li>
      </ul>
      <!-- Only claim there's nothing when every host answered. -->
      <p
        v-else-if="!failures.length && !error"
        class="text-sm text-muted-foreground"
      >
        No review requests in your watched repos.
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
