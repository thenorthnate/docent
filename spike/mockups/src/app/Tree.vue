<script setup lang="ts">
// PROTOTYPE: recursive file tree for Explore and Projects.
import { Bookmark, ChevronDown, ChevronRight, File } from '@lucide/vue'
import type { TreeNode } from '../data/explore'
defineProps<{ nodes: TreeNode[]; depth?: number }>()
</script>
<template>
  <div v-for="n in nodes" :key="n.name">
    <div class="flex items-center gap-1 rounded py-0.5 pr-2" :style="{ paddingLeft: `${(depth ?? 0) * 12 + 6}px` }" :class="n.active ? 'bg-accent' : 'hover:bg-accent/50'">
      <template v-if="n.dir"><ChevronDown v-if="n.open" class="size-3.5 text-muted-foreground" /><ChevronRight v-else class="size-3.5 text-muted-foreground" /></template>
      <File v-else class="ml-[18px] size-3.5 text-muted-foreground" />
      <span class="truncate">{{ n.name }}</span>
      <Bookmark v-if="n.bookmark" class="ml-auto size-3 text-sky-400" />
    </div>
    <Tree v-if="n.dir && n.open && n.children" :nodes="n.children" :depth="(depth ?? 0) + 1" />
  </div>
</template>
