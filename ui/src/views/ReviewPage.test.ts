import { afterEach, describe, expect, it } from "vitest";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { flushPromises, mount } from "@vue/test-utils";
import { createMemoryHistory } from "vue-router";
import ReviewPage from "./ReviewPage.vue";
import { createAppRouter } from "@/router";

async function mountPage(path: string) {
  const router = createAppRouter(createMemoryHistory());
  await router.push(path);
  await router.isReady();
  const wrapper = mount(ReviewPage, { global: { plugins: [router] } });
  return { wrapper, router };
}

describe("ReviewPage", () => {
  it("shows the opened PR's breadcrumb", async () => {
    const { wrapper } = await mountPage("/review/github.com/octocat/Hello-World/1347");

    const crumbs = wrapper.get("nav[aria-label=Breadcrumb]");
    expect(crumbs.findAll("li").map((c) => c.text())).toEqual(["Inbox", "octocat/Hello-World", "#1347"]);
    expect(crumbs.get("[aria-current=page]").text()).toBe("#1347");
  });

  it("leads back to the inbox from the breadcrumb", async () => {
    const { wrapper, router } = await mountPage("/review/github.com/octocat/Hello-World/1347");

    await wrapper.get("nav[aria-label=Breadcrumb] a").trigger("click");
    await flushPromises();

    expect(router.currentRoute.value.path).toBe("/inbox");
  });

  it("asks for a PR when none is open", async () => {
    const { wrapper } = await mountPage("/review");

    expect(wrapper.find("nav[aria-label=Breadcrumb]").exists()).toBe(false);
    expect(wrapper.text()).toContain("Open a pull request from the Inbox");
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

  /** A fake core that answers `open_pr_worktree` with each of `answers` in turn. */
  function mockCore(answers: Array<object | Failure | "never">, bind: object | null = null) {
    const calls: Array<{ cmd: string; args: unknown }> = [];
    mockIPC((cmd, args) => {
      calls.push({ cmd, args });
      if (cmd === "open_pr_worktree") {
        const answer = answers.shift();
        if (answer === "never") return new Promise(() => {});
        if (answer && "kind" in answer) throw answer;
        return answer;
      }
      if (cmd === "bind_repo") return bind;
      throw new Error(`unexpected ${cmd}`);
    });
    return calls;
  }

  afterEach(() => {
    clearMocks();
  });

  async function open() {
    const { wrapper } = await mountPage("/review/github.com/octocat/Hello-World/1347");
    await flushPromises();
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
