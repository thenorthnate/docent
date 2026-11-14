import { afterEach, describe, expect, it } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { createPinia } from "pinia";
import InboxList from "./InboxList.vue";
import InboxSections from "./InboxSections.vue";
import WatchedRepos from "./WatchedRepos.vue";
import { SECTIONS, type Section } from "@/lib/sections";
import { useInboxStore, type HostFailure, type PullSummary } from "@/stores/inbox";

type Failure = HostFailure["error"];
type SectionState = { pulls?: PullSummary[]; failures?: HostFailure[]; fail?: Failure };

const watchedRepo = {
  id: 1,
  host: "github.com",
  owner: "octocat",
  name: "Hello-World",
  default_branch: "main",
  local_path: null,
};

function pull(title: string, number = 1): PullSummary {
  return {
    host: "github.com",
    owner: "octocat",
    repo: "Hello-World",
    number,
    title,
    author: "mona",
    created_at: "2026-11-10T12:00:00Z",
  };
}

/** A fake Rust core answering each section from `sections`, which the test can change. */
function mockCore(sections: Partial<Record<Section, SectionState>>, repos: unknown[] = [watchedRepo]) {
  mockIPC((cmd, args) => {
    switch (cmd) {
      case "list_watched_repos":
        return [...repos];
      case "list_section": {
        const state = sections[(args as { section: Section }).section] ?? {};
        if (state.fail) throw state.fail;
        return { pulls: state.pulls ?? [], failures: state.failures ?? [] };
      }
      default:
        throw new Error(`unexpected command: ${cmd}`);
    }
  });
}

/** The drawer and the list side by side, sharing one store, as in the app. */
async function mountInbox() {
  const pinia = createPinia();
  const drawer = mount(InboxSections, { global: { plugins: [pinia] } });
  const list = mount(InboxList, { global: { plugins: [pinia] } });
  await flushPromises();
  return { drawer, list };
}

type Drawer = Awaited<ReturnType<typeof mountInbox>>["drawer"];

function sections(drawer: Drawer) {
  return drawer
    .findAll("[data-testid=inbox-section]")
    .map((s) => s.findAll("span").map((part) => part.text()).join(" "));
}

async function select(drawer: Drawer, label: string) {
  const button = drawer.findAll("[data-testid=inbox-section]").find((s) => s.text().startsWith(label));
  await button?.trigger("click");
  await flushPromises();
}

function rows(list: Awaited<ReturnType<typeof mountInbox>>["list"]) {
  return list.findAll("[data-testid=inbox-row]");
}

afterEach(() => {
  clearMocks();
});

