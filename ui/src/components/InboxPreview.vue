<script setup lang="ts">
import { Button } from "@/components/ui/button";
import type { PullSummary } from "@/stores/inbox";

defineProps<{ pull: PullSummary | null }>();
const emit = defineEmits<{ open: [pull: PullSummary] }>();
</script>

<template>
  <section
    data-testid="inbox-preview"
    aria-label="Preview"
    class="space-y-3"
  >
    <p
      v-if="!pull"
      class="text-sm text-muted-foreground"
    >
      Select a pull request to preview it.
    </p>
    <!-- Every field below came from the forge: text interpolation only, never v-html. -->
    <template v-else>
      <div class="space-y-1">
        <h2 class="text-base font-semibold break-words">
          {{ pull.title }}
        </h2>
        <p class="text-sm text-muted-foreground">
          {{ pull.owner }}/{{ pull.repo }} #{{ pull.number }} · {{ pull.author }}
        </p>
      </div>
      <Button
        size="sm"
        @click="emit('open', pull)"
      >
        Open review
      </Button>
      <p
        v-if="pull.body?.trim()"
        data-testid="pr-description"
        class="text-sm break-words whitespace-pre-wrap"
        v-text="pull.body"
      />
      <p
        v-else
        class="text-sm text-muted-foreground italic"
      >
        No description provided.
      </p>
    </template>
  </section>
</template>
