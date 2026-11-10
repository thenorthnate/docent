<script setup lang="ts">
// PROTOTYPE inbox variant A, "Dense table": one scrolling page, a section per inbox
// bucket, one row per PR with fixed columns. Keyboard-first (j/k, enter). Linear-like.
import { Eye, Filter, Inbox, Plus, Settings, Telescope } from '@lucide/vue'
import { filtered, inbox, unbound } from '../data/mock'
import InboxBits from '../components/InboxBits.vue'
import { btn, reviewLabel } from '../components/bits'
</script>

<template>
  <div class="flex h-full flex-col">
    <header class="flex items-center gap-3 border-b px-4 py-2">
      <Inbox class="size-4" /><span class="font-semibold">Inbox</span>
      <div class="ml-4 flex gap-1 text-xs"><span class="rounded-md bg-accent px-2 py-1">All watched</span><span class="rounded-md px-2 py-1 text-muted-foreground">payments only</span><span class="rounded-md px-2 py-1 text-muted-foreground">Needs me</span><button class="px-1 text-muted-foreground"><Plus class="size-3.5" /></button></div>
      <div class="ml-auto flex items-center gap-2"><button :class="btn"><Filter class="size-3.5" /> Filter</button><button :class="btn"><Telescope class="size-3.5" /> Explore</button><button :class="btn"><Settings class="size-3.5" /></button></div>
    </header>
    <div class="flex items-center gap-2 border-b bg-card px-4 py-1.5 text-xs text-muted-foreground">
      <Eye class="size-3.5" /> {{ filtered.count }} review requests in unwatched repos ({{ filtered.repo }}) <button :class="btn" class="!py-0">Watch</button>
      <span class="mx-2">·</span> {{ unbound.repo }} has no local clone <button :class="btn" class="!py-0">Bind</button><button :class="btn" class="!py-0">Clone</button>
    </div>
    <div class="min-h-0 flex-1 overflow-auto pb-16">
      <div class="sticky top-0 z-10 grid grid-cols-[1fr_140px_110px_28px_130px_50px_50px_200px] gap-2 border-b bg-background px-4 py-1 text-[11px] text-muted-foreground">
        <span>Title</span><span>Repo</span><span>Author</span><span>CI</span><span>Review</span><span>Size</span><span>Age</span><span>Since your review</span>
      </div>
      <section v-for="(rows, name) in inbox" :key="name">
        <h2 class="bg-muted/40 px-4 py-1 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">{{ name }} <span class="font-normal">{{ rows.length }}</span></h2>
        <div v-if="!rows.length" class="px-4 py-2 text-xs text-muted-foreground">Nothing pinned. Press <kbd>p</kbd> on a PR to pin it.</div>
        <div v-for="(p, i) in rows" :key="p.n" class="grid grid-cols-[1fr_140px_110px_28px_130px_50px_50px_200px] items-center gap-2 border-b px-4 py-1.5"
          :class="name === 'Review requested' && i === 0 ? 'bg-accent ring-1 ring-inset ring-ring' : 'hover:bg-accent/50'">
          <span class="flex min-w-0 items-center gap-2"><span class="truncate" :class="p.draft ? 'text-muted-foreground' : 'font-medium'">{{ p.title }}</span><span class="text-muted-foreground">#{{ p.n }}</span><InboxBits :p="p" part="trust" /></span>
          <span class="truncate text-muted-foreground">{{ p.repo.split('/')[1] }}</span>
          <span class="truncate text-muted-foreground">{{ p.author }}</span>
          <InboxBits :p="p" part="ci" />
          <span class="text-xs text-muted-foreground">{{ p.draft ? 'Draft' : reviewLabel[p.review] }}</span>
          <span class="font-mono text-xs">{{ p.size }}</span>
          <span class="text-xs text-muted-foreground">{{ p.age }}</span>
          <InboxBits :p="p" part="updated" />
        </div>
      </section>
    </div>
    <footer class="border-t px-4 py-1 text-[11px] text-muted-foreground"><kbd>j</kbd>/<kbd>k</kbd> move · <kbd>enter</kbd> open · <kbd>p</kbd> pin · <kbd>/</kbd> filter</footer>
  </div>
</template>
