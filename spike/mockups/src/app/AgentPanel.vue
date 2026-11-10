<script setup lang="ts">
// PROTOTYPE: right-hand agent panel, the one place findings, Explain, Trace and Chat live.
import { ref } from 'vue'
import { Play } from '@lucide/vue'
import { explain, findings, lenses } from '../data/mock'
import { trace } from '../data/explore'
import TrustGate from '../components/TrustGate.vue'
import { btn, btnPrimary, sevDot, sevText } from '../components/bits'

const props = defineProps<{ mode: 'review' | 'explore' | 'project'; trusted: boolean }>()
const tabs = props.mode === 'review' ? ['explain', 'findings', 'chat'] : props.mode === 'explore' ? ['explain', 'findings', 'trace', 'chat'] : ['trace', 'explain', 'chat']
const tab = ref(props.mode === 'review' ? 'findings' : props.mode === 'explore' ? 'explain' : 'trace')
const picked = ref('f1')
</script>

<template>
  <aside class="flex w-96 shrink-0 flex-col border-l">
    <div v-if="!trusted" class="p-3"><TrustGate /></div>
    <template v-else>
      <div class="flex flex-wrap items-center gap-1.5 border-b p-2">
        <span v-for="l in lenses" :key="l.key" class="inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[11px]" :class="l.on ? '' : 'opacity-40 line-through'">
          <span class="size-1.5 rounded-full" :class="mode !== 'review' ? 'bg-zinc-500' : l.state === 'running' ? 'animate-pulse bg-amber-400' : l.state === 'done' ? 'bg-emerald-500' : 'bg-zinc-500'" />
          {{ l.name }}<span v-if="l.count && mode === 'review'" class="text-muted-foreground">{{ l.count }}</span><span v-if="l.rules" class="text-muted-foreground">· {{ l.rules }} rules</span>
        </span>
      </div>
      <div class="flex border-b text-xs">
        <button v-for="t in tabs" :key="t" class="flex-1 border-b-2 py-1.5 capitalize" :class="tab === t ? 'border-primary' : 'border-transparent text-muted-foreground'" @click="tab = t">{{ t }}</button>
      </div>
      <div class="min-h-0 flex-1 space-y-2 overflow-auto p-3">
        <template v-if="tab === 'findings' && mode === 'explore'">
          <p class="text-muted-foreground">Lenses run on demand in Explore.</p>
          <div class="flex flex-wrap gap-1.5"><button :class="btnPrimary"><Play class="size-3" /> Run on this file</button><button :class="btn">Run on selection</button><button :class="btn">Self-review branch…</button></div>
          <div class="pt-2 text-[11px] text-muted-foreground">Saved findings · 2 <button class="text-sky-400">Copy as markdown</button></div>
        </template>
        <template v-else-if="tab === 'findings'">
          <div class="flex gap-1 text-[11px] text-muted-foreground">Show: <span class="rounded bg-muted px-1">all lenses</span><span class="rounded bg-muted px-1">≥ low</span></div>
          <div v-for="f in findings" :key="f.id" class="cursor-pointer rounded-lg border p-2.5" :class="picked === f.id ? 'border-ring bg-accent/40' : ''" @click="picked = f.id">
            <div class="flex items-center gap-1.5 text-[11px]"><span class="size-2 rounded-full" :class="sevDot[f.severity]" /><span :class="sevText[f.severity]">{{ f.severity }}</span>
              <span class="text-muted-foreground">· {{ f.lens }} · {{ f.unanchored ? 'unanchored' : `${f.file.split('/').pop()}:${f.line}` }}</span></div>
            <div class="mt-0.5 font-medium">{{ f.title }}</div>
            <template v-if="picked === f.id">
              <p class="mt-1 text-muted-foreground">{{ f.rationale }}</p>
              <pre v-if="f.suggestion" class="mt-1.5 rounded bg-muted p-1.5 font-mono text-[11px]">{{ f.suggestion }}</pre>
              <p v-if="f.steeringRule" class="mt-1 text-[11px] text-fuchsia-400">per your rule: {{ f.steeringRule }}</p>
              <div class="mt-2 flex gap-1.5"><button :class="btnPrimary">Promote to comment</button><button :class="btn">Dismiss</button></div>
            </template>
          </div>
        </template>
        <template v-else-if="tab === 'trace'">
          <div class="flex gap-1 text-[11px]"><span class="rounded bg-accent px-1.5 py-0.5">Where does this value come from?</span><span class="rounded px-1.5 py-0.5 text-muted-foreground">How do we get here?</span></div>
          <div class="text-[11px] text-muted-foreground">From <span class="font-mono">deliver.go:28</span> · client timeout</div>
          <ol class="relative ml-1.5 border-l pl-4">
            <li v-for="(s, i) in trace" :key="i" class="relative mb-3">
              <span class="absolute -left-[21px] top-1 size-2.5 rounded-full border-2 border-background" :class="s.inferred ? 'bg-amber-500' : 'bg-sky-500'" />
              <div class="flex items-center gap-1.5 text-[11px]"><span class="rounded bg-muted px-1">{{ s.repo }}</span><a class="font-mono text-sky-400 underline">{{ s.loc }}</a>
                <span v-if="s.inferred" class="rounded bg-amber-500/15 px-1 text-amber-500">inferred</span></div>
              <pre class="mt-1 overflow-hidden rounded bg-muted p-1.5 font-mono text-[11px]">{{ s.code }}</pre>
              <div class="mt-0.5 text-[11px] text-muted-foreground">{{ s.note }}</div>
            </li>
          </ol>
          <p class="rounded-md bg-amber-500/10 p-2 text-[11px] text-amber-500">The deploy overrides the 10s default with 30s. The cross-repo step is matched on the env var name, not proven.</p>
        </template>
        <template v-else-if="tab === 'explain'">
          <div class="text-[11px] text-muted-foreground">Explain · {{ mode === 'review' ? explain.target : 'deliver.go:28' }}</div>
          <h4 class="font-semibold">What</h4><p class="text-muted-foreground">{{ explain.what }}</p>
          <h4 class="font-semibold">Why</h4><p class="text-muted-foreground">{{ explain.why }}</p>
          <h4 class="font-semibold">Related</h4><a v-for="r in explain.related" :key="r.label" class="block text-sky-400 underline">{{ r.label }}</a>
        </template>
        <template v-else>
          <div class="rounded-lg bg-muted p-2">Why not keep the in-process retry as a fast path?</div>
          <div class="p-2 text-muted-foreground">The old loop slept on the request goroutine…</div>
        </template>
      </div>
      <div v-if="tab === 'chat'" class="border-t p-2"><input class="w-full rounded-md border bg-background px-2 py-1.5" placeholder="Ask…" /></div>
    </template>
  </aside>
</template>
