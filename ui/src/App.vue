<script setup lang="ts">
import { RouterView } from "vue-router";
import ActivityBar from "@/components/ActivityBar.vue";
import { useShellStore } from "@/stores/shell";

const shell = useShellStore();
</script>

<template>
  <div class="flex h-screen bg-background text-foreground">
    <ActivityBar />
    <aside
      v-if="shell.drawerOpen"
      data-testid="drawer"
      class="w-64 shrink-0 overflow-y-auto border-r bg-sidebar text-sidebar-foreground"
    >
      <RouterView name="drawer" />
    </aside>
    <main class="min-w-0 flex-1 overflow-y-auto">
      <RouterView />
    </main>
  </div>
</template>
