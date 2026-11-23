<script setup lang="ts">
import { computed } from "vue";
import { renderMarkdown } from "@/lib/markdown";

const props = defineProps<{ source: string }>();

const html = computed(() => renderMarkdown(props.source));

/**
 * A link would navigate the app's own window away from Docent, so clicks are
 * dropped; the URL shows on hover. Opening links in the browser comes later.
 */
function holdLinks(event: MouseEvent) {
  if ((event.target as Element | null)?.closest("a")) event.preventDefault();
}

/** Each link's URL as its tooltip, once rendered. */
function titleLinks(el: Element | null) {
  el?.querySelectorAll("a[href]").forEach((a) => a.setAttribute("title", a.getAttribute("href") ?? ""));
}
</script>

<template>
  <!-- The only v-html for forge content: renderMarkdown escapes raw HTML (see lib/markdown.ts). -->
  <!-- eslint-disable vue/no-v-html -->
  <div
    :ref="(el) => titleLinks(el as Element | null)"
    class="markdown text-sm break-words"
    @click="holdLinks"
    v-html="html"
  />
  <!-- eslint-enable vue/no-v-html -->
</template>
