<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { storeToRefs } from "pinia";
import { useRouter } from "vue-router";
import InboxPreview from "@/components/InboxPreview.vue";
import ReviewRequestedList from "@/components/ReviewRequestedList.vue";
import { pullKey } from "@/lib/pull";
import { useInboxStore, type PullSummary } from "@/stores/inbox";

const router = useRouter();
const { pulls } = storeToRefs(useInboxStore());
const selectedKey = ref<string | null>(null);
/** The selected PR, or `null` once a reload no longer lists it. */
const selected = computed(() => pulls.value.find((pr) => pullKey(pr) === selectedKey.value) ?? null);

/** Moves the selection by `step` rows, from the top when nothing is selected, stopping at the ends. */
function move(step: number) {
  if (!pulls.value.length) return;
  const at = pulls.value.findIndex((pr) => pullKey(pr) === selectedKey.value);
  const next = at === -1 ? 0 : Math.min(Math.max(at + step, 0), pulls.value.length - 1);
  selectedKey.value = pullKey(pulls.value[next]);
}

function open(pr: PullSummary) {
  void router.push({
    name: "review-pr",
    params: { host: pr.host, owner: pr.owner, repo: pr.repo, number: String(pr.number) },
  });
}

/** Whether a key press belongs to something the user is typing into. */
function isTyping(target: EventTarget | null) {
  return target instanceof HTMLElement && (target.isContentEditable || target.closest("input, textarea, select") !== null);
}

function onKeydown(event: KeyboardEvent) {
  if (event.defaultPrevented || event.altKey || event.ctrlKey || event.metaKey || isTyping(event.target)) return;
  if (event.key === "Enter") {
    // A focused button or link handles its own Enter.
    if (!selected.value || (event.target instanceof Element && event.target.closest("a, button"))) return;
    event.preventDefault();
    open(selected.value);
    return;
  }
  const step = { j: 1, ArrowDown: 1, k: -1, ArrowUp: -1 }[event.key];
  if (step === undefined) return;
  event.preventDefault();
  move(step);
}

onMounted(() => document.addEventListener("keydown", onKeydown));
onBeforeUnmount(() => document.removeEventListener("keydown", onKeydown));
</script>

<template>
  <div class="flex h-full">
    <div class="min-w-0 flex-1 space-y-4 overflow-y-auto p-6">
      <h1 class="text-lg font-semibold">
        Inbox
      </h1>
      <ReviewRequestedList
        v-model:selected="selectedKey"
        @open="open"
      />
    </div>
    <aside class="w-96 shrink-0 overflow-y-auto border-l p-6">
      <InboxPreview
        :pull="selected"
        @open="open"
      />
    </aside>
  </div>
</template>
