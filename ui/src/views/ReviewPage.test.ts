import { afterEach, describe, expect, it } from "vitest";
import { enableAutoUnmount, flushPromises, mount } from "@vue/test-utils";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { createMemoryHistory } from "vue-router";
import ReviewPage from "./ReviewPage.vue";
import { createAppRouter } from "@/router";
import type { PullOverview } from "@/lib/review";

const PR_PATH = "/review/github.com/octocat/Hello-World/1347";

function overview(overrides: Partial<PullOverview> = {}): PullOverview {
  return {
    title: "Fix the greeting",
    author: "mona",
    body: "Say hello **properly**.",
    ci: "failing",
    checks: [
      { name: "build", state: "passing" },
      { name: "test", state: "failing" },
    ],
    timeline: [
      {
        kind: "commit",
        sha: "6dcb09b5b57875f334f61aebed695e2e4193db5e",
        headline: "Say hello properly",
        author: "mona",
        at: "2026-11-10T09:00:00Z",
      },
      { kind: "comment", author: "hubot", body: "Looks _close_.", at: "2026-11-11T08:00:00Z" },
      {
        kind: "review",
        author: "octocat",
        verdict: "changes_requested",
        body: "Please add a test.",
        at: "2026-11-11T09:00:00Z",
      },
    ],
    timeline_truncated: false,
    ...overrides,
  };
}

/** Answers the core's commands other than `pull_overview`. */
type Core = (cmd: string, args: unknown) => unknown;

/** By default the PR's worktree never finishes preparing. */
const preparing: Core = (cmd) => {
  if (cmd === "open_pr_worktree") return new Promise(() => {});
  throw new Error(`unexpected command: ${cmd}`);
};

/**
 * The review page at `path`, over a fake core whose `pull_overview` runs
 * `answer` and whose other commands go to `others`.
 */
async function mountPage(
  path: string,
  answer: (args: unknown) => unknown = () => overview(),
  others: Core = preparing,
) {
  const asked: unknown[] = [];
  mockIPC((cmd, args) => {
    if (cmd !== "pull_overview") return others(cmd, args);
    asked.push(args);
    return answer(args);
  });
  const router = createAppRouter(createMemoryHistory());
  await router.push(path);
  await router.isReady();
  const wrapper = mount(ReviewPage, { global: { plugins: [router] }, attachTo: document.body });
  await flushPromises();
  return { wrapper, router, asked };
}

type Page = Awaited<ReturnType<typeof mountPage>>["wrapper"];

function section(wrapper: Page, name: string) {
  return wrapper.get(`section[aria-label="${name}"]`);
}

enableAutoUnmount(afterEach);
afterEach(() => clearMocks());

describe("ReviewPage", () => {
  it("asks the core for the opened PR's overview", async () => {
    const { asked } = await mountPage(PR_PATH);

    expect(asked).toEqual([{ host: "github.com", owner: "octocat", repo: "Hello-World", number: 1347 }]);
  });

  it("asks again only when another PR is opened", async () => {
    const { router, asked } = await mountPage(PR_PATH);

    await router.push(`${PR_PATH}?tab=overview`);
    await flushPromises();
    expect(asked).toHaveLength(1);

    await router.push("/review/github.com/octocat/Hello-World/7");
    await flushPromises();
    expect(asked).toHaveLength(2);
  });

  it("names the PR in the breadcrumb and shows its CI status in the top bar", async () => {
    const { wrapper } = await mountPage(PR_PATH);

    const crumbs = wrapper.get("nav[aria-label=Breadcrumb]");
    expect(crumbs.findAll("li").map((c) => c.text())).toEqual([
      "Inbox",
      "octocat/Hello-World",
      "Fix the greeting #1347",
    ]);
    expect(wrapper.get("[data-testid=ci-status]").text()).toBe("Checks failing");
  });

  it("leads back to the inbox from the breadcrumb", async () => {
    const { wrapper, router } = await mountPage(PR_PATH);

    await wrapper.get("nav[aria-label=Breadcrumb] a").trigger("click");
    await flushPromises();

    expect(router.currentRoute.value.path).toBe("/inbox");
  });

  it("shows the description as rendered Markdown on the Overview tab", async () => {
    const { wrapper } = await mountPage(PR_PATH);

    expect(wrapper.get("[role=tab][aria-selected=true]").text()).toBe("Overview");
    expect(section(wrapper, "Description").get("strong").text()).toBe("properly");
  });

  it("never runs a script from the description", async () => {
    const body = '<script>window.pwned = true</script>\n\n<img src=x onerror="window.pwned = true">';
    const { wrapper } = await mountPage(PR_PATH, () => overview({ body }));

    const description = section(wrapper, "Description");
    expect(description.find("script, img").exists()).toBe(false);
    expect(description.text()).toContain("<script>window.pwned = true</script>");
    expect((window as { pwned?: boolean }).pwned).toBeUndefined();
  });

  it("keeps link clicks from navigating the app window", async () => {
    const { wrapper } = await mountPage(PR_PATH, () => overview({ body: "[docs](https://docs.test/a)" }));

    const link = section(wrapper, "Description").get("a");
    const click = new MouseEvent("click", { bubbles: true, cancelable: true });
    link.element.dispatchEvent(click);

    expect(click.defaultPrevented).toBe(true);
    expect(link.attributes("title")).toBe("https://docs.test/a");
  });

  it("says when there is no description or no checks", async () => {
    const { wrapper } = await mountPage(PR_PATH, () => overview({ body: null, checks: [] }));

    expect(section(wrapper, "Description").text()).toBe("No description provided.");
    expect(section(wrapper, "Checks").text()).toContain("No checks on the latest commit.");
  });

  it("lists each check with its state", async () => {
    const { wrapper } = await mountPage(PR_PATH);

    const rows = section(wrapper, "Checks").findAll("li");
    expect(rows.map((r) => r.findAll("span").map((cell) => cell.text()))).toEqual([
      ["build", "Passed"],
      ["test", "Failed"],
    ]);
  });

  it("lists commits, comments and reviews in order", async () => {
    const { wrapper } = await mountPage(PR_PATH);

    const items = section(wrapper, "Timeline").findAll("li");
    expect(items).toHaveLength(3);
    expect(items[0].text()).toContain("mona committed 6dcb09b Say hello properly");
    expect(items[1].text()).toContain("hubot commented");
    expect(items[1].get("em").text()).toBe("close");
    expect(items[2].text()).toContain("octocat requested changes");
    expect(items[2].text()).toContain("Please add a test.");
  });

  it("says when the timeline was cut short", async () => {
    const { wrapper } = await mountPage(PR_PATH, () => overview({ timeline_truncated: true }));

    expect(section(wrapper, "Timeline").text()).toContain("Showing the latest 100 events.");
  });

  it("shows why the overview couldn't load", async () => {
    const { wrapper } = await mountPage(PR_PATH, () => {
      throw { kind: "not_watched", message: "octocat/Hello-World on github.com isn't watched." };
    });

    expect(wrapper.get("[role=alert]").text()).toBe("octocat/Hello-World on github.com isn't watched.");
    expect(wrapper.get("nav[aria-label=Breadcrumb] [aria-current=page]").text()).toBe("#1347");
  });

  it("asks for a PR when none is open", async () => {
    const { wrapper, asked } = await mountPage("/review");

    expect(wrapper.find("nav[aria-label=Breadcrumb]").exists()).toBe(false);
    expect(wrapper.text()).toContain("Open a pull request from the Inbox");
    expect(asked).toEqual([]);
  });
});

