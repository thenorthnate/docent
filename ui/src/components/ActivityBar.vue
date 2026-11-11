<script setup lang="ts">
import { computed } from "vue";
import { useRoute } from "vue-router";
import { Inbox, GitPullRequest, Settings } from "@lucide/vue";
import { Button } from "@/components/ui/button";
import { useShellStore } from "@/stores/shell";

const route = useRoute();
const shell = useShellStore();
const activeArea = computed(() => route.meta.area);

const top = [
  { area: "inbox", label: "Inbox", icon: Inbox },
  { area: "review", label: "Review", icon: GitPullRequest },
] as const;
const bottom = [{ area: "settings", label: "Settings", icon: Settings }] as const;
</script>

<template>
  <nav
    aria-label="Activity bar"
    class="flex w-12 shrink-0 flex-col items-center gap-1 border-r bg-sidebar py-2"
  >
    <template
      v-for="(group, index) in [top, bottom]"
      :key="index"
    >
      <div
        v-if="index === 1"
        class="flex-1"
      />
      <Button
        v-for="item in group"
        :key="item.area"
        variant="ghost"
        size="icon-lg"
        :aria-label="item.label"
        :title="item.label"
        :aria-current="activeArea === item.area ? 'page' : undefined"
        :class="activeArea === item.area ? 'bg-sidebar-accent text-sidebar-accent-foreground' : 'text-muted-foreground'"
        @click="shell.selectArea(item.area)"
      >
        <component :is="item.icon" />
      </Button>
    </template>
  </nav>
</template>
