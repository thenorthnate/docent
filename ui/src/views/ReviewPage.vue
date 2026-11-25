<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { RouterLink, useRoute } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import DiffViewer from "@/components/DiffViewer.vue";
import PrWorktree from "@/components/PrWorktree.vue";
import PullOverview from "@/components/PullOverview.vue";
import { messageOf } from "@/lib/ipc";
import { CI_ICONS, CI_LABELS, pullKey } from "@/lib/pull";
import type { PullOverview as Overview } from "@/lib/review";
import { useReviewStore } from "@/stores/review";

const route = useRoute();
const review = useReviewStore();

/** The opened PR, from the `review-pr` route; `null` on plain `/review`. */
const pr = computed(() => {
  if (route.name !== "review-pr") return null;
  const { host, owner, repo, number } = route.params as Record<string, string>;
  return { host, owner, repo, number: Number(number) };
});

const overview = ref<Overview | null>(null);
const error = ref<string | null>(null);
const loading = ref(false);

/** The head the PR's worktree is at, once it's ready. */
const headSha = ref<string | null>(null);

/** Counts loads, so only the latest one started may write its answer. */
let latest = 0;

const TABS = [
  { id: "overview", label: "Overview" },
  { id: "changes", label: "Changes" },
] as const;
const tab = ref<(typeof TABS)[number]["id"]>("overview");

// Picking a file in the drawer shows its changes.
watch(
  () => review.currentFile,
  (file) => {
    if (file) tab.value = "changes";
  },
);

// Keyed on the PR itself, so a route change that keeps the same PR doesn't refetch.
watch(
  () => (pr.value ? pullKey(pr.value) : null),
  async () => {
    const opened = pr.value;
    const id = ++latest;
    overview.value = null;
    headSha.value = null;
    tab.value = "overview";
    error.value = null;
    if (!opened) return;
    loading.value = true;
    try {
      const found = await invoke<Overview>("pull_overview", opened);
      if (id === latest) overview.value = found;
    } catch (err) {
      if (id === latest) error.value = messageOf(err);
    } finally {
      if (id === latest) loading.value = false;
    }
  },
  { immediate: true },
);

// The drawer's files: known once the forge has given the base and the worktree the head.
watch(
  [() => (pr.value ? pullKey(pr.value) : null), () => overview.value?.base_sha, headSha],
  ([, base_sha, head_sha]) => {
    void review.show(pr.value && base_sha && head_sha ? { ...pr.value, base_sha, head_sha } : null);
  },
  { immediate: true },
);
</script>

<template>
  <div
    v-if="pr"
    class="space-y-4 p-6"
  >
    <header class="flex flex-wrap items-center justify-between gap-x-4 gap-y-2">
      <nav
        aria-label="Breadcrumb"
        class="min-w-0"
      >
        <ol class="flex items-center gap-2 text-sm text-muted-foreground">
          <li>
            <RouterLink
              to="/inbox"
              class="hover:text-foreground hover:underline"
            >
              Inbox
            </RouterLink>
          </li>
          <li class="shrink-0 before:mr-2 before:content-['›']">
            {{ pr.owner }}/{{ pr.repo }}
          </li>
          <li
            aria-current="page"
            class="truncate font-medium text-foreground before:mr-2 before:font-normal before:text-muted-foreground before:content-['›']"
          >
            <template v-if="overview">
              {{ overview.title }}
            </template>
            #{{ pr.number }}
          </li>
        </ol>
      </nav>
      <span
        v-if="overview"
        data-testid="ci-status"
        class="flex shrink-0 items-center gap-1.5 text-sm"
        :class="CI_ICONS[overview.ci].tone"
      >
        <component
          :is="CI_ICONS[overview.ci].icon"
          class="size-4"
          aria-hidden="true"
        />
        {{ CI_LABELS[overview.ci] }}
      </span>
    </header>

    <PrWorktree
      :host="pr.host"
      :owner="pr.owner"
      :repo="pr.repo"
      :number="pr.number"
      @ready="headSha = $event"
    />

    <div
      role="tablist"
      class="flex gap-4 border-b text-sm"
    >
      <button
        v-for="item in TABS"
        :key="item.id"
        role="tab"
        :aria-selected="tab === item.id"
        class="-mb-px border-b-2 border-transparent pb-2 text-muted-foreground aria-selected:border-foreground aria-selected:font-medium aria-selected:text-foreground"
        @click="tab = item.id"
      >
        {{ item.label }}
      </button>
    </div>

    <DiffViewer
      v-if="tab === 'changes'"
      role="tabpanel"
    />
    <p
      v-else-if="error"
      role="alert"
      class="text-sm text-destructive"
    >
      {{ error }}
    </p>
    <p
      v-else-if="loading"
      class="text-sm text-muted-foreground"
    >
      Loading…
    </p>
    <PullOverview
      v-else-if="overview"
      role="tabpanel"
      :overview="overview"
    />
  </div>
  <div
    v-else
    class="p-6"
  >
    <h1 class="text-lg font-semibold">
      Review
    </h1>
    <p class="text-sm text-muted-foreground">
      Open a pull request from the Inbox to review it.
    </p>
  </div>
</template>
