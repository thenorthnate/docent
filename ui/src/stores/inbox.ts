import { defineStore } from "pinia";
import { computed, ref } from "vue";
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

/** The sections, in the order the drawer lists them. */
export const SECTIONS: readonly Section[] = ["review_requested", "assigned", "authored", "mentioned"];

/**
 * One section's last answer. `loaded` is false until a load has answered for
 * it, and `error` is set when the whole section couldn't be read.
 */
export type SectionState = {
  pulls: PullSummary[];
  failures: HostFailure[];
  error: string | null;
  loaded: boolean;
};

type SectionPulls = { pulls: PullSummary[]; failures: HostFailure[] };

function emptySection(): SectionState {
  return { pulls: [], failures: [], error: null, loaded: false };
}

/** The inbox's sections, and which one the list shows. */
export const useInboxStore = defineStore("inbox", () => {
  const sections = ref<Record<Section, SectionState>>({
    review_requested: emptySection(),
    assigned: emptySection(),
    authored: emptySection(),
    mentioned: emptySection(),
  });
  const selected = ref<Section>("review_requested");
  const loading = ref(false);
  /** Whether any repo is watched; `null` until the first load. */
  const hasWatchedRepos = ref<boolean | null>(null);

  const current = computed(() => sections.value[selected.value]);

  /** Counts loads, so only the latest one started may write its answer. */
  let latest = 0;

  /** Reads one section; a failure is kept with that section, not thrown. */
  async function readSection(section: Section): Promise<SectionState> {
    try {
      const found = await invoke<SectionPulls>("list_section", { section });
      return { ...found, error: null, loaded: true };
    } catch (err) {
      // Rows from an earlier load may no longer be right, so don't keep them.
      return { pulls: [], failures: [], error: messageOf(err), loaded: true };
    }
  }

  /**
   * Reads the watched repos, then, if there are any, every section at once,
   * so each count is current. A load can take a while (each host has its own
   * timeout), so if another starts meanwhile, the older one's answer is dropped.
   */
  async function load() {
    const id = ++latest;
    const isLatest = () => id === latest;
    loading.value = true;
    try {
      let answers: SectionState[];
      try {
        const watched = await invoke<unknown[]>("list_watched_repos");
        if (!isLatest()) return;
        hasWatchedRepos.value = watched.length > 0;
        answers = hasWatchedRepos.value
          ? await Promise.all(SECTIONS.map(readSection))
          : SECTIONS.map(() => ({ ...emptySection(), loaded: true }));
      } catch (err) {
        const error = messageOf(err);
        answers = SECTIONS.map(() => ({ ...emptySection(), error, loaded: true }));
      }
      if (!isLatest()) return;
      SECTIONS.forEach((section, i) => (sections.value[section] = answers[i]));
    } finally {
      if (isLatest()) loading.value = false;
    }
  }

  function select(section: Section) {
    selected.value = section;
  }

  return { sections, selected, current, hasWatchedRepos, loading, load, select };
});
