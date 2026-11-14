import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { messageOf } from "@/lib/ipc";

/** An open PR in an inbox section, as the Rust core reports it. */
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

/** An inbox section the Rust core can search, by its snake-case name. */
export type Section = "review_requested" | "assigned" | "authored" | "mentioned";

type SectionPulls = { pulls: PullSummary[]; failures: HostFailure[] };

/** The inbox's Review requested section. */
export const useInboxStore = defineStore("inbox", () => {
  const pulls = ref<PullSummary[]>([]);
  const failures = ref<HostFailure[]>([]);
  const error = ref<string | null>(null);
  const loading = ref(false);
  /** Whether any repo is watched; `null` until the first load. */
  const hasWatchedRepos = ref<boolean | null>(null);

  /** Counts loads, so only the latest one started may write its answer. */
  let latest = 0;

  /**
   * Reads the watched repos, then, if there are any, the forge's review
   * requests. A load can take a while (each host has its own timeout), so
   * if another starts meanwhile, the older one's answer is dropped.
   */
  async function load() {
    const id = ++latest;
    const current = () => id === latest;
    loading.value = true;
    try {
      const watched = await invoke<unknown[]>("list_watched_repos");
      if (!current()) return;
      hasWatchedRepos.value = watched.length > 0;
      const found = hasWatchedRepos.value
        ? await invoke<SectionPulls>("list_section", { section: "review_requested" satisfies Section })
        : { pulls: [], failures: [] };
      if (!current()) return;
      pulls.value = found.pulls;
      failures.value = found.failures;
      error.value = null;
    } catch (err) {
      if (!current()) return;
      // Rows from an earlier load may no longer be right, so don't keep them.
      pulls.value = [];
      failures.value = [];
      error.value = messageOf(err);
    } finally {
      if (current()) loading.value = false;
    }
  }

  return { pulls, failures, error, hasWatchedRepos, loading, load };
});
