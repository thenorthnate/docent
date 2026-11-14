import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { messageOf } from "@/lib/ipc";
import { SECTIONS, type Section } from "@/lib/sections";

export type { Section };

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

function sectionState(answer: Partial<SectionState> = {}): SectionState {
  return { pulls: [], failures: [], error: null, loaded: false, ...answer };
}

/** The inbox's sections, and which one the list shows. */
export const useInboxStore = defineStore("inbox", () => {
  const sections = ref(Object.fromEntries(SECTIONS.map((s) => [s, sectionState()])) as Record<Section, SectionState>);
  const selected = ref<Section>("review_requested");
  const loading = ref(false);
  /** Whether any repo is watched; `null` until the first load. */
  const hasWatchedRepos = ref<boolean | null>(null);

  const selectedSection = computed(() => sections.value[selected.value]);

  /** Counts loads, so only the latest one started may write its answer. */
  let latest = 0;

  /**
   * Reads the watched repos, then, if there are any, every section at once,
   * so each count is current. Each section shows as soon as it answers, since
   * a slow host can take its whole timeout. If another load starts meanwhile,
   * the older one's answers are dropped.
   */
  async function load() {
    const id = ++latest;
    const isLatest = () => id === latest;
    // Rows from an earlier load may no longer be right, so a failure clears them.
    const write = (section: Section, answer: Partial<SectionState>) => {
      if (isLatest()) sections.value[section] = sectionState({ ...answer, loaded: true });
    };
    loading.value = true;
    try {
      const watched = await invoke<unknown[]>("list_watched_repos");
      if (!isLatest()) return;
      hasWatchedRepos.value = watched.length > 0;
      if (!hasWatchedRepos.value) {
        SECTIONS.forEach((section) => write(section, {}));
        return;
      }
      await Promise.all(
        SECTIONS.map((section) =>
          invoke<SectionPulls>("list_section", { section }).then(
            (found) => write(section, found),
            (err: unknown) => write(section, { error: messageOf(err) }),
          ),
        ),
      );
    } catch (err) {
      const error = messageOf(err);
      SECTIONS.forEach((section) => write(section, { error }));
    } finally {
      if (isLatest()) loading.value = false;
    }
  }

  function select(section: Section) {
    selected.value = section;
  }

  return { sections, selected, selectedSection, hasWatchedRepos, loading, load, select };
});
