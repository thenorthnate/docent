<script setup lang="ts">
// PROTOTYPE: content for the untrusted state (PLAN.md §5.6). Placement is per variant.
import { ShieldAlert } from '@lucide/vue'
import { btn, btnPrimary } from './bits'
defineProps<{ compact?: boolean }>()
</script>

<template>
  <div class="rounded-lg border border-amber-500/40 bg-amber-500/10 p-3 text-[13px]">
    <div class="flex items-center gap-2 font-medium text-amber-500"><ShieldAlert class="size-4" /> Agents are off for this PR</div>
    <p class="mt-1 text-muted-foreground" v-if="!compact">
      <b class="text-foreground">acme/web-checkout</b> isn't trusted, and this PR comes from a fork. Docent won't start an agent or linter in its worktree until you trust it. You can still read the diff.
    </p>
    <div class="mt-2 flex flex-wrap gap-2">
      <button :class="btnPrimary">Trust this PR</button>
      <button :class="btn">Trust repo…</button>
      <button :class="btn" v-if="!compact">What does trust allow?</button>
    </div>
  </div>
</template>
