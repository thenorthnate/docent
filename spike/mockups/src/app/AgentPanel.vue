<script setup lang="ts">
// PROTOTYPE: right-hand agent panel, the one place findings, your comments, Explain, Trace and Chat live.
// Review mode groups findings by lens; each lens has an on/off switch and starts collapsed.
import { ref, watch } from 'vue'
import { ChevronDown, ChevronRight, Loader, MessageSquare, Play, Sparkles, X } from '@lucide/vue'
import { explain, findings, lenses } from '../data/mock'
import { trace } from '../data/explore'
import { review } from './store'
import TrustGate from '../components/TrustGate.vue'
import { btn, btnPrimary, sevDot, sevText } from '../components/bits'

const props = defineProps<{ mode: 'review' | 'explore' | 'project'; trusted: boolean }>()
const tabs = props.mode === 'review' ? ['findings', 'explain', 'chat'] : props.mode === 'explore' ? ['explain', 'findings', 'trace', 'chat'] : ['trace', 'explain', 'chat']
const tab = ref(tabs[0])
const picked = ref('f1')
const groups = lenses.filter(l => !['explain', 'chat'].includes(l.key))
const byLens = (name: string) => findings.filter(f => f.lens === name.split(' ')[0])

// new comment (opened by clicking a line number in the diff)
const kind = ref('General')
const body = ref('')
const drafted = ref(false)
watch(() => review.draftLine, l => { if (l) { tab.value = 'findings'; body.value = ''; drafted.value = false } })
function add() {
  review.pending.push({ id: `p${Date.now()}`, line: review.draftLine!, kind: kind.value, body: body.value || '(empty)' })
  review.draftLine = null
}
</script>

