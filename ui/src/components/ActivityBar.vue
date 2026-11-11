<script setup lang="ts">
import { Inbox, GitPullRequest, Settings } from "@lucide/vue";
import RailButton from "@/components/RailButton.vue";

const mainAreas = [
  { area: "inbox", label: "Inbox", icon: Inbox },
  { area: "review", label: "Review", icon: GitPullRequest },
] as const;
const footerAreas = [{ area: "settings", label: "Settings", icon: Settings }] as const;
</script>

<template>
  <nav
    aria-label="Activity bar"
    class="flex w-12 shrink-0 flex-col items-center gap-1 border-r bg-sidebar py-2"
  >
    <RailButton
      v-for="item in mainAreas"
      :key="item.area"
      v-bind="item"
    />
    <div class="flex-1" />
    <RailButton
      v-for="item in footerAreas"
      :key="item.area"
      v-bind="item"
    />
  </nav>
</template>
