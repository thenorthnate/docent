import { afterEach, describe, expect, it } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import type { Channel } from "@tauri-apps/api/core";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { createPinia } from "pinia";
import type { CloneUpdate } from "@/stores/clones";
import WatchedRepos from "./WatchedRepos.vue";

type Account = { host: string; login: string; error?: Failure | null };
type Repo = {
  id: number;
  host: string;
  owner: string;
  name: string;
  default_branch: string;
  local_path: string | null;
};
type Failure = { kind: string; message: string };

afterEach(() => {
  clearMocks();
});

/** A fake Rust core holding `accounts` and a list of watched repos. */
type CoreOptions = {
  failWatch?: Failure;
  failAccounts?: Failure;
  /** What Bind's folder picker yields: a clone path, a cancel, or a failure. */
  bind?: { path: string } | "cancel" | Failure;
  /** Answers Clone. */
  clone?: (args: unknown) => unknown;
};

/** A clone that reports and finishes when told to. */
function deferredClone() {
  let channel: Channel<CloneUpdate>;
  let resolve!: (repo: Repo) => void;
  const answer = new Promise<Repo>((r) => (resolve = r));
  return {
    run: (args: unknown) => {
      channel = (args as { onProgress: Channel<CloneUpdate> }).onProgress;
      return answer;
    },
    report: (update: CloneUpdate) => channel.onmessage(update),
    resolve: (repo: Repo) => resolve(repo),
  };
}

function mockCore(accounts: Account[], options: CoreOptions = {}) {
  const repos: Repo[] = [];
  const calls: Array<{ cmd: string; args: unknown }> = [];
  let nextId = 1;
  mockIPC((cmd, args) => {
    calls.push({ cmd, args });
    switch (cmd) {
      case "list_accounts":
        if (options.failAccounts) throw options.failAccounts;
        return [...accounts];
      case "list_watched_repos":
        return [...repos];
      case "watch_repo": {
        if (options.failWatch) throw options.failWatch;
        const { host, repo } = args as { host: string; repo: string };
        const [owner, name] = repo.split("/");
        const watched = { id: nextId++, host, owner, name, default_branch: "main", local_path: null };
        repos.push(watched);
        return watched;
      }
      case "unwatch_repo": {
        const { id } = args as { id: number };
        repos.splice(
          repos.findIndex((r) => r.id === id),
          1,
        );
        return null;
      }
      case "bind_repo": {
        const { id } = args as { id: number };
        const bind = options.bind ?? "cancel";
        if (bind === "cancel") return null;
        if ("kind" in bind) throw bind;
        const repo = repos.find((r) => r.id === id)!;
        repo.local_path = bind.path;
        return { ...repo };
      }
      case "clone_repo":
        return options.clone?.(args) ?? null;
      case "list_section":
        return { pulls: [], failures: [] };
      default:
        throw new Error(`unexpected command: ${cmd}`);
    }
  });
  return { calls, repos };
}

async function mountRepos(pinia = createPinia()) {
  const wrapper = mount(WatchedRepos, { global: { plugins: [pinia] } });
  await flushPromises();
  return wrapper;
}

type Wrapper = Awaited<ReturnType<typeof mountRepos>>;

function rows(wrapper: Wrapper) {
  return wrapper.findAll("[data-testid=watched-repo]");
}

async function watch(wrapper: Wrapper, repo: string) {
  await wrapper.get("input[name=repo]").setValue(repo);
  await wrapper.get("form").trigger("submit");
  await flushPromises();
}

