<script setup lang="ts">
import { onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

const version = ref<string | null>(null);

onMounted(async () => {
  version.value = await invoke<string>("app_version");
});
</script>

<template>
  <main class="container">
    <h1>Docent</h1>
    <p v-if="version">
      v{{ version }}
    </p>
  </main>
</template>

<style>
:root {
  font-family: system-ui, sans-serif;
  color: #0f0f0f;
  background-color: #f6f6f6;
}

@media (prefers-color-scheme: dark) {
  :root {
    color: #f6f6f6;
    background-color: #2f2f2f;
  }
}

.container {
  padding-top: 10vh;
  text-align: center;
}
</style>
