<script setup lang="ts">
import { computed, onMounted } from "vue";
import { storeToRefs } from "pinia";
import { Button } from "@/components/ui/button";
import { formatAge } from "@/lib/age";
import { SECTION_EMPTY, SECTION_LABELS } from "@/lib/sections";
import { useInboxStore } from "@/stores/inbox";

const inbox = useInboxStore();
const { selectedSection, selected, hasWatchedRepos, loading } = storeToRefs(inbox);
const pulls = computed(() => selectedSection.value.pulls);
const failures = computed(() => selectedSection.value.failures);
const error = computed(() => selectedSection.value.error);

/** Rows name their host only when the PRs span more than one. */
const showHost = computed(() => new Set(pulls.value.map((pr) => pr.host)).size > 1);

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
        class="divide-y rounded-md border"
      >
        <li
          v-for="pr in pulls"
          :key="`${pr.host}/${pr.owner}/${pr.repo}#${pr.number}`"
          data-testid="inbox-row"
          class="flex items-baseline gap-3 px-3 py-2 text-sm"
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
        {{ SECTION_EMPTY[selected] }}
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
