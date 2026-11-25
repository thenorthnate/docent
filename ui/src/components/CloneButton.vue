<script setup lang="ts">
import { computed } from "vue";
import { Button } from "@/components/ui/button";
import { useClonesStore } from "@/stores/clones";

/** Clones watched repo `id` (PLAN §4, "Repo binding"). */
const props = defineProps<{ id: number; disabled?: boolean }>();

const CLONE_TOOLTIP = "Make a full clone into a folder you pick, using your existing git credentials.";

const clones = useClonesStore();
const update = computed(() => clones.running[props.id]);
const progress = computed(() => {
  if (!update.value) return null;
  const { phase, percent } = update.value;
  return percent === null ? `${phase}…` : `${phase} ${percent}%`;
});
</script>

<template>
  <Button
    variant="outline"
    size="xs"
    :title="CLONE_TOOLTIP"
    :disabled="disabled || !!update"
    @click="clones.start(id)"
  >
    {{ update ? "Cloning…" : "Clone" }}
  </Button>
  <p
    v-if="progress"
    role="status"
    class="w-full truncate text-xs text-muted-foreground"
    :title="progress"
  >
    {{ progress }}
  </p>
  <p
    v-if="clones.errors[id]"
    role="alert"
    class="w-full text-xs break-words text-destructive"
  >
    {{ clones.errors[id] }}
  </p>
</template>
