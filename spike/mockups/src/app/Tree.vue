<script setup lang="ts">
// PROTOTYPE: recursive file tree for Explore and Projects. With showChanges, files the PR
// touches are badged and files it adds appear; without it, added files are hidden (not on main).
import { Bookmark, ChevronDown, ChevronRight, File } from '@lucide/vue'
import type { TreeNode } from '../data/explore'
const props = defineProps<{ nodes: TreeNode[]; depth?: number; showChanges?: boolean }>()
const visible = (n: TreeNode) => props.showChanges || n.change !== 'A'
</script>
<template>
  <template v-for="n in nodes" :key="n.name">
    <div v-if="visible(n)">
      <div class="flex items-center gap-1 rounded py-0.5 pr-2" :style="{ paddingLeft: `${(depth ?? 0) * 12 + 6}px` }" :class="n.active ? 'bg-accent' : 'hover:bg-accent/50'">
        <template v-if="n.dir"><ChevronDown v-if="n.open" class="size-3.5 text-muted-foreground" /><ChevronRight v-else class="size-3.5 text-muted-foreground" /></template>
        <File v-else class="ml-[18px] size-3.5 text-muted-foreground" />
        <span class="truncate" :class="showChanges && n.change ? (n.change === 'A' ? 'text-emerald-500' : 'text-amber-500') : ''">{{ n.name }}</span>
        <span class="ml-auto flex items-center gap-1">
          <Bookmark v-if="n.bookmark" class="size-3 text-sky-400" />
          <span v-if="showChanges && n.change && !n.dir" class="font-mono text-[10px]" :class="n.change === 'A' ? 'text-emerald-500' : 'text-amber-500'">{{ n.change }}</span>
          <span v-else-if="showChanges && n.change" class="size-1.5 rounded-full bg-amber-500" />
        </span>
      </div>
      <Tree v-if="n.dir && n.open && n.children" :nodes="n.children" :depth="(depth ?? 0) + 1" :show-changes="showChanges" />
    </div>
  </template>
</template>
