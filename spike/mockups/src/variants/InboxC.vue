<script setup lang="ts">
// PROTOTYPE inbox variant C, "Board": each inbox section is a column of cards. Gives an
// at-a-glance sense of load; cards carry more context (summary line, lens results).
import { Eye, Inbox, Settings, Telescope } from '@lucide/vue'
import { filtered, inbox, unbound } from '../data/mock'
import InboxBits from '../components/InboxBits.vue'
import { btn, reviewLabel } from '../components/bits'
</script>

<template>
  <div class="flex h-full flex-col">
    <header class="flex items-center gap-3 border-b px-4 py-2">
      <Inbox class="size-4" /><span class="font-semibold">Inbox</span>
      <span class="ml-3 flex items-center gap-1 text-xs text-muted-foreground"><Eye class="size-3.5" />{{ filtered.count }} hidden in unwatched repos</span>
      <span class="text-xs text-muted-foreground">· {{ unbound.repo }} needs a clone</span>
      <div class="ml-auto flex gap-2"><button :class="btn"><Telescope class="size-3.5" /> Explore</button><button :class="btn"><Settings class="size-3.5" /></button></div>
    </header>
    <div class="flex min-h-0 flex-1 gap-3 overflow-x-auto bg-muted/30 p-3 pb-16">
      <div v-for="(rows, name) in inbox" :key="name" class="flex w-72 shrink-0 flex-col">
        <div class="mb-2 flex items-center px-1 text-xs font-semibold">{{ name }}<span class="ml-1.5 font-normal text-muted-foreground">{{ rows.length }}</span></div>
        <div class="space-y-2 overflow-auto">
          <div v-for="p in rows" :key="p.n" class="rounded-lg border bg-card p-2.5 shadow-sm hover:ring-1 hover:ring-ring">
            <div class="flex items-center gap-1.5 text-[11px] text-muted-foreground"><InboxBits :p="p" part="ci" />{{ p.repo.split('/')[1] }} #{{ p.n }}<span class="ml-auto">{{ p.age }}</span></div>
            <div class="mt-1 font-medium leading-snug" :class="p.draft ? 'text-muted-foreground' : ''">{{ p.title }}</div>
            <div class="mt-1.5 flex flex-wrap items-center gap-1.5 text-[11px]">
              <span class="text-muted-foreground">{{ p.author }}</span>
              <span class="rounded bg-muted px-1.5">{{ p.size }}</span>
              <span v-if="p.review !== 'none'" class="text-muted-foreground">{{ reviewLabel[p.review] }}</span>
            </div>
            <div class="mt-1.5 flex flex-wrap gap-1"><InboxBits :p="p" part="updated" /><InboxBits :p="p" part="trust" />
              <span v-if="p.findings" class="rounded-full bg-amber-500/15 px-2 py-0.5 text-[11px] text-amber-500">{{ p.findings }} findings</span></div>
          </div>
          <div v-if="!rows.length" class="rounded-lg border border-dashed p-4 text-center text-xs text-muted-foreground">Pin a PR to keep it here</div>
        </div>
      </div>
    </div>
  </div>
</template>
