<script setup lang="ts">
import { useRoute } from "vue-router";
import ChangedFiles from "@/components/ChangedFiles.vue";

const route = useRoute();
</script>

<template>
  <div class="space-y-3 p-3">
    <h2 class="text-sm font-semibold">
      Review
    </h2>
    <ChangedFiles v-if="route.name === 'review-pr'" />
  </div>
</template>
