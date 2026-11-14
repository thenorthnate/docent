import { afterEach, describe, expect, it } from "vitest";
import { flushPromises } from "@vue/test-utils";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { createPinia, setActivePinia } from "pinia";
import { useInboxStore } from "./inbox";

const watchedRepo = { id: 1, host: "github.com", owner: "octocat", name: "Hello-World" };
const requested = {
  host: "github.com",
  owner: "octocat",
  repo: "Hello-World",
  number: 1,
  title: "Requested",
  author: "mona",
  created_at: "2026-11-10T12:00:00Z",
};

afterEach(() => {
  clearMocks();
});

describe("inbox store", () => {
  it("shows each section as soon as it answers, not when the slowest one does", async () => {
    let releaseAssigned: (value: unknown) => void = () => {};
    mockIPC((cmd, args) => {
      if (cmd === "list_watched_repos") return [watchedRepo];
      switch ((args as { section: string }).section) {
        case "review_requested":
          return { pulls: [requested], failures: [] };
        case "assigned":
          // A host that takes its whole timeout to answer.
          return new Promise((resolve) => (releaseAssigned = resolve));
        default:
          return { pulls: [], failures: [] };
      }
    });
    setActivePinia(createPinia());
    const inbox = useInboxStore();

    const load = inbox.load();
    await flushPromises();

    expect(inbox.selectedSection.pulls).toEqual([requested]);
    expect(inbox.sections.assigned.loaded).toBe(false);
    expect(inbox.loading).toBe(true);

    releaseAssigned({ pulls: [requested], failures: [] });
    await load;

    expect(inbox.sections.assigned.pulls).toEqual([requested]);
    expect(inbox.loading).toBe(false);
  });
});
