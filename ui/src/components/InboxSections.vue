<script setup lang="ts">
import { storeToRefs } from "pinia";
import { SECTION_LABELS, SECTIONS, type Section } from "@/lib/sections";
import { useInboxStore } from "@/stores/inbox";

const inbox = useInboxStore();
const { sections, selected } = storeToRefs(inbox);

/** Why a section's count may be short, or `undefined` when it is exact. */
function countCaveat(section: Section): string | undefined {
  const { failures, incomplete } = sections.value[section];
  if (failures.length) return "Not every host answered, so there may be more.";
  if (incomplete.length) return "A host didn't return all its results, so there may be more.";
  return undefined;
}
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
            :title="countCaveat(section)"
          >{{ sections[section].pulls.length }}<template v-if="countCaveat(section)">+</template></span>
        </button>
      </li>
    </ul>
  </nav>
</template>
