<script setup lang="ts">
import MarkdownView from "@/components/MarkdownView.vue";
import { formatAge } from "@/lib/age";
import { CHECK_LABELS, VERDICT_LABELS, type CheckState, type PullOverview } from "@/lib/review";

defineProps<{ overview: PullOverview }>();

const CHECK_TONES: Record<CheckState, string> = {
  passing: "text-emerald-600 dark:text-emerald-400",
  failing: "text-destructive",
  pending: "text-amber-600 dark:text-amber-400",
  skipped: "text-muted-foreground",
};
</script>

<template>
  <!-- Every field came from the forge: text interpolation, or MarkdownView for Markdown. -->
  <div class="space-y-6">
    <section
      aria-label="Description"
      class="rounded-md border p-4"
    >
      <MarkdownView
        v-if="overview.body"
        :source="overview.body"
      />
      <p
        v-else
        class="text-sm text-muted-foreground italic"
      >
        No description provided.
      </p>
    </section>

    <section
      aria-label="Checks"
      class="space-y-2"
    >
      <h2 class="text-sm font-semibold">
        Checks
      </h2>
      <ul
        v-if="overview.checks.length"
        class="divide-y rounded-md border text-sm"
      >
        <li
          v-for="(check, i) in overview.checks"
          :key="i"
          class="flex items-center justify-between gap-4 px-3 py-1.5"
        >
          <span class="truncate">{{ check.name }}</span>
          <span :class="CHECK_TONES[check.state]">{{ CHECK_LABELS[check.state] }}</span>
        </li>
      </ul>
      <p
        v-else
        class="text-sm text-muted-foreground"
      >
        No checks on the latest commit.
      </p>
    </section>

    <section
      aria-label="Timeline"
      class="space-y-2"
    >
      <h2 class="text-sm font-semibold">
        Timeline
      </h2>
      <ol class="space-y-3">
        <li
          v-for="(item, i) in overview.timeline"
          :key="i"
          class="text-sm"
        >
          <p
            v-if="item.kind === 'commit'"
            class="text-muted-foreground"
          >
            <span class="font-medium text-foreground">{{ item.author }}</span> committed
            <code class="font-mono text-xs">{{ item.sha.slice(0, 7) }}</code>&#32;<span class="text-foreground">{{ item.headline }}</span>
            · {{ formatAge(item.at) }}
          </p>
          <div
            v-else
            class="rounded-md border"
          >
            <p class="border-b bg-muted/40 px-3 py-1.5 text-muted-foreground">
              <span class="font-medium text-foreground">{{ item.author }}</span>
              {{ item.kind === "review" ? VERDICT_LABELS[item.verdict] : "commented" }}
              · {{ formatAge(item.at) }}
            </p>
            <div
              v-if="item.body"
              class="px-3 py-2"
            >
              <MarkdownView :source="item.body" />
            </div>
          </div>
        </li>
      </ol>
      <p
        v-if="overview.timeline_truncated"
        class="text-sm text-muted-foreground"
      >
        Showing the first 100 events.
      </p>
    </section>
  </div>
</template>
