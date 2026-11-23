<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { RouterLink, useRoute } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import PrWorktree from "@/components/PrWorktree.vue";
import PullOverview from "@/components/PullOverview.vue";
import { messageOf } from "@/lib/ipc";
import { CI_ICONS, CI_LABELS, pullKey } from "@/lib/pull";
import type { PullOverview as Overview } from "@/lib/review";

const route = useRoute();

/** The opened PR, from the `review-pr` route; `null` on plain `/review`. */
const pr = computed(() => {
  if (route.name !== "review-pr") return null;
  const { host, owner, repo, number } = route.params as Record<string, string>;
  return { host, owner, repo, number: Number(number) };
});

const overview = ref<Overview | null>(null);
const error = ref<string | null>(null);
const loading = ref(false);

/** Counts loads, so only the latest one started may write its answer. */
let latest = 0;

// Keyed on the PR itself, so a route change that keeps the same PR doesn't refetch.
watch(
  () => (pr.value ? pullKey(pr.value) : null),
  async () => {
    const opened = pr.value;
    const id = ++latest;
    overview.value = null;
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
    />

    <div
      role="tablist"
      class="flex gap-4 border-b text-sm"
    >
      <button
        role="tab"
        aria-selected="true"
        class="-mb-px border-b-2 border-foreground pb-2 font-medium"
      >
        Overview
      </button>
    </div>

    <p
      v-if="error"
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
