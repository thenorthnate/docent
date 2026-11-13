<script setup lang="ts">
import { onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { messageOf } from "@/lib/ipc";

/**
 * A host the user connected, as the Rust core reports it. It never includes
 * the token. `error` says why a remembered host failed to reconnect at
 * startup, and is null while it is connected.
 */
type Account = { host: string; login: string; error: { kind: string; message: string } | null };

// Reactive component state: the template re-renders when any of these change.
/** The connected hosts, as last read from the Rust core. */
const accounts = ref<Account[]>([]);
/** The host typed into the form, bound to the input with `v-model`. */
const host = ref("github.com");
/** True while a connect is in flight, to disable the button. */
const connecting = ref(false);
/** The last error to show, or null when there is none. */
const error = ref<string | null>(null);

async function refresh() {
  try {
    accounts.value = await invoke<Account[]>("list_accounts");
  } catch (err) {
    error.value = messageOf(err);
  }
}

/** Connects `target`: the typed host, or a remembered one to reconnect. */
async function connect(target: string = host.value) {
  connecting.value = true;
  error.value = null;
  try {
    await invoke<Account>("connect_host", { host: target });
    await refresh();
  } catch (err) {
    error.value = messageOf(err);
  } finally {
    connecting.value = false;
  }
}

onMounted(refresh);
</script>

<template>
  <section
    aria-labelledby="accounts-heading"
    class="space-y-3"
  >
    <h2
      id="accounts-heading"
      class="text-base font-semibold"
    >
      Accounts
    </h2>
    <p class="text-sm text-muted-foreground">
      Docent uses the token from your GitHub CLI (<code>gh</code>) login and keeps it in memory only.
      It remembers which hosts you connected (never the token) and reconnects them through <code>gh</code> when it starts.
      Only GitHub (github.com and GitHub Enterprise Server) is supported for now.
    </p>
    <ul
      v-if="accounts.length"
      class="divide-y rounded-lg border text-sm"
    >
      <li
        v-for="account in accounts"
        :key="account.host"
        data-testid="account"
        class="space-y-1.5 px-3 py-2"
      >
        <div class="flex justify-between">
          <span class="font-medium">{{ account.host }}</span>
          <span class="text-muted-foreground">{{ account.login }}</span>
        </div>
        <div
          v-if="account.error"
          class="flex items-start justify-between gap-2"
        >
          <p class="text-destructive">
            {{ account.error.message }}
          </p>
          <Button
            variant="outline"
            size="sm"
            :aria-label="`Reconnect ${account.host}`"
            :disabled="connecting"
            @click="connect(account.host)"
          >
            Reconnect
          </Button>
        </div>
      </li>
    </ul>
    <form
      class="flex gap-2"
      @submit.prevent="connect()"
    >
      <label
        for="account-host"
        class="sr-only"
      >Host</label>
      <input
        id="account-host"
        v-model="host"
        name="host"
        type="text"
        autocomplete="off"
        spellcheck="false"
        placeholder="github.com"
        class="h-8 flex-1 rounded-lg border bg-background px-2.5 text-sm"
      >
      <Button
        type="submit"
        :disabled="connecting || !host.trim()"
      >
        {{ connecting ? "Connecting…" : "Connect" }}
      </Button>
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
