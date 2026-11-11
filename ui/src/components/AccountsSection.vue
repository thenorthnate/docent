<script setup lang="ts">
import { onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";

/** A connected host, as the Rust core reports it. It never includes the token. */
type Account = { host: string; login: string };

const accounts = ref<Account[]>([]);
const host = ref("github.com");
const connecting = ref(false);
const error = ref<string | null>(null);

function messageOf(err: unknown): string {
  if (typeof err === "object" && err !== null && "message" in err) {
    return String(err.message);
  }
  return String(err);
}

async function refresh() {
  try {
    accounts.value = await invoke<Account[]>("list_accounts");
  } catch (err) {
    error.value = messageOf(err);
  }
}

async function connect() {
  connecting.value = true;
  error.value = null;
  try {
    await invoke<Account>("connect_host", { host: host.value });
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
    </p>
    <ul
      v-if="accounts.length"
      class="divide-y rounded-lg border text-sm"
    >
      <li
        v-for="account in accounts"
        :key="account.host"
        data-testid="account"
        class="flex justify-between px-3 py-2"
      >
        <span class="font-medium">{{ account.host }}</span>
        <span class="text-muted-foreground">{{ account.login }}</span>
      </li>
    </ul>
    <form
      class="flex gap-2"
      @submit.prevent="connect"
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
