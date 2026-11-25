import { defineStore } from "pinia";
import { ref } from "vue";
import { Channel, invoke } from "@tauri-apps/api/core";
import { messageOf } from "@/lib/ipc";

/** How far a clone has got. `percent` is `null` until git reports one. */
export type CloneUpdate = { phase: string; percent: number | null };

/**
 * Running and failed clones, by repo id. They live in a store because a clone
 * outlasts the page it was started from.
 */
export const useClonesStore = defineStore("clones", () => {
  const running = ref<Record<number, CloneUpdate>>({});
  const errors = ref<Record<number, string>>({});
  /** Counts the clones that worked, so a page can tell when to reload. */
  const finished = ref(0);

  /** The core opens the folder picker, clones and binds; `null` means cancelled. */
  async function start(id: number) {
    if (id in running.value) return;
    running.value[id] = { phase: "Choosing a folder", percent: null };
    delete errors.value[id];
    const onProgress = new Channel<CloneUpdate>((update) => {
      // Messages can arrive after the command has answered.
      if (id in running.value) running.value[id] = update;
    });
    try {
      const bound = await invoke("clone_repo", { id, onProgress });
      if (bound) finished.value++;
    } catch (err) {
      errors.value[id] = messageOf(err);
    } finally {
      delete running.value[id];
    }
  }

  return { running, errors, finished, start };
});
