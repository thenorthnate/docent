<script setup lang="ts">
// PROTOTYPE: stand-in for the CodeMirror 6 merge view (unified mode). Layout only; the
// real viewer is chosen (docs/spikes/viewer-choice.md). Supports what M0 proved CodeMirror
// can do: gutter markers, a hover `?` at the far right, moved-block headers, block widgets.
import { ref } from 'vue'
import { MessageSquare } from '@lucide/vue'
import { diff, findings, threads, type Finding, type Thread } from '../data/mock'
import { sevDot } from './bits'

const props = withDefaults(defineProps<{
  file?: string
  inline?: boolean          // render findings/threads as block widgets under the line
  focusLine?: number
  showMarkers?: boolean
  findingsOn?: boolean     // false when the PR is untrusted: no lens ran
}>(), { file: 'internal/webhook/deliver.go', inline: false, showMarkers: true, findingsOn: true })
const emit = defineEmits<{ explain: [line: number]; pick: [id: string] }>()
defineSlots<{ widget(p: { line: number; findings: Finding[]; threads: Thread[] }): any }>()

const hover = ref<number | null>(null)
const fAt = (n?: number) => (n && props.findingsOn ? findings.filter(f => f.file === props.file && f.line === n) : [])
const tAt = (n?: number) => (n ? threads.filter(t => t.file === props.file && t.line === n) : [])
const bg = { add: 'bg-add', del: 'bg-del', ctx: '', hunk: 'bg-muted/60 text-muted-foreground', moved: '' }
</script>

<template>
  <div class="font-mono text-[12px] leading-[20px]">
    <template v-for="(l, i) in diff" :key="i">
      <div v-if="l.kind === 'moved'" class="flex items-center gap-2 border-y border-dashed border-violet-500/40 bg-violet-500/10 px-3 py-1 text-violet-400">
        <span>▸</span><span>{{ l.text }}</span><span class="ml-auto text-[11px] underline">jump</span>
      </div>
      <div v-else class="group relative flex" :class="[bg[l.kind], focusLine && l.new === focusLine ? 'ring-1 ring-inset ring-amber-400' : '']"
        @mouseenter="hover = i" @mouseleave="hover = null">
        <div v-if="showMarkers" class="flex w-7 shrink-0 items-center justify-center gap-0.5">
          <button v-for="f in fAt(l.new)" :key="f.id" class="size-2 rounded-full" :class="sevDot[f.severity]" @click="emit('pick', f.id)" />
          <MessageSquare v-if="tAt(l.new).length" class="size-3" :class="tAt(l.new).some(t => t.pending) ? 'text-fuchsia-400' : 'text-muted-foreground'" />
        </div>
        <span class="w-10 shrink-0 select-none pr-2 text-right text-muted-foreground/70">{{ l.old ?? '' }}</span>
        <span class="w-10 shrink-0 select-none pr-2 text-right text-muted-foreground/70">{{ l.new ?? '' }}</span>
        <span class="w-4 shrink-0 select-none text-muted-foreground">{{ l.kind === 'add' ? '+' : l.kind === 'del' ? '−' : '' }}</span>
        <pre class="flex-1 whitespace-pre pr-10">{{ l.text }}</pre>
        <button v-if="hover === i && l.kind !== 'hunk' && l.kind !== 'del'" class="absolute right-2 top-0.5 flex size-4 items-center justify-center rounded bg-primary text-[10px] font-bold text-primary-foreground"
          title="Explain this" @click="emit('explain', l.new!)">?</button>
      </div>
      <div v-if="inline && (fAt(l.new).length || tAt(l.new).length)" class="border-y bg-background px-3 py-2 font-sans text-[13px] leading-normal">
        <slot name="widget" :line="(l.new ?? l.old)!" :findings="fAt(l.new)" :threads="tAt(l.new)" />
      </div>
    </template>
  </div>
</template>