describe("ReviewPage worktree", () => {
  type Failure = { kind: string; message: string };
  const repo = {
    id: 3,
    host: "github.com",
    owner: "octocat",
    name: "Hello-World",
    default_branch: "main",
    local_path: null,
  };

  let core: Core = preparing;

  /** A fake core that answers `open_pr_worktree` with each of `answers` in turn. */
  function mockCore(answers: Array<object | Failure | "never">, bind: object | null = null) {
    const calls: Array<{ cmd: string; args: unknown }> = [];
    core = (cmd, args) => {
      calls.push({ cmd, args });
      if (cmd === "open_pr_worktree") {
        const answer = answers.shift();
        if (answer === "never") return new Promise(() => {});
        if (answer && "kind" in answer) throw answer;
        return answer;
      }
      if (cmd === "bind_repo") return bind;
      throw new Error(`unexpected command: ${cmd}`);
    };
    return calls;
  }

  async function open() {
    const { wrapper } = await mountPage(PR_PATH, undefined, core);
    return wrapper.get("[data-testid=worktree]");
  }

  it("asks the core for the PR's worktree", async () => {
    const calls = mockCore([{ status: "ready", path: "/data/worktrees/3/pr-1347", head_sha: "a".repeat(40) }]);

    await open();

    expect(calls).toContainEqual({
      cmd: "open_pr_worktree",
      args: { host: "github.com", owner: "octocat", repo: "Hello-World", number: 1347 },
    });
  });

  it("says it is preparing until the worktree is ready", async () => {
    mockCore(["never"]);

    const status = await open();

    expect(status.text()).toContain("Preparing worktree");
  });

  it("shows the head the worktree is at once ready", async () => {
    mockCore([{ status: "ready", path: "/data/worktrees/3/pr-1347", head_sha: "0123456789abcdef".padEnd(40, "0") }]);

    const status = await open();

    expect(status.text()).toContain("Worktree ready at 0123456");
    expect(status.get("[title]").attributes("title")).toBe("/data/worktrees/3/pr-1347");
  });

  it("shows the core's error and retries on request", async () => {
    const calls = mockCore([
      { kind: "git_failed", message: "git failed: couldn't find remote ref" },
      { status: "ready", path: "/w", head_sha: "b".repeat(40) },
    ]);

    const status = await open();
    expect(status.get("[role=alert]").text()).toContain("couldn't find remote ref");
    await status.get("button").trigger("click");
    await flushPromises();

    expect(calls.filter((c) => c.cmd === "open_pr_worktree")).toHaveLength(2);
    expect(status.text()).toContain("Worktree ready at bbbbbbb");
  });

  it("offers Bind and Clone for an unbound repo, and opens the worktree once bound", async () => {
    const calls = mockCore(
      [
        { status: "unbound", repo },
        { status: "ready", path: "/w", head_sha: "c".repeat(40) },
      ],
      { ...repo, local_path: "/Users/me/code/hello" },
    );

    const status = await open();
    expect(status.text()).toContain("isn't bound to a local clone");
    const clone = status.findAll("button").find((b) => b.text() === "Clone");
    expect(clone?.attributes("disabled")).toBeDefined();
    await status.findAll("button").find((b) => b.text() === "Bind")!.trigger("click");
    await flushPromises();

    expect(calls).toContainEqual({ cmd: "bind_repo", args: { id: 3 } });
    expect(status.text()).toContain("Worktree ready at ccccccc");
  });

  it("stays unbound when the folder picker is cancelled", async () => {
    const calls = mockCore([{ status: "unbound", repo }], null);

    const status = await open();
    await status.findAll("button").find((b) => b.text() === "Bind")!.trigger("click");
    await flushPromises();

    expect(calls.filter((c) => c.cmd === "open_pr_worktree")).toHaveLength(1);
    expect(status.text()).toContain("isn't bound to a local clone");
  });
});
