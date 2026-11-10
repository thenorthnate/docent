<script setup lang="ts">
// PROTOTYPE: app shell. Variant A = VS Code-style activity bar + drawer. Variant B = browser-style
// tabs across the top, for comparison. Pages wrap themselves in this and fill #drawer and the default slot.
import { ref } from 'vue'
import { Bookmark, FolderGit2, GitPullRequest, Inbox, Layers, Lightbulb, Settings, Telescope, X } from '@lucide/vue'
import { go, page, state } from '../url'
import { bookmarks, steering } from '../data/explore'
import { btn, btnPrimary } from '../components/bits'

defineProps<{ title?: string }>()
const rail = [
  { key: 'inbox', icon: Inbox, label: 'Inbox', badge: 4 },
  { key: 'review', icon: GitPullRequest, label: 'Review', badge: 0 },
  { key: 'explore', icon: Telescope, label: 'Explore', badge: 0 },
  { key: 'project', icon: Layers, label: 'Projects', badge: 0 },
  { key: 'bookmarks', icon: Bookmark, label: 'Bookmarks', badge: 0 },
  { key: 'steering', icon: Lightbulb, label: 'Steering suggestions', badge: steering.length },
]
const pages = ['inbox', 'review', 'explore', 'project', 'settings']
const panel = ref<string | null>(page.current)

function click(key: string) {
  if (pages.includes(key) && key !== page.current) { go(key); panel.value = key; return }
  panel.value = panel.value === key ? null : key
}
const tabs = [
  { key: 'inbox', label: 'Inbox', icon: Inbox, pinned: true },
  { key: 'review', label: '#4821 retry webhooks', icon: GitPullRequest },
  { key: 'explore', label: 'payments @ main', icon: FolderGit2 },
  { key: 'project', label: 'checkout-stack', icon: Layers },
]
</script>

<template>
  <div class="flex h-full flex-col">
    <!-- B: tabs -->
    <div v-if="state.variant === 'B'" class="flex items-end gap-0.5 border-b bg-card px-2 pt-1.5">
      <button v-for="t in tabs" :key="t.key" class="flex items-center gap-1.5 rounded-t-md border border-b-0 px-3 py-1.5 text-xs"
        :class="page.current === t.key ? 'bg-background' : 'border-transparent text-muted-foreground hover:bg-accent/50'" @click="go(t.key); panel = t.key">
        <component :is="t.icon" class="size-3.5" />{{ t.label }}<X v-if="!t.pinned" class="size-3 opacity-50" />
      </button>
      <div class="ml-auto flex items-center gap-1 pb-1">
        <button :class="btn" @click="panel = panel === 'bookmarks' ? page.current : 'bookmarks'"><Bookmark class="size-3.5" /></button>
        <button :class="btn" @click="panel = panel === 'steering' ? page.current : 'steering'"><Lightbulb class="size-3.5" /><span class="rounded-full bg-fuchsia-500 px-1.5 text-[10px] text-white">{{ steering.length }}</span></button>
        <button :class="btn" @click="go('settings'); panel = 'settings'"><Settings class="size-3.5" /></button>
      </div>
    </div>

    <div class="flex min-h-0 flex-1">
      <!-- A: activity bar -->
      <nav v-if="state.variant !== 'B'" class="flex w-12 shrink-0 flex-col items-center gap-1 border-r bg-card py-2">
        <button v-for="r in rail" :key="r.key" :title="r.label" class="relative flex size-9 items-center justify-center rounded-md"
          :class="panel === r.key || (!panel && page.current === r.key) ? 'text-foreground' : 'text-muted-foreground hover:text-foreground'" @click="click(r.key)">
          <span v-if="page.current === r.key" class="absolute -left-1.5 top-1.5 h-6 w-0.5 rounded bg-primary" />
          <component :is="r.icon" class="size-5" />
          <span v-if="r.badge" class="absolute right-0.5 top-0.5 min-w-4 rounded-full px-1 text-[9px] leading-4 text-white" :class="r.key === 'steering' ? 'bg-fuchsia-500' : 'bg-sky-500'">{{ r.badge }}</span>
        </button>
        <button title="Settings" class="relative mt-auto flex size-9 items-center justify-center rounded-md" :class="page.current === 'settings' ? 'text-foreground' : 'text-muted-foreground hover:text-foreground'" @click="click('settings')">
          <span v-if="page.current === 'settings'" class="absolute -left-1.5 top-1.5 h-6 w-0.5 rounded bg-primary" /><Settings class="size-5" /></button>
      </nav>

      <!-- drawer -->
      <aside v-if="panel" class="flex w-64 shrink-0 flex-col overflow-hidden border-r bg-card/60">
        <template v-if="panel === 'bookmarks'">
          <div class="px-3 py-2 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">Bookmarks</div>
          <div v-for="b in bookmarks" :key="b.loc" class="mx-1 rounded-md px-2 py-1.5 hover:bg-accent">
            <div class="truncate font-mono text-[11px]"><span class="text-muted-foreground">{{ b.repo }}/</span>{{ b.loc }}</div>
            <div class="truncate font-mono text-[11px] text-muted-foreground">{{ b.snippet }}</div>
            <div v-if="b.stale" class="text-[11px] text-amber-500">Snippet not found on main · see original commit</div>
          </div>
        </template>
        <template v-else-if="panel === 'steering'">
          <div class="px-3 py-2 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">Steering suggestions</div>
          <div v-for="s in steering" :key="s.text" class="mx-2 mb-2 rounded-lg border bg-background p-2.5">
            <div class="text-[11px] text-muted-foreground">{{ s.lens }} · from {{ s.from }}</div>
            <div class="mt-0.5">{{ s.text }}</div>
            <div v-if="s.merge" class="mt-1 text-[11px] text-sky-400">{{ s.merge }}</div>
            <div class="mt-2 flex flex-wrap gap-1"><button :class="btnPrimary">Accept</button><button :class="btn">Edit</button><button :class="btn">Repo ▾</button><button :class="btn">Dismiss</button></div>
          </div>
        </template>
        <slot v-else name="drawer" />
      </aside>

      <div class="flex min-w-0 flex-1 flex-col"><slot /></div>
    </div>
  </div>
</template>
