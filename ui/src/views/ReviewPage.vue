<script setup lang="ts">
import { computed } from "vue";
import { RouterLink, useRoute } from "vue-router";

const route = useRoute();

/** The opened PR, from the `review-pr` route; `null` on plain `/review`. */
const pr = computed(() => {
  if (route.name !== "review-pr") return null;
  const { owner, repo, number } = route.params as Record<string, string>;
  return { owner, repo, number };
});
</script>

<template>
  <div
    v-if="pr"
    class="space-y-4 p-6"
  >
    <nav aria-label="Breadcrumb">
      <ol class="flex items-center gap-2 text-sm text-muted-foreground">
        <li>
          <RouterLink
            to="/inbox"
            class="hover:text-foreground hover:underline"
          >
            Inbox
          </RouterLink>
        </li>
        <li class="before:mr-2 before:content-['›']">
          {{ pr.owner }}/{{ pr.repo }}
        </li>
        <li
          aria-current="page"
          class="font-medium text-foreground before:mr-2 before:font-normal before:text-muted-foreground before:content-['›']"
        >
          #{{ pr.number }}
        </li>
      </ol>
    </nav>
    <p class="text-sm text-muted-foreground">
      The review view is coming in a later version.
    </p>
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
