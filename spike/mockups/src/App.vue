<script setup lang="ts">
// PROTOTYPE: Docent UI mockups. Three structurally different variants each of the
// PR review view and the inbox, switchable with ?screen= and ?variant=.
// Throwaway: lives on spike/ui-mockups, never merged.
import { computed, watchEffect } from 'vue'
import { state } from './url'
import Switcher from './Switcher.vue'
import PrA from './variants/PrA.vue'
import PrB from './variants/PrB.vue'
import PrC from './variants/PrC.vue'
import InboxA from './variants/InboxA.vue'
import InboxB from './variants/InboxB.vue'
import InboxC from './variants/InboxC.vue'
import { page } from './url'
import InboxPage from './app/InboxPage.vue'
import ReviewPage from './app/ReviewPage.vue'
import ExplorePage from './app/ExplorePage.vue'
import ProjectPage from './app/ProjectPage.vue'

const screens = {
  app: { A: ['Activity bar', null], B: ['Top tabs', null] },
  pr: { A: ['Three panes', PrA], B: ['Inline + popover', PrB], C: ['Triage queue', PrC] },
  inbox: { A: ['Dense table', InboxA], B: ['Mail split', InboxB], C: ['Board', InboxC] },
} as const

const pages = { inbox: InboxPage, review: ReviewPage, explore: ExplorePage, project: ProjectPage } as const
const current = computed(() => {
  if (state.screen === 'app') return [0, pages[page.current as keyof typeof pages] ?? InboxPage]
  const s = screens[state.screen as keyof typeof screens] ?? screens.pr
  return (s as any)[state.variant] ?? s.A
})
watchEffect(() => document.documentElement.classList.toggle('dark', state.theme === 'dark'))
</script>

<template>
  <div class="h-full">
    <component :is="current[1]" :trusted="state.trust === 'trusted'" />
    <Switcher :screens="screens" />
  </div>
</template>
