<script setup lang="ts">
import { computed, type Component } from "vue";
import { Button } from "@/components/ui/button";
import { useShellStore } from "@/stores/shell";
import type { Area } from "@/router";

const props = defineProps<{ area: Area; label: string; icon: Component }>();

const shell = useShellStore();
const active = computed(() => shell.activeArea === props.area);
</script>

<template>
  <Button
    variant="ghost"
    size="icon-lg"
    :aria-label="label"
    :title="label"
    :aria-current="active ? 'page' : undefined"
    :aria-expanded="active ? shell.drawerOpen : undefined"
    :class="active ? 'bg-sidebar-accent text-sidebar-accent-foreground' : 'text-muted-foreground'"
    @click="shell.selectArea(area)"
  >
    <component :is="icon" />
  </Button>
</template>
