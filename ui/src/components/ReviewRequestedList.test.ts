import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { createPinia } from "pinia";
import ReviewRequestedList from "./ReviewRequestedList.vue";
import WatchedRepos from "./WatchedRepos.vue";
import { useInboxStore } from "@/stores/inbox";

type Pull = {
  host: string;
  owner: string;
  repo: string;
  number: number;
  title: string;
  author: string;
  created_at: string;
};
type Failure = { host: string; error: { kind: string; message: string } };
type Repo = { id: number; host: string; owner: string; name: string; default_branch: string; local_path: null };

const watchedRepo: Repo = {
  id: 1,
  host: "github.com",
  owner: "octocat",
  name: "Hello-World",
  default_branch: "main",
  local_path: null,
};

function pull(overrides: Partial<Pull> = {}): Pull {
  return {
    host: "github.com",
    owner: "octocat",
    repo: "Hello-World",
    number: 1347,
    title: "Fix the greeting",
    author: "mona",
    created_at: "2026-11-10T12:00:00Z",
    ...overrides,
  };
}

/** A fake Rust core whose answers the test can change between calls. */
function mockCore(state: { pulls?: Pull[]; failures?: Failure[]; repos?: Repo[]; fail?: Failure["error"] }) {
  const calls: string[] = [];
  mockIPC((cmd) => {
    calls.push(cmd);
    switch (cmd) {
      case "list_review_requested":
        if (state.fail) throw state.fail;
        return { pulls: state.pulls ?? [], failures: state.failures ?? [] };
      case "list_watched_repos":
        return state.repos ?? [];
      default:
        throw new Error(`unexpected command: ${cmd}`);
    }
  });
  return calls;
}

async function mountList() {
  const wrapper = mount(ReviewRequestedList, { global: { plugins: [createPinia()] } });
  await flushPromises();
  return wrapper;
}

function rows(wrapper: Awaited<ReturnType<typeof mountList>>) {
  return wrapper.findAll("[data-testid=review-requested-row]");
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ["Date"] });
  vi.setSystemTime(new Date("2026-11-13T12:00:00Z"));
});

afterEach(() => {
  vi.useRealTimers();
  clearMocks();
});

