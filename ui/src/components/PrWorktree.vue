<script setup lang="ts">
import { ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import CloneButton from "@/components/CloneButton.vue";
import { Button } from "@/components/ui/button";
import { messageOf } from "@/lib/ipc";
import { useClonesStore } from "@/stores/clones";

const props = defineProps<{ host: string; owner: string; repo: string; number: number }>();
const emit = defineEmits<{ ready: [headSha: string] }>();

/** A watched repo, as the Rust core reports it. */
type WatchedRepo = { id: number; owner: string; name: string; local_path: string | null };
/** What `open_pr_worktree` reports (PLAN §5.7). */
type Opened = { status: "ready"; path: string; head_sha: string } | { status: "unbound"; repo: WatchedRepo };
type State = { status: "preparing" } | Opened | { status: "error"; message: string };

const state = ref<State>({ status: "preparing" });
const binding = ref(false);
const clones = useClonesStore();
/** Counts requests, so an answer for a PR we've navigated away from is dropped. */
let latest = 0;

async function open() {
  const request = ++latest;
  state.value = { status: "preparing" };
  try {
    const opened = await invoke<Opened>("open_pr_worktree", {
      host: props.host,
      owner: props.owner,
      repo: props.repo,
      number: props.number,
    });
    if (request !== latest) return;
    state.value = opened;
    if (opened.status === "ready") emit("ready", opened.head_sha);
  } catch (err) {
    if (request === latest) state.value = { status: "error", message: messageOf(err) };
  }
}

/** The core opens the folder picker and checks the clone; `null` means cancelled. */
async function bind(id: number) {
  binding.value = true;
  try {
    const bound = await invoke<WatchedRepo | null>("bind_repo", { id });
    if (bound) await open();
  } catch (err) {
    state.value = { status: "error", message: messageOf(err) };
  } finally {
    binding.value = false;
  }
}

watch(() => [props.host, props.owner, props.repo, props.number], open, { immediate: true });
// The clone may have been started elsewhere, such as Watched repos.
watch(
  () => clones.finished,
  () => {
    if (state.value.status === "unbound") void open();
  },
);
</script>

<template>
  <div
    data-testid="worktree"
    class="text-sm"
  >
    <p
      v-if="state.status === 'preparing'"
      class="text-muted-foreground"
    >
      Preparing worktree…
    </p>
    <p
      v-else-if="state.status === 'ready'"
      class="text-muted-foreground"
    >
      Worktree ready at
      <code
        class="font-mono"
        :title="state.path"
      >{{ state.head_sha.slice(0, 7) }}</code>
    </p>
    <div
      v-else-if="state.status === 'unbound'"
      class="space-y-2"
    >
      <p class="text-muted-foreground">
        {{ state.repo.owner }}/{{ state.repo.name }} isn't bound to a local clone yet. Docent reads PRs from
        a clone on this machine.
      </p>
      <div class="flex flex-wrap gap-1">
        <Button
          variant="outline"
          size="xs"
          :disabled="binding || state.repo.id in clones.running"
          @click="bind(state.repo.id)"
        >
          Bind
        </Button>
        <CloneButton
          :id="state.repo.id"
          :disabled="binding"
        />
      </div>
    </div>
    <div
      v-else
      class="flex items-center gap-2"
    >
      <p
        role="alert"
        class="text-destructive"
      >
        {{ state.message }}
      </p>
      <Button
        variant="outline"
        size="xs"
        @click="open"
      >
        Retry
      </Button>
    </div>
  </div>
</template>
