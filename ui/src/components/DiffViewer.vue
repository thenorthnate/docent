<script setup lang="ts">
import { onBeforeUnmount, ref, shallowRef, useTemplateRef, watch } from "vue";
import { storeToRefs } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { EditorState } from "@codemirror/state";
import { EditorView, lineNumbers } from "@codemirror/view";
import { Button } from "@/components/ui/button";
import { gitDiffView } from "@/lib/gitChunks";
import { messageOf } from "@/lib/ipc";
import type { ChangedFile, FileDiff } from "@/lib/review";
import { useReviewStore } from "@/stores/review";

const review = useReviewStore();
const { range, currentFile } = storeToRefs(review);

const diff = shallowRef<FileDiff | null>(null);
const error = ref<string | null>(null);
const host = useTemplateRef<HTMLElement>("host");
let editor: EditorView | null = null;

/** Counts loads, so only the latest one started may write its answer. */
let latest = 0;

async function load() {
  const id = ++latest;
  diff.value = null;
  error.value = null;
  const file = currentFile.value;
  if (!range.value || !file || isSubmodule(file)) return;
  try {
    const blobs = { old_blob: file.old_blob, new_blob: file.new_blob };
    const found = await invoke<FileDiff>("file_diff", { range: range.value, blobs });
    if (id === latest) diff.value = found;
  } catch (err) {
    if (id === latest) error.value = messageOf(err);
  }
}

watch(currentFile, load, { immediate: true });

// The editor is made once its element is in the page, and remade for each diff.
watch(
  [diff, host],
  ([shown, parent]) => {
    editor?.destroy();
    editor = null;
    if (shown?.kind !== "text" || !parent) return;
    const extensions = [lineNumbers(), EditorView.editable.of(false), gitDiffView(shown.old, shown.hunks)];
    editor = new EditorView({ parent, state: EditorState.create({ doc: shown.new, extensions }) });
  },
  { flush: "post" },
);

onBeforeUnmount(() => editor?.destroy());

function mebibytes(bytes: number): string {
  return `${(bytes / (1024 * 1024)).toFixed(1)} MiB`;
}

/** A changed file with a blob on neither side: the core lists a submodule's commits that way. */
function isSubmodule(file: ChangedFile): boolean {
  return file.old_blob === null && file.new_blob === null;
}
</script>

<template>
  <div
    data-testid="diff-viewer"
    class="text-sm"
  >
    <p
      v-if="!currentFile"
      class="text-muted-foreground"
    >
      Select a file to see its changes.
    </p>
    <p
      v-else-if="isSubmodule(currentFile)"
      class="text-muted-foreground"
    >
      Submodule change not shown.
    </p>
    <div
      v-else-if="error"
      class="space-y-2"
    >
      <p
        role="alert"
        class="break-words text-destructive"
      >
        {{ error }}
      </p>
      <Button
        variant="outline"
        size="xs"
        @click="load()"
      >
        Retry
      </Button>
    </div>
    <p
      v-else-if="!diff"
      class="text-muted-foreground"
    >
      Loading…
    </p>
    <p
      v-else-if="diff.kind === 'binary'"
      class="text-muted-foreground"
    >
      Binary file not shown.
    </p>
    <p
      v-else-if="diff.kind === 'too_large'"
      class="text-muted-foreground"
    >
      Too large to show ({{ mebibytes(diff.bytes) }}).
    </p>
    <template v-else>
      <p
        v-if="diff.hunks.length === 0 && currentFile.old_blob && currentFile.new_blob"
        data-testid="diff-note"
        class="mb-2 text-muted-foreground"
      >
        Contents unchanged.
      </p>
      <div
        ref="host"
        class="overflow-hidden rounded-md border font-mono text-xs"
      />
    </template>
  </div>
</template>
