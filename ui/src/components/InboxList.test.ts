import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { createPinia } from "pinia";
import { defineComponent, h } from "vue";
import InboxList from "./InboxList.vue";
import { TooltipProvider } from "@/components/ui/tooltip";
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
  details?: Details | null;
};
type Details = {
  ci: string;
  review: string;
  additions: number;
  deletions: number;
  changed_files: number;
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
function mockCore(state: {
  pulls?: Pull[];
  failures?: Failure[];
  incomplete?: string[];
  repos?: Repo[];
  fail?: Failure["error"];
}) {
  const calls: string[] = [];
  mockIPC((cmd, args) => {
    calls.push(cmd);
    switch (cmd) {
      case "list_section":
        // These cases are about Review requested; the other sections stay empty.
        if ((args as { section: string }).section !== "review_requested")
          return { pulls: [], failures: [], incomplete: [] };
        if (state.fail) throw state.fail;
        return { pulls: state.pulls ?? [], failures: state.failures ?? [], incomplete: state.incomplete ?? [] };
      case "list_watched_repos":
        return state.repos ?? [];
      default:
        throw new Error(`unexpected command: ${cmd}`);
    }
  });
  return calls;
}

async function mountList() {
  // The app provides tooltips from its root.
  const Root = defineComponent(() => () => h(TooltipProvider, () => h(InboxList)));
  const wrapper = mount(Root, { global: { plugins: [createPinia()] } });
  await flushPromises();
  return wrapper;
}

function rows(wrapper: Awaited<ReturnType<typeof mountList>>) {
  return wrapper.findAll("[data-testid=inbox-row]");
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ["Date"] });
  vi.setSystemTime(new Date("2026-11-13T12:00:00Z"));
});

afterEach(() => {
  vi.useRealTimers();
  clearMocks();
});

describe("InboxList", () => {
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

  it("shows each PR's CI status, review state and size", async () => {
    const details = { ci: "failing", review: "changes_requested", additions: 120, deletions: 30, changed_files: 4 };
    mockCore({ repos: [watchedRepo], pulls: [pull({ details })] });

    const wrapper = await mountList();

    const row = rows(wrapper)[0];
    expect(row.get("[data-testid=row-ci]").text()).toBe("Checks failing");
    expect(row.get("[data-testid=row-review]").text()).toBe("Changes requested");
    expect(row.get("[data-testid=row-size]").text()).toBe("M · 150 lines");
  });

  it.each([
    ["the CI icon", "row-ci", "Checks failing"],
    ["the size", "row-size", "+120 −30 · 4 files"],
  ])("explains %s in a tooltip on hover", async (_, testid, tooltip) => {
    const details = { ci: "failing", review: "approved", additions: 120, deletions: 30, changed_files: 4 };
    mockCore({ repos: [watchedRepo], pulls: [pull({ details })] });
    const wrapper = await mountList();

    await rows(wrapper)[0].get(`[data-testid=${testid}]`).trigger("pointermove");
    await flushPromises();

    expect(document.body.querySelector("[role=tooltip]")?.textContent).toBe(tooltip);
    wrapper.unmount();
  });

  it.each([
    [0, 1, "XS · 1 line"],
    [5, 4, "XS · 9 lines"],
    [10, 0, "S · 10 lines"],
    [60, 39, "S · 99 lines"],
    [100, 0, "M · 100 lines"],
    [400, 99, "M · 499 lines"],
    [250, 250, "L · 500 lines"],
    [1000, 210, "L · 1,210 lines"],
  ])("sizes +%i −%i as %s", async (additions, deletions, label) => {
    const details = { ci: "passing", review: "approved", additions, deletions, changed_files: 1 };
    mockCore({ repos: [watchedRepo], pulls: [pull({ details })] });

    const wrapper = await mountList();

    expect(rows(wrapper)[0].get("[data-testid=row-size]").text()).toBe(label);
  });

  it("says when a PR has no checks and no reviews", async () => {
    const details = { ci: "no_checks", review: "no_reviews", additions: 3, deletions: 0, changed_files: 1 };
    mockCore({ repos: [watchedRepo], pulls: [pull({ details })] });

    const wrapper = await mountList();

    const row = rows(wrapper)[0];
    expect(row.get("[data-testid=row-ci]").text()).toBe("No checks");
    expect(row.get("[data-testid=row-review]").text()).toBe("No reviews");
    expect(row.get("[data-testid=row-size]").text()).toBe("XS · 3 lines");
  });

  it("still shows a row whose details the forge couldn't give", async () => {
    mockCore({ repos: [watchedRepo], pulls: [pull({ details: null })] });

    const wrapper = await mountList();

    const row = rows(wrapper)[0];
    expect(row.text()).toContain("Fix the greeting");
    expect(row.find("[data-testid=row-ci]").exists()).toBe(false);
    expect(row.find("[data-testid=row-review]").exists()).toBe(false);
    expect(row.find("[data-testid=row-size]").exists()).toBe(false);
  });

  it("asks for a watched repo when none is watched, without asking the forge", async () => {
    const calls = mockCore({ repos: [] });

    const wrapper = await mountList();

    expect(wrapper.text()).toContain("Watch a repo in the drawer");
    expect(calls).not.toContain("list_section");
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

  it("quietly notes a host whose results were cut short, without raising an alert", async () => {
    mockCore({ repos: [watchedRepo], pulls: [pull()], incomplete: ["github.com"] });

    const wrapper = await mountList();

    expect(rows(wrapper)).toHaveLength(1);
    expect(wrapper.find("[role=alert]").exists()).toBe(false);
    expect(wrapper.get("[data-testid=inbox-incomplete]").text()).toBe(
      "github.com didn't return all its results, so some PRs may be missing.",
    );
  });

  it("doesn't claim a section is empty when a host's results were cut short", async () => {
    mockCore({ repos: [watchedRepo], pulls: [], incomplete: ["github.com"] });

    const wrapper = await mountList();

    expect(wrapper.text()).not.toContain("No review requests");
    expect(wrapper.find("[data-testid=inbox-incomplete]").exists()).toBe(true);
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
    await wrapper.get("button[aria-label='Refresh inbox']").trigger("click");
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
        case "list_section":
          return { pulls: [pull()], failures: [] };
        default:
          throw new Error(`unexpected command: ${cmd}`);
      }
    });
    const pinia = createPinia();
    const list = mount(InboxList, { global: { plugins: [pinia] } });
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
    mockIPC((cmd, args) => {
      if (cmd === "list_watched_repos") return [watchedRepo];
      if ((args as { section: string }).section !== "review_requested") return { pulls: [], failures: [] };
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
    await wrapper.get("button[aria-label='Refresh inbox']").trigger("click");
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
