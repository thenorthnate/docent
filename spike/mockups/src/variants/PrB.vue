<script setup lang="ts">
// PROTOTYPE variant B, "Inline + popover": the diff gets the full width. Findings and threads
// render inline as block widgets under their line (GitHub-like). Explain opens as a popover
// beside the line instead of a permanent side panel. Lens toolbar sits above the diff;
// activity, verify and the pending review live in an editor-style status bar.
import { ref } from 'vue'
import { ChevronDown, CircleCheck, FolderTree, Lightbulb, Play, Sparkles, Terminal, X } from '@lucide/vue'
import { currentFile, explain, files, lenses, pr, steeringSuggestions } from '../data/mock'
import DiffView from '../components/DiffView.vue'
import TrustGate from '../components/TrustGate.vue'
import { btn, btnPrimary, sevDot, sevText } from '../components/bits'

defineProps<{ trusted: boolean }>()
const tree = ref(true)
const pop = ref<number | null>(46)
</script>

<template>
  <div class="flex h-full flex-col">
    <header class="border-b px-4 pb-2 pt-3">
      <div class="flex items-start gap-3">
        <div>
          <div class="text-xs text-muted-foreground">{{ pr.repo }} · {{ pr.author }} · <span class="font-mono">{{ pr.head }} → {{ pr.base }}</span></div>
          <h1 class="text-base font-semibold">{{ pr.title }} <span class="font-normal text-muted-foreground">#{{ pr.number }}</span></h1>
        </div>
        <div class="ml-auto flex items-center gap-2">
          <span class="flex items-center gap-1 text-xs text-emerald-500"><CircleCheck class="size-3.5" /> CI</span>
          <button :class="btn">Since last review <span class="text-sky-400">+3</span></button>
          <button :class="btn"><Lightbulb class="size-3.5" /> {{ steeringSuggestions }}</button>
        </div>
      </div>
      <p v-if="trusted" class="mt-2 flex gap-1.5 rounded-md bg-muted/60 px-2.5 py-1.5 text-xs text-muted-foreground">
        <Sparkles class="mt-0.5 size-3.5 shrink-0" /> {{ pr.summary }}
      </p>
    </header>

    <!-- lens toolbar -->
    <div class="flex items-center gap-1.5 border-b px-3 py-1.5 text-[11px]" :class="trusted ? '' : 'pointer-events-none opacity-40'">
      <button class="rounded p-1 hover:bg-accent" @click="tree = !tree"><FolderTree class="size-4" /></button>
      <span class="mx-1 h-4 w-px bg-border" />
      <label v-for="l in lenses.filter(l => l.key !== 'chat' && l.key !== 'explain')" :key="l.key" class="flex items-center gap-1 rounded-md border px-2 py-0.5">
        <input type="checkbox" :checked="l.on" class="size-3" /> {{ l.name }}
        <span v-if="l.count" class="rounded bg-muted px-1">{{ l.count }}</span>
        <span v-if="l.state === 'running'" class="size-1.5 animate-pulse rounded-full bg-amber-400" />
      </label>
      <span class="ml-auto text-muted-foreground">Severity ≥ <b>low</b> · Unified · Full PR</span>
    </div>

    <div class="flex min-h-0 flex-1">
      <aside v-if="tree" class="w-52 shrink-0 overflow-auto border-r py-1 text-xs">
        <div v-for="f in files" :key="f.path" class="flex items-center gap-1 px-2 py-0.5" :class="f.path === currentFile ? 'bg-accent' : ''">
          <span class="truncate" :class="f.viewed ? 'text-muted-foreground line-through' : ''">{{ f.path.split('/').pop() }}</span>
          <span v-if="f.findings && trusted" class="ml-auto size-1.5 shrink-0 rounded-full bg-amber-500" />
        </div>
      </aside>

      <main class="relative min-w-0 flex-1 overflow-auto">
        <div v-if="!trusted" class="p-3"><TrustGate /></div>
        <div class="sticky top-0 z-10 border-b bg-background px-3 py-1.5 font-mono text-xs">{{ currentFile }}</div>
        <DiffView :inline="true" :show-markers="false" @explain="n => (pop = n)">
          <template #widget="{ findings, threads }">
            <div class="space-y-2">
              <div v-for="f in trusted ? findings : []" :key="f.id" class="max-w-3xl rounded-md border-l-2 bg-card p-2" :class="f.severity === 'high' ? 'border-red-500' : 'border-sky-500'">
                <div class="flex items-center gap-1.5 text-[11px]"><span class="size-2 rounded-full" :class="sevDot[f.severity]" /><span :class="sevText[f.severity]">{{ f.lens }} · {{ f.severity }}</span>
                  <span v-if="f.steeringRule" class="text-fuchsia-400">· per your rule</span>
                  <span class="ml-auto flex gap-1"><button :class="btn">Promote</button><button :class="btn">Dismiss</button></span></div>
                <div class="font-medium">{{ f.title }}</div>
                <div class="text-muted-foreground">{{ f.rationale }}</div>
              </div>
              <div v-for="t in threads" :key="t.id" class="max-w-3xl rounded-md border p-2" :class="t.pending ? 'border-fuchsia-500/50 bg-fuchsia-500/5' : ''">
                <div class="text-[11px] text-muted-foreground"><b class="text-foreground">{{ t.author }}</b> <span v-if="t.pending" class="text-fuchsia-400">pending</span><span v-if="t.isNew" class="text-sky-400">new</span><span v-if="t.replies"> · {{ t.replies }} replies</span></div>
                <div>{{ t.body }}</div>
              </div>
            </div>
          </template>
        </DiffView>

        <!-- explain popover -->
        <div v-if="pop && trusted" class="absolute right-6 top-48 z-20 w-96 rounded-lg border bg-card p-3 shadow-2xl">
          <div class="flex items-center text-[11px] text-muted-foreground">Explain · {{ explain.target }}<button class="ml-auto" @click="pop = null"><X class="size-3.5" /></button></div>
          <p class="mt-1">{{ explain.what }}</p>
          <p class="mt-1 text-muted-foreground"><b>Why:</b> {{ explain.why }}</p>
          <div class="mt-1.5 flex flex-wrap gap-1"><a v-for="r in explain.related" :key="r.label" class="rounded bg-muted px-1.5 py-0.5 text-[11px] text-sky-400">{{ r.label }}</a></div>
          <input class="mt-2 w-full rounded-md border bg-background px-2 py-1 text-xs" placeholder="Follow up in chat…" />
        </div>
      </main>
    </div>

    <footer class="flex items-center gap-3 border-t bg-card px-3 py-1 text-[11px] text-muted-foreground">
      <span class="flex items-center gap-1"><Terminal class="size-3" /> Agent activity</span>
      <span v-if="trusted" class="text-amber-400">Modernize running…</span>
      <span>3/7 files viewed</span>
      <span class="ml-auto flex items-center gap-1"><Play class="size-3" /> Verify</span>
      <button :class="btnPrimary" class="!py-0.5">1 pending · Submit <ChevronDown class="size-3" /></button>
    </footer>
  </div>
</template>
