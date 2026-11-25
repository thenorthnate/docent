import { afterEach, describe, expect, it } from "vitest";
import { enableAutoUnmount, flushPromises, mount } from "@vue/test-utils";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { createPinia } from "pinia";
import { createMemoryHistory } from "vue-router";
import App from "@/App.vue";
import { createAppRouter } from "@/router";
import type { ChangedFile, FileDiff } from "@/lib/review";

const BASE = "b".repeat(40);
const HEAD = "a".repeat(40);
const PR_PATH = "/review/github.com/octocat/Hello-World/1347";
const RANGE = { host: "github.com", owner: "octocat", repo: "Hello-World", number: 1347, base_sha: BASE, head_sha: HEAD };

const file = (path: string, old_blob: string | null, new_blob: string | null): ChangedFile => ({
  path,
  old_path: null,
  status: "modified",
  lines: { added: 1, deleted: 1 },
  old_blob,
  new_blob,
});

// Two names that aren't UTF-8 and look the same once their bad bytes are replaced.
const FILES = [file("bad\u{fffd}.txt", "1".repeat(40), "2".repeat(40)), file("bad\u{fffd}.txt", null, "3".repeat(40))];

const TEXT: FileDiff = {
  kind: "text",
  old: "keep\nold line\nend\n",
  new: "keep\nnew line\nend\n",
  hunks: [{ old_start: 2, old_lines: 1, new_start: 2, new_lines: 1 }],
};

type Args = Record<string, unknown>;

/** The app at the PR, over a fake core whose `file_diff` runs `diff`. */
async function mountApp(diff: (args: Args) => unknown) {
  const asked: Args[] = [];
  mockIPC((cmd, args) => {
    switch (cmd) {
      case "list_watched_repos":
        return [];
      case "open_pr_worktree":
        return { status: "ready", path: "/w", head_sha: HEAD };
      case "pull_overview":
        return { title: "Fix", author: "mona", body: null, base_sha: BASE, ci: "passing", checks: [], timeline: [], timeline_truncated: false };
      case "changed_files":
        return FILES;
      case "file_diff":
        asked.push(args as Args);
        return diff(args as Args);
    }
    throw new Error(`unexpected command: ${cmd}`);
  });
  const router = createAppRouter(createMemoryHistory());
  await router.push(PR_PATH);
  await router.isReady();
  const wrapper = mount(App, { global: { plugins: [createPinia(), router] }, attachTo: document.body });
  await flushPromises();
  const select = async (index: number) => {
    await wrapper.get("[data-testid=drawer]").findAll("button[title]")[index].trigger("click");
    await flushPromises();
  };
  return { wrapper, asked, select, viewer: () => wrapper.get("[data-testid=diff-viewer]") };
}

enableAutoUnmount(afterEach);
afterEach(() => clearMocks());

describe("DiffViewer", () => {
  it("shows the selected file's diff from git's hunks, read-only", async () => {
    const { asked, select, viewer } = await mountApp(() => TEXT);

    await select(0);

    expect(asked).toEqual([{ range: RANGE, blobs: { old_blob: "1".repeat(40), new_blob: "2".repeat(40) } }]);
    expect(viewer().get(".cm-content").text()).toContain("new line");
    expect(viewer().get(".cm-deletedChunk").text()).toBe("old line");
    expect(viewer().get(".cm-content").attributes("contenteditable")).toBe("false");
  });

  it("shows each look-alike file's own diff", async () => {
    const { asked, select, viewer } = await mountApp((args) => {
      const blobs = args.blobs as Args;
      return { ...TEXT, new: `keep\n${String(blobs.new_blob).slice(0, 1)} line\nend\n` };
    });

    await select(1);

    expect(asked.map((args) => args.blobs)).toEqual([{ old_blob: null, new_blob: "3".repeat(40) }]);
    expect(viewer().get(".cm-content").text()).toContain("3 line");
  });

  it("shows a placeholder for binary and very large files", async () => {
    const answers: FileDiff[] = [{ kind: "binary" }, { kind: "too_large", bytes: 6_500_000 }];
    const { select, viewer } = await mountApp(() => answers.shift());

    await select(0);
    expect(viewer().text()).toBe("Binary file not shown.");
    await select(1);
    expect(viewer().text()).toBe("Too large to show (6.2 MB).");
    expect(viewer().find(".cm-editor").exists()).toBe(false);
  });

  it("shows the core's error and retries on request", async () => {
    let failing = true;
    const { select, viewer } = await mountApp(() => {
      if (failing) throw { kind: "worktree_missing", message: "The PR's worktree is gone." };
      return TEXT;
    });
    await select(0);
    expect(viewer().get("[role=alert]").text()).toBe("The PR's worktree is gone.");

    failing = false;
    await viewer().get("button").trigger("click");
    await flushPromises();

    expect(viewer().get(".cm-content").text()).toContain("new line");
  });

  it("drops a late answer for a file no longer selected", async () => {
    let answerFirst: (diff: FileDiff) => void = () => {};
    const { select, viewer } = await mountApp((args) =>
      (args.blobs as Args).old_blob ? new Promise((resolve) => (answerFirst = resolve)) : { kind: "text", old: "second\n", new: "second\n", hunks: [] },
    );

    await select(0);
    await select(1);
    answerFirst(TEXT);
    await flushPromises();

    expect(viewer().get(".cm-content").text()).toBe("second");
  });
});
