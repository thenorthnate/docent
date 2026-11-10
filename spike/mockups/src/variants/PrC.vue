<script setup lang="ts">
// PROTOTYPE variant C, "Triage queue": the left column is not a file tree but a queue of
// everything that needs you (findings, new comments, unviewed files), worked through with
// j/k. The diff follows the queue. Explain and Chat share a console under the diff.
// The file tree moves to a ⌘P palette.
import { computed, ref } from 'vue'
import { Check, ChevronDown, FileCode, MessageSquare, Search, Sparkles } from '@lucide/vue'
import { explain, files, findings, pr, threads } from '../data/mock'
import DiffView from '../components/DiffView.vue'
import TrustGate from '../components/TrustGate.vue'
import { btn, btnPrimary, sevDot, sevText } from '../components/bits'

const props = defineProps<{ trusted: boolean }>()
type Item = { id: string; kind: 'finding' | 'thread' | 'file'; file: string; line?: number; title: string; sub: string; sev?: keyof typeof sevDot }
const items = computed<Item[]>(() => [
  ...(props.trusted ? findings.map(f => ({ id: f.id, kind: 'finding' as const, file: f.file, line: f.line, title: f.title, sub: `${f.lens} · ${f.severity}`, sev: f.severity })) : []),
  ...threads.filter(t => t.isNew).map(t => ({ id: t.id, kind: 'thread' as const, file: t.file, line: t.line, title: t.body, sub: `${t.author} · new comment` })),
  ...files.filter(f => !f.viewed).map(f => ({ id: f.path, kind: 'file' as const, file: f.path, title: f.path.split('/').pop()!, sub: `unviewed · +${f.add} −${f.del}` })),
])
const groups = computed(() => {
  const g: Record<string, Item[]> = {}
  for (const i of items.value) (g[i.file] ??= []).push(i)
  return g
})
const cur = ref('f1')
const current = computed(() => items.value.find(i => i.id === cur.value) ?? items.value[0])
const consoleTab = ref<'explain' | 'chat'>('explain')
</script>

<template>
  <div class="flex h-full flex-col">
    <header class="flex items-center gap-3 border-b px-3 py-2">
      <span class="font-semibold">#{{ pr.number }}</span><span class="truncate">{{ pr.title }}</span>
      <span class="text-xs text-muted-foreground">{{ pr.repo }}</span>
      <div class="ml-auto flex items-center gap-3">
        <div class="flex items-center gap-2 text-xs text-muted-foreground">
          <div class="h-1.5 w-32 overflow-hidden rounded-full bg-muted"><div class="h-full w-1/3 bg-emerald-500" /></div> 4 of {{ items.length + 4 }} handled
        </div>
        <button :class="btn"><Search class="size-3.5" /> Files <kbd class="text-[10px] text-muted-foreground">⌘P</kbd></button>
        <button :class="btnPrimary">Submit (1) <ChevronDown class="size-3.5" /></button>
      </div>
    </header>

    <div class="flex min-h-0 flex-1">
      <!-- queue -->
      <aside class="flex w-80 shrink-0 flex-col border-r">
        <div class="space-y-2 border-b p-3 text-xs">
          <p v-if="trusted" class="flex gap-1.5 text-muted-foreground"><Sparkles class="mt-0.5 size-3.5 shrink-0" />{{ pr.summary }}</p>
          <TrustGate v-else compact />
        </div>
        <div class="flex items-center justify-between px-3 py-1.5 text-[11px] text-muted-foreground">
          <span>To review · grouped by file</span><span><kbd>j</kbd>/<kbd>k</kbd> move · <kbd>e</kbd> done</span>
        </div>
        <div class="min-h-0 flex-1 overflow-auto px-1.5 pb-2">
          <div v-for="(list, file) in groups" :key="file" class="mb-1">
            <div class="truncate px-1.5 py-1 font-mono text-[11px] text-muted-foreground">{{ file }}</div>
            <button v-for="i in list" :key="i.id" class="flex w-full items-start gap-2 rounded-md px-2 py-1.5 text-left"
              :class="current?.id === i.id ? 'bg-accent ring-1 ring-ring' : 'hover:bg-accent/50'" @click="cur = i.id">
              <span v-if="i.kind === 'finding'" class="mt-1.5 size-2 shrink-0 rounded-full" :class="sevDot[i.sev!]" />
              <MessageSquare v-else-if="i.kind === 'thread'" class="mt-0.5 size-3.5 shrink-0 text-sky-400" />
              <FileCode v-else class="mt-0.5 size-3.5 shrink-0 text-muted-foreground" />
              <span class="min-w-0"><span class="line-clamp-2">{{ i.title }}</span><span class="text-[11px] text-muted-foreground">{{ i.sub }}{{ i.line ? ` · L${i.line}` : '' }}</span></span>
            </button>
          </div>
          <div class="flex items-center gap-1.5 px-2 py-2 text-[11px] text-muted-foreground"><Check class="size-3 text-emerald-500" /> 4 handled (hidden)</div>
        </div>
      </aside>

      <!-- focus -->
      <main class="flex min-w-0 flex-1 flex-col">
        <div v-if="current?.kind === 'finding'" class="border-b bg-card px-4 py-2.5">
          <div class="flex items-center gap-2 text-[11px]"><span class="size-2 rounded-full" :class="sevDot[current.sev!]" /><span :class="sevText[current.sev!]">{{ current.sub }}</span>
            <span class="ml-auto flex gap-1.5"><button :class="btnPrimary">Promote <kbd>c</kbd></button><button :class="btn">Dismiss <kbd>x</kbd></button><button :class="btn">Ask <kbd>?</kbd></button></span></div>
          <div class="mt-0.5 font-medium">{{ current.title }}</div>
          <div class="text-muted-foreground">{{ findings.find(f => f.id === current!.id)?.rationale }}</div>
        </div>
        <div class="sticky top-0 border-b px-3 py-1.5 font-mono text-xs">{{ current?.file }}</div>
        <div class="min-h-0 flex-1 overflow-auto"><DiffView :findings-on="trusted" :focus-line="current?.line" :show-markers="true" @pick="id => (cur = id)" /></div>

        <!-- console -->
        <section class="h-56 shrink-0 border-t">
          <div class="flex border-b text-xs">
            <button v-for="t in (['explain', 'chat'] as const)" :key="t" class="border-b-2 px-3 py-1.5 capitalize" :class="consoleTab === t ? 'border-primary' : 'border-transparent text-muted-foreground'" @click="consoleTab = t">{{ t }}</button>
            <span class="ml-auto self-center pr-3 text-[11px] text-muted-foreground">Activity · Verify</span>
          </div>
          <div v-if="trusted" class="grid h-[calc(100%-29px)] grid-cols-2 gap-4 overflow-auto p-3">
            <div><div class="text-[11px] text-muted-foreground">{{ explain.target }}</div><p>{{ explain.what }}</p><p class="mt-1 text-muted-foreground">{{ explain.why }}</p></div>
            <div><div class="text-[11px] text-muted-foreground">Related</div><a v-for="r in explain.related" :key="r.label" class="block text-sky-400 underline">{{ r.label }}</a>
              <input class="mt-3 w-full rounded-md border bg-background px-2 py-1 text-xs" placeholder="Ask a follow-up…" /></div>
          </div>
          <p v-else class="p-3 text-muted-foreground">Agents are off until you trust this PR.</p>
        </section>
      </main>
    </div>
  </div>
</template>
