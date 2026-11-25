<script setup lang="ts">
import { computed, onMounted, ref, watch as watchValue } from "vue";
import { invoke } from "@tauri-apps/api/core";
import CloneButton from "@/components/CloneButton.vue";
import { Button } from "@/components/ui/button";
import { messageOf } from "@/lib/ipc";
import { useClonesStore } from "@/stores/clones";
import { useInboxStore } from "@/stores/inbox";

/** A host, as the Rust core reports it. `error` is set when it failed to reconnect. */
type Account = { host: string; login: string; error: { kind: string; message: string } | null };
/** A watched repo, as the Rust core reports it. */
type WatchedRepo = {
  id: number;
  host: string;
  owner: string;
  name: string;
  default_branch: string;
  local_path: string | null;
};

/** What Bind does (PLAN §4, "Repo binding"). */
const BIND_TOOLTIP = "Point Docent at a clone of this repo you already have, so diffs come from it.";

/** The connected hosts: the ones a repo can be watched on. */
const accounts = ref<Account[]>([]);
const repos = ref<WatchedRepo[]>([]);
/** The host to watch on. Only offered as a choice when several are connected. */
const host = ref("");
/** The `owner/name` typed into the form. */
const repo = ref("");
const watching = ref(false);
const error = ref<string | null>(null);
const inbox = useInboxStore();
const clones = useClonesStore();

/** The host picker only appears when there is a choice to make. */
const pickHost = computed(() => accounts.value.length > 1);
/** Rows name their host only when the watched repos span more than one. */
const showRepoHost = computed(() => new Set(repos.value.map((r) => r.host)).size > 1);

/** Reads both lists independently, so one failing doesn't hide the other. */
async function refresh() {
  const [connected, watched] = await Promise.allSettled([
    invoke<Account[]>("list_accounts"),
    invoke<WatchedRepo[]>("list_watched_repos"),
  ]);
  if (watched.status === "fulfilled") {
    repos.value = watched.value;
  } else {
    error.value = messageOf(watched.reason);
  }
  if (connected.status === "fulfilled") {
    // A host that failed to reconnect has no token, so it can't look a repo up.
    accounts.value = connected.value.filter((a) => !a.error);
    if (!accounts.value.some((a) => a.host === host.value)) {
      host.value = accounts.value[0]?.host ?? "";
    }
  } else {
    error.value = messageOf(connected.reason);
  }
}

async function watch() {
  watching.value = true;
  error.value = null;
  try {
    await invoke<WatchedRepo>("watch_repo", { host: host.value, repo: repo.value });
    repo.value = "";
    // The inbox list follows the watched repos; it reloads on its own time.
    void inbox.load();
    await refresh();
  } catch (err) {
    error.value = messageOf(err);
  } finally {
    watching.value = false;
  }
}

/** The repo whose folder picker is open, so its Bind can't be pressed twice. */
const binding = ref<number | null>(null);

/** The core opens the folder picker and checks the clone; `null` means cancelled. */
async function bind(id: number) {
  binding.value = id;
  error.value = null;
  try {
    const bound = await invoke<WatchedRepo | null>("bind_repo", { id });
    if (bound) await refresh();
  } catch (err) {
    error.value = messageOf(err);
  } finally {
    binding.value = null;
  }
}

async function unwatch(id: number) {
  error.value = null;
  try {
    await invoke("unwatch_repo", { id });
    void inbox.load();
    await refresh();
  } catch (err) {
    error.value = messageOf(err);
  }
}

watchValue(() => clones.finished, refresh);
onMounted(refresh);
</script>

<template>
  <section
    aria-labelledby="watched-repos-heading"
    class="space-y-2"
  >
    <h3
      id="watched-repos-heading"
      class="text-xs font-semibold tracking-wide text-muted-foreground uppercase"
    >
      Watched repos
    </h3>
    <ul
      v-if="repos.length"
      class="space-y-1 text-sm"
    >
      <li
        v-for="r in repos"
        :key="r.id"
        data-testid="watched-repo"
        class="space-y-1 rounded-md px-1 py-1"
      >
        <div class="flex items-center justify-between gap-1">
          <span
            class="truncate font-medium"
            :title="`${r.host}:${r.owner}/${r.name}`"
          >
            {{ r.owner }}/{{ r.name }}
          </span>
          <Button
            variant="ghost"
            size="xs"
            :aria-label="`Unwatch ${r.owner}/${r.name}`"
            @click="unwatch(r.id)"
          >
            Unwatch
          </Button>
        </div>
        <p
          v-if="showRepoHost"
          class="text-xs text-muted-foreground"
        >
          {{ r.host }}
        </p>
        <p
          v-if="r.local_path"
          class="truncate text-xs text-muted-foreground"
          :title="r.local_path"
        >
          {{ r.local_path }}
        </p>
        <div
          v-else
          class="flex flex-wrap gap-1"
        >
          <Button
            variant="outline"
            size="xs"
            :title="BIND_TOOLTIP"
            :disabled="binding !== null || r.id in clones.running"
            @click="bind(r.id)"
          >
            Bind
          </Button>
          <CloneButton
            :id="r.id"
            :disabled="binding === r.id"
          />
        </div>
      </li>
    </ul>
    <p
      v-if="!accounts.length"
      class="text-sm text-muted-foreground"
    >
      Connect a host in Settings → Accounts to watch its repos.
    </p>
    <form
      v-else
      class="space-y-1.5"
      @submit.prevent="watch"
    >
      <template v-if="pickHost">
        <label
          for="watch-host"
          class="sr-only"
        >Host</label>
        <select
          id="watch-host"
          v-model="host"
          name="host"
          class="h-7 w-full rounded-md border bg-background px-1.5 text-sm"
        >
          <option
            v-for="a in accounts"
            :key="a.host"
            :value="a.host"
          >
            {{ a.host }}
          </option>
        </select>
      </template>
      <div class="flex gap-1">
        <label
          for="watch-repo"
          class="sr-only"
        >Repo</label>
        <input
          id="watch-repo"
          v-model="repo"
          name="repo"
          type="text"
          autocomplete="off"
          spellcheck="false"
          placeholder="owner/name"
          class="h-7 min-w-0 flex-1 rounded-md border bg-background px-2 text-sm"
        >
        <Button
          type="submit"
          size="sm"
          :disabled="watching || !repo.trim()"
        >
          {{ watching ? "Watching…" : "Watch" }}
        </Button>
      </div>
    </form>
    <p
      v-if="error"
      role="alert"
      class="text-sm text-destructive"
    >
      {{ error }}
    </p>
  </section>
</template>
