<script setup lang="ts">
// PROTOTYPE: read-only code viewer stand-in (CodeMirror 6 in the real app) with the hover `?`.
import { ref } from 'vue'
import { Bookmark } from '@lucide/vue'
import type { CodeLine } from '../data/explore'
defineProps<{ lines: CodeLine[]; blame?: { line: number; text: string }; bookmarkLine?: number }>()
const hover = ref<number | null>(null)
</script>

<template>
  <div class="font-mono text-[12px] leading-[20px]">
    <div v-for="l in lines" :key="l.n" class="relative flex" :class="l.mark === 'focus' ? 'bg-sky-500/15' : l.mark === 'link' ? 'bg-amber-500/15' : ''"
      @mouseenter="hover = l.n" @mouseleave="hover = null">
      <span class="flex w-5 shrink-0 items-center justify-center"><Bookmark v-if="bookmarkLine === l.n" class="size-3 fill-sky-400 text-sky-400" /></span>
      <span class="w-9 shrink-0 select-none pr-3 text-right text-muted-foreground/70">{{ l.n }}</span>
      <pre class="whitespace-pre pr-10">{{ l.text }}</pre>
      <span v-if="blame && blame.line === l.n" class="ml-6 font-sans text-[11px] italic text-muted-foreground">{{ blame.text }}</span>
      <button v-if="hover === l.n" class="absolute right-2 top-0.5 flex size-4 items-center justify-center rounded bg-primary text-[10px] font-bold text-primary-foreground">?</button>
    </div>
  </div>
</template>
