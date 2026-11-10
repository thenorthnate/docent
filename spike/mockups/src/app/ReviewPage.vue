<script setup lang="ts">
// PROTOTYPE: PR review page = PR variant A (findings only in the agent panel), with the file tree
// in the shell drawer and a breadcrumb back to the inbox.
import { ref } from 'vue'
import { Check, ChevronDown, ChevronRight, CircleCheck, FileCode, Play, Telescope, Terminal } from '@lucide/vue'
import { activity, currentFile, files, pr } from '../data/mock'
import { go } from '../url'
import { exploreBranch, review } from './store'
import Shell from './Shell.vue'
import AgentPanel from './AgentPanel.vue'
import DiffView from '../components/DiffView.vue'
import { btn, btnPrimary } from '../components/bits'

defineProps<{ trusted: boolean }>()
const drawer = ref(false)
</script>

<template>
  <Shell>
    <template #drawer>
      <div class="px-3 py-2 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">Open reviews</div>
      <div class="px-1.5 text-xs">
        <div class="truncate rounded-md bg-accent px-2 py-1">#4821 retry failed invoice webhooks</div>
        <div class="truncate rounded-md px-2 py-1 text-muted-foreground hover:bg-accent/50">#4830 clamp page size to 200</div>
      </div>
      <div class="mt-3 flex items-center px-3 py-1 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">Files changed <span class="ml-auto font-normal normal-case">3/7 viewed</span></div>
      <div class="min-h-0 flex-1 overflow-auto px-1">
        <div v-for="f in files" :key="f.path" class="flex items-center gap-1.5 rounded px-1.5 py-1" :class="f.path === currentFile ? 'bg-accent' : 'hover:bg-accent/50'">
          <Check v-if="f.viewed" class="size-3 shrink-0 text-emerald-500" /><FileCode v-else class="size-3 shrink-0 text-muted-foreground" />
          <span class="truncate" :class="f.viewed ? 'text-muted-foreground' : ''">{{ f.path.split('/').pop() }}</span>
          <span class="ml-auto flex shrink-0 items-center gap-1 text-[10px]">
            <span v-if="f.findings && trusted" class="rounded bg-amber-500/20 px-1 text-amber-500">{{ f.findings }}</span>
            <span v-if="f.comments" class="rounded bg-fuchsia-500/20 px-1 text-fuchsia-400">{{ f.comments }}</span>
          </span>
        </div>
      </div>
    </template>

    <header class="flex items-center gap-2 border-b px-3 py-2">
      <button class="text-muted-foreground hover:text-foreground" @click="go('inbox')">Inbox</button><ChevronRight class="size-3.5 text-muted-foreground" />
      <span class="text-muted-foreground">{{ pr.repo }}</span><ChevronRight class="size-3.5 text-muted-foreground" />
      <span class="truncate font-semibold">#{{ pr.number }} {{ pr.title }}</span>
      <CircleCheck class="size-4 shrink-0 text-emerald-500" />
      <span class="shrink-0 rounded-full bg-sky-500/15 px-2 py-0.5 text-[11px] text-sky-400">{{ pr.newSinceReview.commits }} new commits</span>
      <div class="ml-auto flex shrink-0 items-center gap-2">
        <button :class="btn" title="Browse the whole repo at this PR's head, with its changes marked" @click="exploreBranch(); go('explore')"><Telescope class="size-3.5" /> Explore this branch</button>
        <button :class="btnPrimary">Submit review ({{ review.pending.length }}) <ChevronDown class="size-3.5" /></button>
      </div>
    </header>
    <div class="flex items-center gap-1 border-b px-3 text-xs">
      <span class="border-b-2 border-transparent px-2 py-1.5 text-muted-foreground">Overview</span><span class="border-b-2 border-primary px-2 py-1.5">Files changed</span>
      <span class="ml-3 font-mono text-[11px] text-muted-foreground">{{ pr.head }} → {{ pr.base }}</span>
      <div class="ml-auto flex items-center gap-2 text-muted-foreground">
        <select class="rounded border bg-background px-1 py-0.5"><option>Full PR</option><option>Since last review</option></select>
        <select class="rounded border bg-background px-1 py-0.5"><option>Unified</option><option>Split</option></select>
      </div>
    </div>
    <div class="flex min-h-0 flex-1">
      <main class="min-w-0 flex-1 overflow-auto">
        <div class="sticky top-0 z-10 flex items-center gap-2 border-b bg-background px-3 py-1.5 font-mono text-xs">{{ currentFile }}<label class="ml-auto flex items-center gap-1 font-sans text-muted-foreground"><input type="checkbox" /> Viewed</label></div>
        <DiffView :findings-on="trusted" :lens-on="review.enabled" :draft-line="review.draftLine" :pending-lines="review.pending.map(p => p.line)" @comment="n => (review.draftLine = n)" />
      </main>
      <AgentPanel mode="review" :trusted="trusted" />
    </div>
    <footer class="border-t">
      <button class="flex w-full items-center gap-2 px-3 py-1 text-[11px] text-muted-foreground" @click="drawer = !drawer">
        <Terminal class="size-3.5" /> Agent activity <span v-if="trusted" class="text-amber-400">· Modernize running</span><span class="ml-auto flex items-center gap-1"><Play class="size-3" /> Verify</span>
      </button>
      <div v-if="drawer" class="h-32 overflow-auto border-t bg-card px-3 py-1 font-mono text-[11px]"><div v-for="a in activity" :key="a.t" :class="a.kind === 'deny' ? 'text-red-400' : ''">{{ a.t }} [{{ a.who }}] {{ a.text }}</div></div>
    </footer>
  </Shell>
</template>
