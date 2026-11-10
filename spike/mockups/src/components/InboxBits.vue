<script setup lang="ts">
// PROTOTYPE: small shared inbox atoms (CI icon, updated badge, trust badge).
import { CircleCheck, CircleDot, CircleX, ShieldAlert } from '@lucide/vue'
import type { InboxPr } from '../data/mock'
defineProps<{ p: InboxPr; part: 'ci' | 'updated' | 'trust' }>()
</script>
<template>
  <template v-if="part === 'ci'">
    <CircleCheck v-if="p.ci === 'passing'" class="size-3.5 text-emerald-500" />
    <CircleX v-else-if="p.ci === 'failing'" class="size-3.5 text-red-500" />
    <CircleDot v-else class="size-3.5 text-amber-500" />
  </template>
  <span v-else-if="part === 'updated' && p.updated" class="whitespace-nowrap rounded-full bg-sky-500/15 px-2 py-0.5 text-[11px] text-sky-400">{{ p.updated }}</span>
  <span v-else-if="part === 'trust' && !p.trusted" class="inline-flex items-center gap-1 whitespace-nowrap rounded-full bg-amber-500/15 px-2 py-0.5 text-[11px] text-amber-500"><ShieldAlert class="size-3" />{{ p.fork ? 'fork · untrusted' : 'untrusted' }}</span>
</template>
