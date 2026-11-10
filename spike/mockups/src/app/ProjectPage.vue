<script setup lang="ts">
// PROTOTYPE: Project explore (PLAN.md §5.9, M6b). Several bound repos, each at its own ref,
// one multi-root tree, editors side by side, Trace across repos with inferred links marked.
import { ChevronDown, Layers, Plus, ShieldCheck } from '@lucide/vue'
import { deploymentYaml, envGo, infraTree, paymentsTree, projects } from '../data/explore'
import Shell from './Shell.vue'
import Tree from './Tree.vue'
import CodeView from './CodeView.vue'
import AgentPanel from './AgentPanel.vue'

defineProps<{ trusted: boolean }>()
const project = projects[0]
</script>

<template>
  <Shell>
    <template #drawer>
      <div class="px-3 py-2 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">Projects</div>
      <div class="px-2"><button class="flex w-full items-center gap-1.5 rounded-md border bg-background px-2 py-1"><Layers class="size-3.5" />{{ project.name }}<ChevronDown class="ml-auto size-3.5" /></button></div>
      <div class="mt-2 min-h-0 flex-1 overflow-auto px-1 pb-4 text-[13px]">
        <template v-for="(r, i) in project.repos" :key="r.name">
          <div class="mt-1 flex items-center gap-1.5 px-1.5 py-1 text-[11px] font-semibold uppercase tracking-wide">
            <ShieldCheck class="size-3.5 text-emerald-500" />{{ r.name.split('/')[1] }}<span class="ml-auto rounded bg-muted px-1.5 font-mono font-normal normal-case">{{ r.ref }}</span>
          </div>
          <Tree :nodes="i === 0 ? paymentsTree : infraTree" />
        </template>
        <button class="mt-2 flex items-center gap-1 px-2 text-[11px] text-sky-400"><Plus class="size-3" /> Add repo</button>
      </div>
    </template>

    <div class="flex items-center gap-2 border-b px-3 py-2 text-xs">
      <Layers class="size-4" /><span class="font-semibold">{{ project.name }}</span>
      <span v-for="r in project.repos" :key="r.name" class="rounded-full border px-2 py-0.5">{{ r.name }} @ <span class="font-mono">{{ r.ref }}</span></span>
      <span class="ml-auto text-muted-foreground">All repos trusted · agents see both worktrees</span>
    </div>
    <div class="flex min-h-0 flex-1">
      <main class="grid min-w-0 flex-1 grid-cols-2 divide-x overflow-hidden">
        <div class="overflow-auto">
          <div class="sticky top-0 flex items-center gap-1.5 border-b bg-background px-3 py-1.5 text-xs"><span class="rounded bg-muted px-1.5">payments @ main</span><span class="font-mono text-muted-foreground">internal/config/env.go</span></div>
          <CodeView :lines="envGo" />
        </div>
        <div class="overflow-auto">
          <div class="sticky top-0 flex items-center gap-1.5 border-b bg-background px-3 py-1.5 text-xs"><span class="rounded bg-muted px-1.5">infra-deploy @ v2.14.0</span><span class="font-mono text-muted-foreground">k8s/payments/deployment.yaml</span></div>
          <CodeView :lines="deploymentYaml" />
        </div>
      </main>
      <AgentPanel mode="project" :trusted="trusted" />
    </div>
  </Shell>
</template>
