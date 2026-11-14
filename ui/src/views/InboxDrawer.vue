<script setup lang="ts">
import InboxSections from "@/components/InboxSections.vue";
import WatchedRepos from "@/components/WatchedRepos.vue";
</script>

<template>
  <div class="space-y-4 p-3">
    <h2 class="text-sm font-semibold">
      Inbox
    </h2>
    <InboxSections />
    <WatchedRepos />
  </div>
</template>