describe("ReviewRequestedList", () => {
  it("shows each PR's title, repo, author and age", async () => {
    mockCore({ repos: [watchedRepo], pulls: [pull()] });

    const wrapper = await mountList();

    expect(rows(wrapper)).toHaveLength(1);
    const row = rows(wrapper)[0];
    expect(row.text()).toContain("Fix the greeting");
    expect(row.text()).toContain("octocat/Hello-World");
    expect(row.text()).toContain("mona");
    expect(row.text()).toContain("3d");
  });

  it("asks for a watched repo when none is watched, without asking the forge", async () => {
    const calls = mockCore({ repos: [] });

    const wrapper = await mountList();

    expect(wrapper.text()).toContain("Watch a repo in the drawer");
    expect(calls).not.toContain("list_review_requested");
  });

  it("says nothing is waiting when watched repos have no review requests", async () => {
    mockCore({ repos: [watchedRepo], pulls: [] });

    const wrapper = await mountList();

    expect(rows(wrapper)).toHaveLength(0);
    expect(wrapper.text()).toContain("No review requests");
  });

  it("names each host that couldn't be asked and still shows the others' PRs", async () => {
    mockCore({
      repos: [watchedRepo],
      pulls: [pull()],
      failures: [
        { host: "ghe.corp.test", error: { kind: "unreachable", message: "Could not reach ghe.corp.test: timed out" } },
      ],
    });

    const wrapper = await mountList();

    expect(rows(wrapper)).toHaveLength(1);
    expect(wrapper.get("[role=alert]").text()).toContain("Could not reach ghe.corp.test: timed out");
    expect(wrapper.text()).not.toContain("No review requests");
  });

  it("shows the core's message when the list can't be read at all", async () => {
    mockCore({ repos: [watchedRepo], fail: { kind: "database", message: "Docent's database failed: locked" } });

    const wrapper = await mountList();

    expect(wrapper.get("[role=alert]").text()).toBe("Docent's database failed: locked");
    expect(wrapper.text()).not.toContain("No review requests");
  });

  it("asks again on Refresh", async () => {
    const state = { repos: [watchedRepo], pulls: [pull()] };
    mockCore(state);
    const wrapper = await mountList();

    state.pulls = [pull(), pull({ number: 7, title: "Another one" })];
    await wrapper.get("button[aria-label='Refresh review requests']").trigger("click");
    await flushPromises();

    expect(rows(wrapper).map((r) => r.text())).toEqual([
      expect.stringContaining("Fix the greeting"),
      expect.stringContaining("Another one"),
    ]);
  });

  it.each([
    ["watching", "watch"],
    ["unwatching", "unwatch"],
  ])("reloads after %s a repo in the drawer", async (_case, action) => {
    const repos: Repo[] = action === "watch" ? [] : [watchedRepo];
    mockIPC((cmd) => {
      switch (cmd) {
        case "list_accounts":
          return [{ host: "github.com", login: "octocat", error: null }];
        case "list_watched_repos":
          return [...repos];
        case "watch_repo":
          repos.push(watchedRepo);
          return watchedRepo;
        case "unwatch_repo":
          repos.pop();
          return null;
        case "list_review_requested":
          return { pulls: [pull()], failures: [] };
        default:
          throw new Error(`unexpected command: ${cmd}`);
      }
    });
    const pinia = createPinia();
    const list = mount(ReviewRequestedList, { global: { plugins: [pinia] } });
    const drawer = mount(WatchedRepos, { global: { plugins: [pinia] } });
    await flushPromises();
    expect(rows(list)).toHaveLength(action === "watch" ? 0 : 1);

    if (action === "watch") {
      await drawer.get("input[name=repo]").setValue("octocat/Hello-World");
      await drawer.get("form").trigger("submit");
    } else {
      await drawer.get("button[aria-label='Unwatch octocat/Hello-World']").trigger("click");
    }
    await flushPromises();

    expect(rows(list)).toHaveLength(action === "watch" ? 1 : 0);
  });

  it("keeps the newest answer when an older, slower load finishes last", async () => {
    let releaseSlow: (value: unknown) => void = () => {};
    let call = 0;
    mockIPC((cmd) => {
      if (cmd === "list_watched_repos") return [watchedRepo];
      call += 1;
      if (call === 1) return { pulls: [], failures: [] };
      if (call === 2) return new Promise((resolve) => (releaseSlow = resolve));
      return { pulls: [pull({ title: "Newest" })], failures: [] };
    });
    const wrapper = await mountList();
    const refresh = () => useInboxStore().load();

    const slow = refresh();
    await flushPromises();
    await refresh();
    releaseSlow({ pulls: [pull({ title: "Stale" })], failures: [] });
    await slow;
    await flushPromises();

    expect(rows(wrapper).map((r) => r.text())).toEqual([expect.stringContaining("Newest")]);
    expect(useInboxStore().loading).toBe(false);
  });

  it("drops the old rows when a reload fails, rather than showing them as current", async () => {
    const state: Parameters<typeof mockCore>[0] = { repos: [watchedRepo], pulls: [pull()] };
    mockCore(state);
    const wrapper = await mountList();

    state.fail = { kind: "database", message: "locked" };
    await wrapper.get("button[aria-label='Refresh review requests']").trigger("click");
    await flushPromises();

    expect(rows(wrapper)).toHaveLength(0);
    expect(wrapper.get("[role=alert]").text()).toBe("locked");
  });

  it("names each row's host when the PRs span hosts", async () => {
    mockCore({
      repos: [watchedRepo],
      pulls: [pull(), pull({ host: "ghe.corp.test", number: 2 })],
    });

    const wrapper = await mountList();

    expect(rows(wrapper).map((r) => r.text())).toEqual([
      expect.stringContaining("github.com"),
      expect.stringContaining("ghe.corp.test"),
    ]);
  });
});
