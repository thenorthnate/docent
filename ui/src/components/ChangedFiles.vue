<script setup lang="ts">
import { computed } from "vue";
import { storeToRefs } from "pinia";
import { Button } from "@/components/ui/button";
import { fileTreeRows } from "@/lib/fileTree";
import { FILE_STATUSES, type ChangedFile } from "@/lib/review";
import { useReviewStore } from "@/stores/review";

const review = useReviewStore();
const { range, files, error, currentFile } = storeToRefs(review);

const rows = computed(() => fileTreeRows(files.value ?? []));

/** Indents a row, up to a limit: a PR can nest folders deeper than the drawer is wide. */
function indent(depth: number) {
  return { paddingLeft: `${0.5 + Math.min(depth, 8) * 0.75}rem` };
}

function describe(file: ChangedFile): string {
  const path = file.old_path === null ? file.path : `${file.old_path} → ${file.path}`;
  return `${FILE_STATUSES[file.status].label}: ${path}`;
}
</script>

<template>
  <section
    aria-labelledby="changed-files-heading"
    class="space-y-1"
  >
    <h3
      id="changed-files-heading"
      class="text-xs font-semibold tracking-wide text-muted-foreground uppercase"
    >
      Files changed
      <span
        v-if="files"
        class="tabular-nums"
      >{{ files.length }}</span>
    </h3>
    <div
      v-if="error"
      class="space-y-2 text-sm"
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
        @click="review.load()"
      >
        Retry
      </Button>
    </div>
    <p
      v-else-if="!files"
      class="text-sm text-muted-foreground"
    >
      {{ range ? "Loading…" : "Files show once the worktree is ready." }}
    </p>
    <p
      v-else-if="files.length === 0"
      class="text-sm text-muted-foreground"
    >
      No files changed.
    </p>
    <ul
      v-else
      class="space-y-0.5 text-sm"
    >
      <li
        v-for="row in rows"
        :key="row.key"
      >
        <div
          v-if="!row.file"
          data-testid="file-folder"
          class="truncate py-0.5 text-muted-foreground"
          :style="indent(row.depth)"
          :title="row.key"
        >
          {{ row.name }}
        </div>
        <button
          v-else
          type="button"
          :aria-current="row.file === currentFile"
          :title="describe(row.file)"
          class="w-full rounded-md py-0.5 pr-2 text-left hover:bg-accent aria-[current=true]:bg-accent aria-[current=true]:font-medium"
          :style="indent(row.depth)"
          @click="review.select(row.file.path)"
        >
          <span class="flex items-center gap-2">
            <span
              data-testid="file-status"
              class="shrink-0 font-mono text-xs"
              :class="FILE_STATUSES[row.file.status].tone"
            >{{ FILE_STATUSES[row.file.status].letter }}</span>
            <span
              data-testid="file-name"
              class="min-w-0 flex-1 truncate"
            >{{ row.name }}</span>
            <span
              data-testid="file-lines"
              class="shrink-0 text-xs text-muted-foreground tabular-nums"
            >{{ row.file.lines ? `+${row.file.lines.added} −${row.file.lines.deleted}` : "binary" }}</span>
          </span>
          <span
            v-if="row.file.old_path !== null"
            class="block truncate pl-4 text-xs text-muted-foreground"
          >from {{ row.file.old_path }}</span>
        </button>
      </li>
    </ul>
  </section>
</template>
