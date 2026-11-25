import { defineStore } from "pinia";
import { computed, ref, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { messageOf } from "@/lib/ipc";
import type { ChangedFile } from "@/lib/review";

/** The PR being reviewed, and the commits its diff compares. */
export type PullRange = {
  host: string;
  owner: string;
  repo: string;
  number: number;
  base_sha: string;
  head_sha: string;
};

/** The open PR's changed files, and the one being viewed. */
export const useReviewStore = defineStore("review", () => {
  const range = ref<PullRange | null>(null);
  /** `null` until they've loaded. */
  const files = shallowRef<ChangedFile[] | null>(null);
  const error = ref<string | null>(null);
  /** Held by identity: two names that aren't UTF-8 can read the same. */
  const current = shallowRef<ChangedFile | null>(null);
  const currentFile = computed(() => (current.value && files.value?.includes(current.value) ? current.value : null));

  /** Counts loads, so only the latest one started may write its answer. */
  let latest = 0;

  /** Reads the files in the current range. */
  async function load() {
    const id = ++latest;
    files.value = null;
    error.value = null;
    if (!range.value) return;
    try {
      const found = await invoke<ChangedFile[]>("changed_files", { range: range.value });
      if (id === latest) files.value = found;
    } catch (err) {
      if (id === latest) error.value = messageOf(err);
    }
  }

  /** Switches to `next`, or to nothing, forgetting the last range's files and selection. */
  async function show(next: PullRange | null) {
    range.value = next;
    current.value = null;
    await load();
  }

  function select(file: ChangedFile) {
    current.value = file;
  }

  return { range, files, error, currentFile, load, show, select };
});
