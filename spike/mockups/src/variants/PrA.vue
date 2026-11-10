<script setup lang="ts">
// PROTOTYPE variant A, "Three panes": PLAN.md §5.4 as drawn. File tree | diff | agent panel,
// top bar with PR meta and Submit, activity drawer docked at the bottom.
import { ref } from 'vue'
import { Check, ChevronDown, CircleCheck, FileCode, Inbox, Lightbulb, Play, Sparkles, Terminal } from '@lucide/vue'
import { activity, currentFile, explain, files, findings, lenses, pr, steeringSuggestions } from '../data/mock'
import DiffView from '../components/DiffView.vue'
import TrustGate from '../components/TrustGate.vue'
import { btn, btnPrimary, sevDot, sevText } from '../components/bits'

defineProps<{ trusted: boolean }>()
const tab = ref<'explain' | 'findings' | 'chat'>('findings')
const view = ref<'overview' | 'files'>('files')
const drawer = ref(false)
const picked = ref('f1')
</script>

<template>
  <div class="flex h-full flex-col">
    <!-- top bar -->
    <header class="flex items-center gap-3 border-b px-3 py-2">
      <Inbox class="size-4 text-muted-foreground" />
      <span class="text-muted-foreground">{{ pr.repo }}</span>
      <span class="font-semibold">#{{ pr.number }} {{ pr.title }}</span>
      <span class="rounded bg-muted px-1.5 py-0.5 font-mono text-[11px]">{{ pr.head }} → {{ pr.base }}</span>
      <CircleCheck class="size-4 text-emerald-500" />
      <span class="rounded-full bg-sky-500/15 px-2 py-0.5 text-[11px] text-sky-400">{{ pr.newSinceReview.commits }} new commits since your review</span>
      <div class="ml-auto flex items-center gap-2">
        <button :class="btn"><Lightbulb class="size-3.5" /> Steering <span class="rounded-full bg-fuchsia-500 px-1.5 text-[10px] text-white">{{ steeringSuggestions }}</span></button>
        <button :class="btnPrimary">Submit review (1) <ChevronDown class="size-3.5" /></button>
      </div>
    </header>
    <div class="flex items-center gap-1 border-b px-3 text-xs">
      <button v-for="v in (['overview', 'files'] as const)" :key="v" class="border-b-2 px-2 py-1.5 capitalize"
        :class="view === v ? 'border-primary' : 'border-transparent text-muted-foreground'" @click="view = v">{{ v === 'files' ? 'Files changed' : 'Overview' }}</button>
      <div class="ml-auto flex items-center gap-2 text-muted-foreground">
        <select class="rounded border bg-background px-1 py-0.5"><option>Full PR</option><option>Since last review</option></select>
        <select class="rounded border bg-background px-1 py-0.5"><option>Unified</option><option>Split</option></select>
        <label class="flex items-center gap-1"><input type="checkbox" /> Ignore whitespace</label>
      </div>
    </div>

    <div class="flex min-h-0 flex-1">
      <!-- file tree -->
      <aside class="w-64 shrink-0 overflow-auto border-r p-1">
        <div v-for="f in files" :key="f.path" class="flex items-center gap-1.5 rounded px-1.5 py-1"
          :class="f.path === currentFile ? 'bg-accent' : 'hover:bg-accent/50'">
          <Check v-if="f.viewed" class="size-3 shrink-0 text-emerald-500" /><FileCode v-else class="size-3 shrink-0 text-muted-foreground" />
          <span class="truncate" :class="f.viewed ? 'text-muted-foreground' : ''">{{ f.path.split('/').pop() }}</span>
          <span class="ml-auto flex shrink-0 items-center gap-1 text-[10px]">
            <span v-if="f.findings && trusted" class="rounded bg-amber-500/20 px-1 text-amber-500">{{ f.findings }}</span>
            <span v-if="f.comments" class="rounded bg-fuchsia-500/20 px-1 text-fuchsia-400">{{ f.comments }}</span>
            <span class="text-emerald-500">+{{ f.add }}</span><span class="text-red-500">−{{ f.del }}</span>
          </span>
        </div>
      </aside>

      <!-- center -->
      <main class="min-w-0 flex-1 overflow-auto">
        <template v-if="view === 'files'">
          <div class="sticky top-0 z-10 flex items-center gap-2 border-b bg-background px-3 py-1.5 font-mono text-xs">
            {{ currentFile }} <label class="ml-auto flex items-center gap-1 font-sans text-muted-foreground"><input type="checkbox" /> Viewed</label>
          </div>
          <DiffView :findings-on="trusted" :show-markers="true" @pick="id => { picked = id; tab = 'findings' }" @explain="tab = 'explain'" />
        </template>
        <div v-else class="mx-auto max-w-2xl space-y-4 p-6">
          <section class="rounded-lg border p-4"><h3 class="mb-1 flex items-center gap-1.5 font-semibold"><Sparkles class="size-4" /> AI summary</h3><p class="text-muted-foreground">{{ trusted ? pr.summary : 'Not generated: this PR is not trusted.' }}</p></section>
          <section class="rounded-lg border p-4"><h3 class="mb-1 font-semibold">Description</h3><p class="whitespace-pre-line text-muted-foreground">{{ pr.body }}</p></section>
          <section class="rounded-lg border p-4"><h3 class="mb-1 font-semibold">Checks</h3><p class="text-emerald-500">7/7 passing</p></section>
        </div>
      </main>

      <!-- agent panel -->
      <aside class="flex w-96 shrink-0 flex-col border-l">
        <template v-if="trusted">
          <div class="flex flex-wrap gap-1.5 border-b p-2">
            <span v-for="l in lenses" :key="l.key" class="inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[11px]"
              :class="l.on ? '' : 'opacity-40 line-through'">
              <span class="size-1.5 rounded-full" :class="l.state === 'running' ? 'animate-pulse bg-amber-400' : l.state === 'done' ? 'bg-emerald-500' : 'bg-zinc-500'" />
              {{ l.name }}<span v-if="l.count" class="text-muted-foreground">{{ l.count }}</span><span v-if="l.rules" class="text-muted-foreground">· {{ l.rules }} rules</span>
            </span>
          </div>
          <div class="flex border-b text-xs">
            <button v-for="t in (['explain', 'findings', 'chat'] as const)" :key="t" class="flex-1 border-b-2 py-1.5 capitalize"
              :class="tab === t ? 'border-primary' : 'border-transparent text-muted-foreground'" @click="tab = t">{{ t }}</button>
          </div>
          <div class="min-h-0 flex-1 space-y-2 overflow-auto p-3">
            <template v-if="tab === 'findings'">
              <div class="flex gap-1 text-[11px] text-muted-foreground">Show: <span class="rounded bg-muted px-1">all lenses</span><span class="rounded bg-muted px-1">≥ low</span></div>
              <div v-for="f in findings" :key="f.id" class="rounded-lg border p-2.5" :class="picked === f.id ? 'border-ring bg-accent/40' : ''" @click="picked = f.id">
                <div class="flex items-center gap-1.5 text-[11px]">
                  <span class="size-2 rounded-full" :class="sevDot[f.severity]" /><span :class="sevText[f.severity]">{{ f.severity }}</span>
                  <span class="text-muted-foreground">· {{ f.lens }} · {{ f.unanchored ? 'unanchored' : `${f.file.split('/').pop()}:${f.line}` }}</span>
                </div>
                <div class="mt-0.5 font-medium">{{ f.title }}</div>
                <template v-if="picked === f.id">
                  <p class="mt-1 text-muted-foreground">{{ f.rationale }}</p>
                  <pre v-if="f.suggestion" class="mt-1.5 rounded bg-muted p-1.5 font-mono text-[11px]">{{ f.suggestion }}</pre>
                  <p v-if="f.steeringRule" class="mt-1 text-[11px] text-fuchsia-400">per your rule: {{ f.steeringRule }}</p>
                  <div class="mt-2 flex gap-1.5"><button :class="btnPrimary">Promote to comment</button><button :class="btn">Dismiss</button></div>
                </template>
              </div>
            </template>
            <template v-else-if="tab === 'explain'">
              <div class="text-[11px] text-muted-foreground">Explain · {{ explain.target }}</div>
              <h4 class="font-semibold">What</h4><p class="text-muted-foreground">{{ explain.what }}</p>
              <h4 class="font-semibold">Why</h4><p class="text-muted-foreground">{{ explain.why }}</p>
              <h4 class="font-semibold">Related</h4>
              <a v-for="r in explain.related" :key="r.label" class="block text-sky-400 underline">{{ r.label }}</a>
            </template>
            <template v-else>
              <div class="rounded-lg bg-muted p-2">Why not keep the in-process retry as a fast path?</div>
              <div class="p-2 text-muted-foreground">The old loop slept on the request goroutine…</div>
            </template>
          </div>
          <div v-if="tab === 'chat'" class="border-t p-2"><input class="w-full rounded-md border bg-background px-2 py-1.5" placeholder="Ask about this PR…" /></div>
        </template>
        <div v-else class="p-3"><TrustGate /></div>
      </aside>
    </div>

    <!-- activity drawer -->
    <footer class="border-t">
      <button class="flex w-full items-center gap-2 px-3 py-1 text-[11px] text-muted-foreground" @click="drawer = !drawer">
        <Terminal class="size-3.5" /> Agent activity <span v-if="trusted" class="text-amber-400">· Modernize running</span>
        <span class="ml-auto flex items-center gap-1"><Play class="size-3" /> Verify</span>
      </button>
      <div v-if="drawer" class="h-32 overflow-auto border-t bg-card px-3 py-1 font-mono text-[11px]">
        <div v-for="a in activity" :key="a.t" :class="a.kind === 'deny' ? 'text-red-400' : ''">{{ a.t }} [{{ a.who }}] {{ a.text }}</div>
      </div>
    </footer>
  </div>
</template>
