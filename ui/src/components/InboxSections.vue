<script setup lang="ts">
import { storeToRefs } from "pinia";
import { SECTION_LABELS, SECTIONS } from "@/lib/sections";
import { useInboxStore } from "@/stores/inbox";

const inbox = useInboxStore();
const { sections, selected } = storeToRefs(inbox);
</script>

<template>
  <nav
    aria-labelledby="inbox-sections-heading"
    class="space-y-1"
  >
    <h3
      id="inbox-sections-heading"
      class="text-xs font-semibold tracking-wide text-muted-foreground uppercase"
    >
      Sections
    </h3>
    <ul class="space-y-0.5 text-sm">
      <li
        v-for="section in SECTIONS"
        :key="section"
      >
        <button
          type="button"
          data-testid="inbox-section"
          :aria-current="section === selected"
          class="flex w-full items-center justify-between gap-2 rounded-md px-2 py-1 text-left hover:bg-accent aria-[current=true]:bg-accent aria-[current=true]:font-medium"
          @click="inbox.select(section)"
        >
          <span class="truncate">{{ SECTION_LABELS[section] }}</span>
          <!-- No count until the section has answered, or when it couldn't be read. -->
          <span
            v-if="sections[section].loaded && !sections[section].error"
            class="shrink-0 text-xs text-muted-foreground tabular-nums"
            :title="sections[section].failures.length ? 'Not every host answered, so there may be more.' : undefined"
          >{{ sections[section].pulls.length }}<template v-if="sections[section].failures.length">+</template></span>
        </button>
      </li>
    </ul>
  </nav>
</template>