describe("InboxSections", () => {
  it("lists the four sections, each with its count, with Review requested selected", async () => {
    mockCore({
      review_requested: { pulls: [pull("a"), pull("b", 2)] },
      assigned: { pulls: [pull("c")] },
      authored: { pulls: [] },
      mentioned: { pulls: [pull("d"), pull("e", 2), pull("f", 3)] },
    });

    const { drawer } = await mountInbox();

    expect(sections(drawer)).toEqual(["Review requested 2", "Assigned 1", "Authored by me 0", "Mentioned 3"]);
    expect(drawer.get("[aria-current=true]").text()).toContain("Review requested");
  });

  it("shows the selected section's PRs in the list", async () => {
    mockCore({
      review_requested: { pulls: [pull("Requested")] },
      mentioned: { pulls: [pull("Mentions you")] },
    });
    const { drawer, list } = await mountInbox();
    expect(list.get("h2").text()).toBe("Review requested");
    expect(list.text()).toContain("Requested");

    await select(drawer, "Mentioned");

    expect(drawer.get("[aria-current=true]").text()).toContain("Mentioned");
    expect(list.get("h2").text()).toBe("Mentioned");
    expect(rows(list).map((r) => r.text())).toEqual([expect.stringContaining("Mentions you")]);
  });

  it.each([
    ["Assigned", "No PRs assigned to you"],
    ["Authored by me", "You have no open PRs"],
    ["Mentioned", "No PRs mention you"],
  ])("says when %s is empty", async (label, empty) => {
    mockCore({ review_requested: { pulls: [pull("Requested")] } });
    const { drawer, list } = await mountInbox();

    await select(drawer, label);

    expect(rows(list)).toHaveLength(0);
    expect(list.text()).toContain(empty);
    expect(list.text()).not.toContain("No review requests");
  });

  it("asks for a watched repo in every section, without naming review requests", async () => {
    mockCore({}, []);
    const { drawer, list } = await mountInbox();

    await select(drawer, "Assigned");

    expect(list.text()).toContain("Watch a repo in the drawer");
    expect(list.text()).not.toContain("review requests");
  });

  it("shows a section's host failures, and doesn't claim it is empty", async () => {
    mockCore({
      authored: {
        pulls: [pull("Mine")],
        failures: [{ host: "ghe.corp.test", error: { kind: "unreachable", message: "Could not reach ghe.corp.test" } }],
      },
    });
    const { drawer, list } = await mountInbox();
    expect(list.find("[role=alert]").exists()).toBe(false);

    await select(drawer, "Authored by me");

    expect(rows(list)).toHaveLength(1);
    expect(list.get("[role=alert]").text()).toContain("Could not reach ghe.corp.test");
  });

  it("marks a count as a lower bound when some hosts couldn't be asked", async () => {
    mockCore({
      authored: {
        pulls: [pull("Mine")],
        failures: [{ host: "ghe.corp.test", error: { kind: "unreachable", message: "Could not reach ghe.corp.test" } }],
      },
    });

    const { drawer } = await mountInbox();

    expect(sections(drawer)).toEqual(["Review requested 0", "Assigned 0", "Authored by me 1+", "Mentioned 0"]);
    const authored = drawer.findAll("[data-testid=inbox-section]")[2];
    expect(authored.get("[title]").attributes("title")).toContain("Not every host answered");
  });

  it("shows no count for a section that couldn't be read, and its message when selected", async () => {
    mockCore({
      review_requested: { pulls: [pull("Requested")] },
      assigned: { fail: { kind: "database", message: "locked" } },
    });
    const { drawer, list } = await mountInbox();

    expect(sections(drawer)).toEqual(["Review requested 1", "Assigned", "Authored by me 0", "Mentioned 0"]);
    await select(drawer, "Assigned");
    expect(list.get("[role=alert]").text()).toBe("locked");
    expect(list.text()).not.toContain("No PRs assigned to you");
  });

  it.each([
    ["watching", "watch"],
    ["unwatching", "unwatch"],
  ])("updates every count after %s a repo", async (_case, action) => {
    const repos: unknown[] = action === "watch" ? [] : [watchedRepo];
    mockIPC((cmd, args) => {
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
        case "list_section": {
          // The real core answers nothing while no repo is watched.
          if (!repos.length) return { pulls: [], failures: [] };
          const n = SECTIONS.indexOf((args as { section: Section }).section) + 1;
          return { pulls: Array.from({ length: n }, (_, i) => pull(`PR ${i}`, i)), failures: [] };
        }
        default:
          throw new Error(`unexpected command: ${cmd}`);
      }
    });
    const pinia = createPinia();
    const drawer = mount(InboxSections, { global: { plugins: [pinia] } });
    const watched = mount(WatchedRepos, { global: { plugins: [pinia] } });
    await useInboxStore(pinia).load();
    await flushPromises();
    const watchingCounts = ["Review requested 1", "Assigned 2", "Authored by me 3", "Mentioned 4"];
    const emptyCounts = ["Review requested 0", "Assigned 0", "Authored by me 0", "Mentioned 0"];
    expect(sections(drawer)).toEqual(action === "watch" ? emptyCounts : watchingCounts);

    if (action === "watch") {
      await watched.get("input[name=repo]").setValue("octocat/Hello-World");
      await watched.get("form").trigger("submit");
    } else {
      await watched.get("button[aria-label='Unwatch octocat/Hello-World']").trigger("click");
    }
    await flushPromises();

    expect(sections(drawer)).toEqual(action === "watch" ? watchingCounts : emptyCounts);
  });
});
