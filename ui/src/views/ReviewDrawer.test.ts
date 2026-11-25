import { afterEach, describe, expect, it } from "vitest";
import { enableAutoUnmount, flushPromises, mount } from "@vue/test-utils";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { createPinia } from "pinia";
import { createMemoryHistory } from "vue-router";
import App from "@/App.vue";
import { createAppRouter } from "@/router";
import type { ChangedFile } from "@/lib/review";
import { useReviewStore } from "@/stores/review";

const BASE = "b".repeat(40);
const HEAD = "a".repeat(40);
const PR_PATH = "/review/github.com/octocat/Hello-World/1347";
const OTHER_PATH = "/review/github.com/octocat/Hello-World/7";

const [B1, B2, B3, B4, B5] = ["1", "2", "3", "4", "5"].map((c) => c.repeat(40));

const FILES: ChangedFile[] = [
  { path: "gone.txt", old_path: null, status: "deleted", lines: { added: 0, deleted: 3 }, old_blob: B1, new_blob: null },
  { path: "logo.png", old_path: null, status: "added", lines: null, old_blob: null, new_blob: B2 },
  { path: "src/main.ts", old_path: null, status: "modified", lines: { added: 12, deleted: 4 }, old_blob: B3, new_blob: B4 },
  { path: "src/new.ts", old_path: "lib/old.ts", status: "renamed", lines: { added: 1, deleted: 1 }, old_blob: B5, new_blob: B5 },
];

type Args = Record<string, unknown>;
type Answer = (args: Args) => unknown;

/**
 * The app at `path`, over a fake core. `changed_files` runs `files`; each
 * other command may be overridden in `others`.
 */
async function mountApp(path: string, files: Answer = () => FILES, others: Record<string, Answer> = {}) {
  const asked: Args[] = [];
  const answers: Record<string, Answer> = {
    list_watched_repos: () => [],
    open_pr_worktree: () => ({ status: "ready", path: "/w", head_sha: HEAD }),
    pull_overview: () => ({
      title: "Fix",
      author: "mona",
      body: null,
      base_sha: BASE,
      ci: "passing",
      checks: [],
      timeline: [],
      timeline_truncated: false,
    }),
    changed_files: (args) => {
      asked.push(args.range as Args);
      return files(args.range as Args);
    },
    ...others,
  };
  mockIPC((cmd, args) => {
    if (!(cmd in answers)) throw new Error(`unexpected command: ${cmd}`);
    return answers[cmd](args as Args);
  });
  const pinia = createPinia();
  const router = createAppRouter(createMemoryHistory());
  await router.push(path);
  await router.isReady();
  const wrapper = mount(App, { global: { plugins: [pinia, router] }, attachTo: document.body });
  await flushPromises();
  return { drawer: () => wrapper.get("[data-testid=drawer]"), router, asked, store: useReviewStore(pinia) };
}

enableAutoUnmount(afterEach);
afterEach(() => clearMocks());

describe("ReviewDrawer", () => {
  it("asks for the files between the forge's base and the worktree's head", async () => {
    const { asked } = await mountApp(PR_PATH);

    expect(asked).toEqual([
      { host: "github.com", owner: "octocat", repo: "Hello-World", number: 1347, base_sha: BASE, head_sha: HEAD },
    ]);
  });

  it("lists each file in its folder with its status and line counts", async () => {
    const { drawer } = await mountApp(PR_PATH);

    const text = (part: string) =>
      drawer()
        .findAll(`[data-testid=file-${part}]`)
        .map((el) => el.text());
    expect(drawer().get("h3").text()).toBe("Files changed 4");
    expect(text("folder")).toEqual(["src"]);
    expect(text("name")).toEqual(["main.ts", "new.ts", "gone.txt", "logo.png"]);
    expect(text("status")).toEqual(["M", "R", "D", "A"]);
    expect(text("lines")).toEqual(["+12 −4", "+1 −1", "+0 −3", "binary"]);
  });

  it("shows where a renamed file came from", async () => {
    const { drawer } = await mountApp(PR_PATH);

    const renamed = drawer().findAll("button").find((b) => b.text().includes("new.ts"))!;
    expect(renamed.text()).toContain("from lib/old.ts");
    expect(renamed.attributes("title")).toBe("Renamed: lib/old.ts → src/new.ts");
  });

  it("makes the selected file the current one", async () => {
    const { drawer, store } = await mountApp(PR_PATH);
    expect(store.currentFile).toBeNull();

    await drawer().findAll("button").find((b) => b.text().includes("main.ts"))!.trigger("click");

    expect(store.currentFile).toEqual(FILES[2]);
    const current = drawer().findAll("[aria-current=true]");
    expect(current.map((b) => b.get("[data-testid=file-name]").text())).toEqual(["main.ts"]);
  });

  it("waits for the worktree before listing files", async () => {
    const { drawer, asked } = await mountApp(PR_PATH, undefined, { open_pr_worktree: () => new Promise(() => {}) });

    expect(asked).toEqual([]);
    expect(drawer().text()).toContain("Files show once the worktree is ready.");
  });

  it("shows the core's error and retries on request", async () => {
    let failing = true;
    const { drawer } = await mountApp(PR_PATH, () => {
      if (failing) throw { kind: "git_failed", message: "git failed: no merge base" };
      return FILES;
    });
    expect(drawer().get("[role=alert]").text()).toBe("git failed: no merge base");

    failing = false;
    await drawer().get("button").trigger("click");
    await flushPromises();

    expect(drawer().findAll("[data-testid=file-name]")).toHaveLength(4);
  });

  it("says when the PR changes no files", async () => {
    const { drawer } = await mountApp(PR_PATH, () => []);

    expect(drawer().text()).toContain("No files changed.");
  });

  it("drops the files and selection of a PR that was left, even if they answer late", async () => {
    let answerFirst: (files: ChangedFile[]) => void = () => {};
    const { drawer, router, store } = await mountApp(PR_PATH, (range) =>
      range.number === 1347 ? new Promise((resolve) => (answerFirst = resolve)) : [FILES[0]],
    );

    await router.push(OTHER_PATH);
    await flushPromises();
    store.select("gone.txt");
    answerFirst(FILES);
    await flushPromises();

    expect(drawer().findAll("[data-testid=file-name]").map((el) => el.text())).toEqual(["gone.txt"]);

    await router.push(PR_PATH);
    await flushPromises();
    expect(store.currentFile).toBeNull();
    expect(drawer().findAll("[data-testid=file-name]")).toHaveLength(0);
  });

  it("lists nothing when no PR is open", async () => {
    const { drawer, router, asked } = await mountApp(PR_PATH);

    await router.push("/review");
    await flushPromises();

    expect(asked).toHaveLength(1);
    expect(drawer().find("h3").exists()).toBe(false);
  });
});
