<script setup lang="ts">
// PROTOTYPE: inbox page = mail split (inbox variant B), with its section list moved into the shell drawer.
import { computed, ref } from 'vue'
import { ArrowRight, Eye, FolderGit2, Pin, Sparkles } from '@lucide/vue'
import { filtered, inbox, pr, unbound } from '../data/mock'
import { go } from '../url'
import Shell from './Shell.vue'
import InboxBits from '../components/InboxBits.vue'
import TrustGate from '../components/TrustGate.vue'
import { btn, btnPrimary, reviewLabel } from '../components/bits'

const section = ref('Review requested')
const sel = ref(4821)
const rows = computed(() => inbox[section.value])
const current = computed(() => rows.value.find(p => p.n === sel.value) ?? rows.value[0])
</script>

<template>
  <Shell>
    <template #drawer>
      <div class="px-3 py-2 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">Inbox</div>
      <div class="px-1.5">
        <button v-for="(r, name) in inbox" :key="name" class="flex w-full items-center rounded-md px-2 py-1" :class="section === name ? 'bg-accent' : 'text-muted-foreground hover:bg-accent/50'" @click="section = name; sel = r[0]?.n">
          <Pin v-if="name === 'Pinned'" class="mr-1.5 size-3.5" />{{ name }}<span class="ml-auto text-xs">{{ r.length }}</span>
        </button>
      </div>
      <div class="mt-4 px-3 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">Watched repos</div>
      <div v-for="r in ['payments', 'infra-deploy', 'web-checkout']" :key="r" class="flex items-center gap-1.5 px-3 py-1 text-muted-foreground"><FolderGit2 class="size-3.5" />{{ r }}
        <button v-if="'acme/' + r === unbound.repo" class="ml-auto text-[11px] text-sky-400">Bind · Clone</button></div>
      <div class="mt-auto flex items-center gap-1.5 border-t px-3 py-2 text-[11px] text-muted-foreground"><Eye class="size-3.5" />{{ filtered.count }} hidden in unwatched repos <button class="text-sky-400">Watch</button></div>
    </template>

    <div class="flex min-h-0 flex-1">
      <div class="w-[420px] shrink-0 overflow-auto border-r">
        <div class="border-b px-3 py-2 font-semibold">{{ section }}</div>
        <button v-for="p in rows" :key="p.n" class="block w-full border-b px-3 py-2.5 text-left" :class="current?.n === p.n ? 'bg-accent' : 'hover:bg-accent/40'" @click="sel = p.n" @dblclick="go('review')">
          <div class="flex items-center gap-1.5 text-[11px] text-muted-foreground"><InboxBits :p="p" part="ci" />{{ p.repo }} · {{ p.author }}<span class="ml-auto">{{ p.age }}</span></div>
          <div class="mt-0.5 font-medium" :class="p.draft ? 'text-muted-foreground' : ''">{{ p.title }}</div>
          <div class="mt-1 flex flex-wrap gap-1.5"><InboxBits :p="p" part="updated" /><InboxBits :p="p" part="trust" /><span class="rounded bg-muted px-1.5 text-[11px]">{{ p.size }} · {{ p.lines }} lines</span></div>
        </button>
        <div v-if="!rows.length" class="p-4 text-muted-foreground">Nothing here.</div>
      </div>
      <section v-if="current" class="min-w-0 flex-1 overflow-auto p-6 pb-20">
        <div class="text-xs text-muted-foreground">{{ current.repo }} #{{ current.n }} · {{ current.author }} · {{ reviewLabel[current.review] }}</div>
        <h1 class="mt-1 text-lg font-semibold">{{ current.title }}</h1>
        <div class="mt-3 flex gap-2"><button :class="btnPrimary" @click="go('review')">Open review <ArrowRight class="size-3.5" /></button><button :class="btn">Pin</button><button :class="btn">Open on GitHub</button></div>
        <div v-if="!current.trusted" class="mt-4"><TrustGate /></div>
        <div v-if="current.updated" class="mt-4 rounded-lg border p-3"><div class="font-medium">Since your last review</div>
          <ul class="mt-1 list-disc pl-5 text-muted-foreground"><li>3 commits: fix body leak, add jitter, rename table</li><li>5 comments, 2 replying to you</li></ul></div>
        <div v-if="current.n === pr.number" class="mt-4 rounded-lg border p-3"><div class="flex items-center gap-1.5 font-medium"><Sparkles class="size-4" /> Summary</div><p class="mt-1 text-muted-foreground">{{ pr.summary }}</p>
          <div class="mt-2 text-xs text-muted-foreground">Lenses ran on 9f2c1a: <span class="text-red-400">1 high</span> · <span class="text-amber-400">2 medium</span> · 3 low</div></div>
      </section>
    </div>
  </Shell>
</template>
