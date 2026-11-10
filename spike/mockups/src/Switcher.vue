<script setup lang="ts">
// PROTOTYPE: floating variant switcher. Not part of any design being evaluated.
import { computed, onMounted, onUnmounted } from 'vue'
import { ChevronLeft, ChevronRight } from '@lucide/vue'
import { state, set } from './url'

const props = defineProps<{ screens: Record<string, Record<string, readonly [string, unknown]>> }>()
const keys = computed(() => Object.keys(props.screens[state.screen] ?? props.screens.pr))
const label = computed(() => (props.screens[state.screen] as any)?.[state.variant]?.[0] ?? '')

function step(d: number) {
  const k = keys.value
  const i = (k.indexOf(state.variant) + d + k.length) % k.length
  set({ variant: k[i] })
}
function onKey(e: KeyboardEvent) {
  const t = e.target as HTMLElement
  if (t.closest('input, textarea, [contenteditable]')) return
  if (e.key === 'ArrowLeft') step(-1)
  if (e.key === 'ArrowRight') step(1)
}
onMounted(() => addEventListener('keydown', onKey))
onUnmounted(() => removeEventListener('keydown', onKey))
const show = import.meta.env.DEV
</script>

<template>
  <div v-if="show" class="fixed bottom-4 left-1/2 z-50 flex -translate-x-1/2 items-center gap-1 rounded-full bg-fuchsia-600 px-2 py-1.5 text-xs font-medium text-white shadow-2xl ring-2 ring-white/40">
    <button v-for="s in Object.keys(screens)" :key="s" class="rounded-full px-2.5 py-1 capitalize"
      :class="state.screen === s ? 'bg-white text-fuchsia-700' : 'hover:bg-white/20'"
      @click="set({ screen: s, variant: 'A' })">{{ s === 'pr' ? 'PR view' : s }}</button>
    <span class="mx-1 h-4 w-px bg-white/40" />
    <button class="rounded-full p-1 hover:bg-white/20" @click="step(-1)"><ChevronLeft class="size-4" /></button>
    <span class="min-w-36 text-center">{{ state.variant }} ({{ label }})</span>
    <button class="rounded-full p-1 hover:bg-white/20" @click="step(1)"><ChevronRight class="size-4" /></button>
    <span class="mx-1 h-4 w-px bg-white/40" />
    <button class="rounded-full px-2.5 py-1 hover:bg-white/20" @click="set({ trust: state.trust === 'trusted' ? 'untrusted' : 'trusted' })">
      {{ state.trust === 'trusted' ? 'Trusted' : 'Untrusted' }}
    </button>
    <button class="rounded-full px-2.5 py-1 hover:bg-white/20" @click="set({ theme: state.theme === 'dark' ? 'light' : 'dark' })">
      {{ state.theme === 'dark' ? 'Dark' : 'Light' }}
    </button>
  </div>
</template>
