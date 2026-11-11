<script setup lang="ts">
import { onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

const version = ref<string | null>(null);
const versionFailed = ref(false);

onMounted(async () => {
  try {
    version.value = await invoke<string>("app_version");
  } catch (error) {
    versionFailed.value = true;
    console.error("Could not read the app version", error);
  }
});
</script>

<template>
  <div class="p-6">
    <h1 class="text-lg font-semibold">
      Settings
    </h1>
    <p class="text-sm text-muted-foreground">
      <template v-if="version">
        Docent v{{ version }}
      </template>
      <template v-else-if="versionFailed">
        Docent, version unknown
      </template>
    </p>
  </div>
</template>