describe("WatchedRepos", () => {
  it("asks for a connected host before a repo can be watched", async () => {
    mockCore([]);

    const wrapper = await mountRepos();

    expect(wrapper.text()).toContain("Connect a host in Settings");
    expect(wrapper.find("form").exists()).toBe(false);
  });

  it("only offers hosts that are connected, not ones that failed to reconnect", async () => {
    const core = mockCore([
      { host: "ghe.corp.test", login: "mona", error: { kind: "not_logged_in", message: "not logged in" } },
      { host: "github.com", login: "octocat", error: null },
    ]);
    const wrapper = await mountRepos();
    expect(wrapper.find("select").exists()).toBe(false);

    await watch(wrapper, "octocat/Hello-World");

    expect(core.calls).toContainEqual({
      cmd: "watch_repo",
      args: { host: "github.com", repo: "octocat/Hello-World" },
    });
  });

  it("asks to reconnect when no host is connected", async () => {
    mockCore([{ host: "github.com", login: "octocat", error: { kind: "not_logged_in", message: "x" } }]);

    const wrapper = await mountRepos();

    expect(wrapper.text()).toContain("Connect a host in Settings");
    expect(wrapper.find("form").exists()).toBe(false);
  });

  it("lists the repos already watched", async () => {
    const core = mockCore([{ host: "github.com", login: "octocat" }]);
    core.repos.push({
      id: 7,
      host: "github.com",
      owner: "octocat",
      name: "Hello-World",
      default_branch: "main",
      local_path: null,
    });

    const wrapper = await mountRepos();

    expect(rows(wrapper).map((r) => r.text())).toEqual([expect.stringContaining("octocat/Hello-World")]);
  });

  it("watches a repo on the only connected host and lists it", async () => {
    const core = mockCore([{ host: "github.com", login: "octocat" }]);
    const wrapper = await mountRepos();
    expect(wrapper.find("select").exists()).toBe(false);

    await watch(wrapper, "octocat/Hello-World");

    expect(core.calls).toContainEqual({
      cmd: "watch_repo",
      args: { host: "github.com", repo: "octocat/Hello-World" },
    });
    expect(rows(wrapper)).toHaveLength(1);
    expect((wrapper.get("input[name=repo]").element as HTMLInputElement).value).toBe("");
  });

  it("lets the user pick the host when more than one is connected", async () => {
    const core = mockCore([
      { host: "ghe.corp.test", login: "mona" },
      { host: "github.com", login: "octocat" },
    ]);
    const wrapper = await mountRepos();

    await wrapper.get("select[name=host]").setValue("github.com");
    await watch(wrapper, "octocat/Hello-World");

    expect(core.calls).toContainEqual({
      cmd: "watch_repo",
      args: { host: "github.com", repo: "octocat/Hello-World" },
    });
    expect(rows(wrapper)).toHaveLength(1);
  });

  it.each([
    ["a duplicate", { kind: "already_watched", message: "github.com:octocat/Hello-World is already watched." }],
    [
      "a missing repo",
      { kind: "not_found", message: "github.com has no repo octocat/nope, or your account can't see it." },
    ],
  ])("shows the core's message when watching %s fails", async (_case, failure) => {
    mockCore([{ host: "github.com", login: "octocat" }], { failWatch: failure });
    const wrapper = await mountRepos();

    await watch(wrapper, "octocat/Hello-World");

    expect(wrapper.get("[role=alert]").text()).toBe(failure.message);
    expect(rows(wrapper)).toHaveLength(0);
  });

  function button(wrapper: Wrapper, label: string) {
    return rows(wrapper)[0]
      .findAll("button")
      .find((b) => b.text() === label);
  }

  it("offers Bind and Clone for a repo with no local clone", async () => {
    mockCore([{ host: "github.com", login: "octocat" }]);
    const wrapper = await mountRepos();

    await watch(wrapper, "octocat/Hello-World");

    expect(button(wrapper, "Bind")?.attributes("disabled")).toBeUndefined();
    expect(button(wrapper, "Bind")?.attributes("title")).toContain("already have");
    expect(button(wrapper, "Clone")?.attributes("disabled")).toBeUndefined();
    expect(button(wrapper, "Clone")?.attributes("title")).toContain("existing git credentials");
  });

  it("shows a clone's progress, then the bound path", async () => {
    const clone = deferredClone();
    const core = mockCore([{ host: "github.com", login: "octocat" }], { clone: clone.run });
    const wrapper = await mountRepos();
    await watch(wrapper, "octocat/Hello-World");

    await button(wrapper, "Clone")!.trigger("click");
    expect(wrapper.get("[role=status]").text()).toBe("Choosing a folder…");
    clone.report({ phase: "Starting", percent: null });
    await flushPromises();
    expect(wrapper.get("[role=status]").text()).toBe("Starting…");
    clone.report({ phase: "Receiving objects", percent: 45 });
    await flushPromises();
    expect(wrapper.get("[role=status]").text()).toBe("Receiving objects 45%");
    expect(button(wrapper, "Cloning…")?.attributes("disabled")).toBeDefined();
    expect(button(wrapper, "Bind")?.attributes("disabled")).toBeDefined();

    core.repos[0].local_path = "/Users/me/code/Hello-World";
    clone.resolve({ ...core.repos[0] });
    await flushPromises();

    expect(rows(wrapper)[0].text()).toContain("/Users/me/code/Hello-World");
    expect(wrapper.find("[role=status]").exists()).toBe(false);
  });

  it("still shows a running clone after leaving the page and coming back", async () => {
    const clone = deferredClone();
    const core = mockCore([{ host: "github.com", login: "octocat" }], { clone: clone.run });
    const pinia = createPinia();
    const first = await mountRepos(pinia);
    await watch(first, "octocat/Hello-World");
    await button(first, "Clone")!.trigger("click");
    first.unmount();

    const wrapper = await mountRepos(pinia);
    clone.report({ phase: "Receiving objects", percent: 80 });
    await flushPromises();

    expect(wrapper.get("[role=status]").text()).toBe("Receiving objects 80%");
    expect(button(wrapper, "Cloning…")?.attributes("disabled")).toBeDefined();

    core.repos[0].local_path = "/Users/me/code/Hello-World";
    clone.resolve({ ...core.repos[0] });
    await flushPromises();
    expect(rows(wrapper)[0].text()).toContain("/Users/me/code/Hello-World");
  });

  it("shows why a clone failed, and can try again", async () => {
    const answers: Array<() => unknown> = [
      () => {
        throw { kind: "folder_exists", message: "/code/Hello-World already exists." };
      },
      () => null,
    ];
    mockCore([{ host: "github.com", login: "octocat" }], { clone: () => answers.shift()!() });
    const wrapper = await mountRepos();
    await watch(wrapper, "octocat/Hello-World");

    await button(wrapper, "Clone")!.trigger("click");
    await flushPromises();
    expect(wrapper.get("[role=alert]").text()).toBe("/code/Hello-World already exists.");
    expect(button(wrapper, "Bind")?.attributes("disabled")).toBeUndefined();

    // The second time, the picker is cancelled.
    await button(wrapper, "Clone")!.trigger("click");
    await flushPromises();
    expect(wrapper.find("[role=alert]").exists()).toBe(false);
    expect(button(wrapper, "Clone")?.attributes("disabled")).toBeUndefined();
  });

  it("binds a repo to the clone picked and shows its path instead of Bind and Clone", async () => {
    const core = mockCore([{ host: "github.com", login: "octocat" }], {
      bind: { path: "/Users/me/code/hello" },
    });
    const wrapper = await mountRepos();
    await watch(wrapper, "octocat/Hello-World");

    await button(wrapper, "Bind")!.trigger("click");
    await flushPromises();

    expect(core.calls).toContainEqual({ cmd: "bind_repo", args: { id: 1 } });
    expect(rows(wrapper)[0].text()).toContain("/Users/me/code/hello");
    expect(button(wrapper, "Bind")).toBeUndefined();
    expect(button(wrapper, "Clone")).toBeUndefined();
  });

  it("leaves the repo unbound, with no error, when the picker is cancelled", async () => {
    mockCore([{ host: "github.com", login: "octocat" }], { bind: "cancel" });
    const wrapper = await mountRepos();
    await watch(wrapper, "octocat/Hello-World");

    await button(wrapper, "Bind")!.trigger("click");
    await flushPromises();

    expect(button(wrapper, "Bind")).toBeDefined();
    expect(wrapper.find("[role=alert]").exists()).toBe(false);
  });

  it("shows why a folder was rejected and keeps Bind", async () => {
    const failure = {
      kind: "no_matching_remote",
      message: "None of this clone's remotes point at github.com:octocat/Hello-World.",
    };
    mockCore([{ host: "github.com", login: "octocat" }], { bind: failure });
    const wrapper = await mountRepos();
    await watch(wrapper, "octocat/Hello-World");

    await button(wrapper, "Bind")!.trigger("click");
    await flushPromises();

    expect(wrapper.get("[role=alert]").text()).toBe(failure.message);
    expect(button(wrapper, "Bind")).toBeDefined();
  });

  it("shows the clone path of a repo bound earlier", async () => {
    const core = mockCore([{ host: "github.com", login: "octocat" }]);
    core.repos.push({
      id: 3,
      host: "github.com",
      owner: "octocat",
      name: "Hello-World",
      default_branch: "main",
      local_path: "/Users/me/code/hello",
    });

    const wrapper = await mountRepos();

    expect(rows(wrapper)[0].text()).toContain("/Users/me/code/hello");
    expect(button(wrapper, "Bind")).toBeUndefined();
  });

  it("names each repo's host when the watched repos span hosts, even with none connected", async () => {
    const core = mockCore([]);
    for (const [id, host] of [
      [1, "ghe.corp.test"],
      [2, "github.com"],
    ] as const) {
      core.repos.push({ id, host, owner: "octocat", name: "tool", default_branch: "main", local_path: null });
    }

    const wrapper = await mountRepos();

    expect(rows(wrapper).map((r) => r.text())).toEqual([
      expect.stringContaining("ghe.corp.test"),
      expect.stringContaining("github.com"),
    ]);
  });

  it("still lists watched repos when the accounts can't be read", async () => {
    const core = mockCore([], { failAccounts: { kind: "database", message: "boom" } });
    core.repos.push({ id: 1, host: "github.com", owner: "o", name: "r", default_branch: "main", local_path: null });

    const wrapper = await mountRepos();

    expect(rows(wrapper)).toHaveLength(1);
    expect(wrapper.get("[role=alert]").text()).toBe("boom");
  });

  it("unwatches a repo", async () => {
    const core = mockCore([{ host: "github.com", login: "octocat" }]);
    const wrapper = await mountRepos();
    await watch(wrapper, "octocat/Hello-World");

    await rows(wrapper)[0].get("button[aria-label='Unwatch octocat/Hello-World']").trigger("click");
    await flushPromises();

    expect(core.calls).toContainEqual({ cmd: "unwatch_repo", args: { id: 1 } });
    expect(rows(wrapper)).toHaveLength(0);
  });
});
