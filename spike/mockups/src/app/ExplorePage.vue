<script setup lang="ts">
// PROTOTYPE: Explore (PLAN.md §5.9, M6). Any bound repo at any ref, full file tree (not just
// changed files), read-only code with the same `?` and agent panel. Lenses are on demand.
import { ChevronDown, GitBranch, GitPullRequest, Search, X } from '@lucide/vue'
import { blame, deliverGo, paymentsTree } from '../data/explore'
import { go } from '../url'
import { explore } from './store'
import Shell from './Shell.vue'
import Tree from './Tree.vue'
import CodeView from './CodeView.vue'
import AgentPanel from './AgentPanel.vue'
import { btn } from '../components/bits'

defineProps<{ trusted: boolean }>()
</script>

<template>
  <Shell>
    <template #drawer>
      <div class="px-3 py-2 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">Explore</div>
      <div class="space-y-1.5 px-2">
        <button class="flex w-full items-center gap-1.5 rounded-md border bg-background px-2 py-1 text-left">acme/payments<ChevronDown class="ml-auto size-3.5" /></button>
        <button class="flex w-full items-center gap-1.5 rounded-md border bg-background px-2 py-1 text-left"><GitBranch class="size-3.5" /><span class="truncate">{{ explore.ref }}</span><span class="text-[11px] text-muted-foreground">· {{ explore.sha }}</span><ChevronDown class="ml-auto size-3.5" /></button>
        <div class="flex items-center gap-1.5 rounded-md border bg-background px-2 py-1 text-muted-foreground"><Search class="size-3.5" />Go to file… <kbd class="ml-auto text-[10px]">⌘P</kbd></div>
      </div>
      <div class="mt-2 min-h-0 flex-1 overflow-auto px-1 pb-4 text-[13px]"><Tree :nodes="paymentsTree" :show-changes="explore.fromPr" /></div>
    </template>

    <div class="flex items-end border-b bg-card/60 px-2 pt-1 text-xs">
      <span class="flex items-center gap-1.5 rounded-t-md border border-b-0 bg-background px-3 py-1.5">deliver.go<X class="size-3 opacity-50" /></span>
      <span class="flex items-center gap-1.5 px-3 py-1.5 text-muted-foreground">env.go<X class="size-3 opacity-50" /></span>
    </div>
    <div v-if="explore.fromPr" class="flex items-center gap-2 border-b bg-emerald-500/10 px-3 py-1.5 text-xs">
      <GitPullRequest class="size-3.5 text-emerald-500" /> <span>Exploring <b>{{ explore.ref }}</b> (#4821 head) · files and lines the PR changes are marked</span>
      <button :class="btn" class="!py-0" @click="go('review')">Back to review</button>
      <label class="ml-auto flex items-center gap-1 text-muted-foreground"><input type="checkbox" checked /> Show changes</label>
    </div>
    <div v-else class="flex items-center gap-2 border-b bg-sky-500/10 px-3 py-1.5 text-xs">
      <GitPullRequest class="size-3.5 text-sky-400" /> <span><b>feat/webhook-retry</b> has an open PR: #4821</span>
      <button :class="btn" class="!py-0" @click="go('review')">Review it</button>
      <span class="ml-auto text-muted-foreground">Read-only · worktree at {{ explore.ref }}</span>
    </div>
    <div class="flex min-h-0 flex-1">
      <main class="min-w-0 flex-1 overflow-auto">
        <div class="sticky top-0 z-10 border-b bg-background px-3 py-1.5 font-mono text-xs text-muted-foreground">internal / webhook / deliver.go</div>
        <CodeView :lines="deliverGo" :blame="blame" :bookmark-line="28" :changed="explore.fromPr ? [24, 25, 26, 27, 28] : []" />
      </main>
      <AgentPanel mode="explore" :trusted="trusted" />
    </div>
  </Shell>
</template>
