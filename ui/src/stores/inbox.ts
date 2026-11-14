import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { messageOf } from "@/lib/ipc";

/** An open PR requesting the user's review, as the Rust core reports it. */
export type PullSummary = {
  host: string;
  owner: string;
  repo: string;
  number: number;
  title: string;
  author: string;
  /** When the PR was opened, as the forge's RFC 3339 timestamp. */
  created_at: string;
};

/** A host that couldn't be asked, with the core's `{ kind, message }`. */
export type HostFailure = { host: string; error: { kind: string; message: string } };

type ReviewRequested = { pulls: PullSummary[]; failures: HostFailure[] };

/** The inbox's Review requested section. */
export const useInboxStore = defineStore("inbox", () => {
  const pulls = ref<PullSummary[]>([]);
  const failures = ref<HostFailure[]>([]);
  const error = ref<string | null>(null);
  const loading = ref(false);
  /** Whether any repo is watched; `null` until the first load. */
  const hasWatchedRepos = ref<boolean | null>(null);

  /** Reads the watched repos, then, if there are any, the forge's review requests. */
  async function load() {
    loading.value = true;
    try {
      const watched = await invoke<unknown[]>("list_watched_repos");
      hasWatchedRepos.value = watched.length > 0;
      if (!hasWatchedRepos.value) {
        pulls.value = [];
        failures.value = [];
        error.value = null;
        return;
      }
      const found = await invoke<ReviewRequested>("list_review_requested");
      pulls.value = found.pulls;
      failures.value = found.failures;
      error.value = null;
    } catch (err) {
      error.value = messageOf(err);
    } finally {
      loading.value = false;
    }
  }

  return { pulls, failures, error, hasWatchedRepos, loading, load };
});