<template>
  <aside class="flex w-96 shrink-0 flex-col border-l">
    <div v-if="!trusted && mode !== 'review'" class="p-3"><TrustGate /></div>
    <template v-else>
      <div class="flex border-b text-xs">
        <button v-for="t in tabs" :key="t" class="flex-1 border-b-2 py-1.5 capitalize" :class="tab === t ? 'border-primary' : 'border-transparent text-muted-foreground'" @click="tab = t">{{ t }}</button>
      </div>
      <div class="min-h-0 flex-1 space-y-2 overflow-auto p-3">
        <!-- REVIEW: your comments + findings grouped by lens -->
        <template v-if="tab === 'findings' && mode === 'review'">
          <div v-if="review.draftLine" class="rounded-lg border border-fuchsia-500/60 bg-fuchsia-500/5 p-2.5">
            <div class="flex items-center text-[11px] text-muted-foreground">New comment · <span class="ml-1 font-mono">deliver.go:{{ review.draftLine }}</span><button class="ml-auto" @click="review.draftLine = null"><X class="size-3.5" /></button></div>
            <div class="mt-2 flex items-center gap-2 text-xs">
              <span class="text-muted-foreground">Kind</span>
              <select v-model="kind" class="rounded border bg-background px-1.5 py-0.5"><option>General</option><option v-for="g in groups" :key="g.key">{{ g.name.split(' ')[0] }}</option></select>
              <span class="text-[11px] text-muted-foreground">used to file a steering rule</span>
            </div>
            <div class="mt-2 flex gap-2 text-[11px]"><span class="border-b border-primary">Write</span><span class="text-muted-foreground">Preview</span><button class="ml-auto text-muted-foreground hover:text-foreground">± Suggestion</button></div>
            <textarea v-model="body" rows="4" class="mt-1 w-full rounded-md border bg-background p-2 text-[13px]" placeholder="Leave a comment (markdown)…" />
            <div v-if="drafted" class="mt-1 rounded-md border bg-card p-2 text-[13px]">
              <div class="flex items-center gap-1 text-[11px] text-muted-foreground"><Sparkles class="size-3" /> Agent draft · checked against the code at 9f2c1a</div>
              <p class="mt-1">When the POST fails or returns 5xx we return before <code>defer resp.Body.Close()</code>, so the body leaks on every retry. Could we close it on this path too?</p>
              <div class="mt-1.5 flex gap-1.5"><button :class="btn" @click="body = 'When the POST fails or returns 5xx we return before defer resp.Body.Close(), so the body leaks on every retry. Could we close it on this path too?'; drafted = false">Use this</button><button :class="btn" @click="drafted = false">Keep mine</button></div>
            </div>
            <div class="mt-2 flex flex-wrap gap-1.5">
              <button :class="btnPrimary" @click="add">Add to review</button>
              <button :class="btn">Comment now</button>
              <button :class="btn" class="ml-auto" @click="drafted = true"><Sparkles class="size-3.5" /> Draft with agent</button>
            </div>
          </div>

          <section class="rounded-lg border">
            <div class="flex items-center gap-2 px-2.5 py-2"><MessageSquare class="size-3.5 text-fuchsia-400" /><span class="font-medium">Your review</span><span class="text-[11px] text-muted-foreground">{{ review.pending.length }} pending</span>
              <span class="ml-auto text-[11px] text-muted-foreground">click a line number to add</span></div>
            <div v-for="c in review.pending" :key="c.id" class="border-t px-2.5 py-2">
              <div class="text-[11px] text-muted-foreground"><span class="font-mono">deliver.go:{{ c.line }}</span> · {{ c.kind }}</div><div>{{ c.body }}</div>
            </div>
          </section>

          <TrustGate v-if="!trusted" compact />
          <template v-else>
            <div class="flex items-center gap-1 pt-1 text-[11px] text-muted-foreground">Lenses · severity ≥ <span class="rounded bg-muted px-1">low</span></div>
            <section v-for="g in groups" :key="g.key" class="rounded-lg border" :class="review.enabled[g.name] ? '' : 'opacity-60'">
              <div class="flex items-center gap-1.5 px-2.5 py-2">
                <button class="flex min-w-0 flex-1 items-center gap-1.5 text-left" :disabled="!review.enabled[g.name]" @click="review.expanded[g.name] = !review.expanded[g.name]">
                  <component :is="review.enabled[g.name] && review.expanded[g.name] ? ChevronDown : ChevronRight" class="size-3.5 shrink-0" :class="review.enabled[g.name] ? '' : 'invisible'" />
                  <span class="font-medium">{{ g.name }}</span>
                  <span v-if="g.rules" class="text-[11px] text-muted-foreground">· {{ g.rules }} rules</span>
                  <Loader v-if="g.state === 'running' && review.enabled[g.name]" class="size-3 animate-spin text-amber-400" />
                  <span class="ml-auto text-[11px] text-muted-foreground">
                    <template v-if="!review.enabled[g.name]">off</template>
                    <template v-else-if="!review.expanded[g.name]">{{ byLens(g.name).length }} hidden</template>
                    <template v-else>{{ byLens(g.name).length }}</template>
                  </span>
                </button>
                <button role="switch" :aria-checked="review.enabled[g.name]" class="relative h-4 w-7 shrink-0 rounded-full transition" :class="review.enabled[g.name] ? 'bg-emerald-500' : 'bg-muted'"
                  @click="review.enabled[g.name] = !review.enabled[g.name]">
                  <span class="absolute top-0.5 size-3 rounded-full bg-white transition" :class="review.enabled[g.name] ? 'left-3.5' : 'left-0.5'" />
                </button>
              </div>
              <div v-if="!review.enabled[g.name]" class="border-t px-2.5 py-1.5 text-[11px] text-muted-foreground">Won't run on this PR. <button class="text-sky-400">Run anyway</button> · <button class="text-sky-400">Apply to repo</button></div>
              <div v-else-if="review.expanded[g.name]" class="space-y-1.5 border-t p-1.5">
                <div v-if="!byLens(g.name).length" class="px-1 py-1 text-[11px] text-muted-foreground">No findings.</div>
                <div v-for="f in byLens(g.name)" :key="f.id" class="cursor-pointer rounded-md border p-2" :class="picked === f.id ? 'border-ring bg-accent/40' : 'border-transparent hover:bg-accent/30'" @click="picked = f.id">
                  <div class="flex items-center gap-1.5 text-[11px]"><span class="size-2 rounded-full" :class="sevDot[f.severity]" /><span :class="sevText[f.severity]">{{ f.severity }}</span>
                    <span class="text-muted-foreground">· {{ f.unanchored ? 'unanchored' : `${f.file.split('/').pop()}:${f.line}` }}</span></div>
                  <div class="mt-0.5 font-medium">{{ f.title }}</div>
                  <template v-if="picked === f.id">
                    <p class="mt-1 text-muted-foreground">{{ f.rationale }}</p>
                    <pre v-if="f.suggestion" class="mt-1.5 rounded bg-muted p-1.5 font-mono text-[11px]">{{ f.suggestion }}</pre>
                    <p v-if="f.steeringRule" class="mt-1 text-[11px] text-fuchsia-400">per your rule: {{ f.steeringRule }}</p>
                    <div class="mt-2 flex gap-1.5"><button :class="btnPrimary">Promote to comment</button><button :class="btn">Dismiss</button></div>
                  </template>
                </div>
              </div>
            </section>
          </template>
        </template>

        <template v-else-if="tab === 'findings'">
          <p class="text-muted-foreground">Lenses run on demand in Explore.</p>
          <div class="flex flex-wrap gap-1.5"><button :class="btnPrimary"><Play class="size-3" /> Run on this file</button><button :class="btn">Run on selection</button><button :class="btn">Self-review branch…</button></div>
          <div class="pt-2 text-[11px] text-muted-foreground">Saved findings · 2 <button class="text-sky-400">Copy as markdown</button></div>
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
          <TrustGate v-if="!trusted" compact />
          <template v-else>
            <div class="text-[11px] text-muted-foreground">Explain · {{ mode === 'review' ? explain.target : 'deliver.go:28' }}</div>
            <h4 class="font-semibold">What</h4><p class="text-muted-foreground">{{ explain.what }}</p>
            <h4 class="font-semibold">Why</h4><p class="text-muted-foreground">{{ explain.why }}</p>
            <h4 class="font-semibold">Related</h4><a v-for="r in explain.related" :key="r.label" class="block text-sky-400 underline">{{ r.label }}</a>
          </template>
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
