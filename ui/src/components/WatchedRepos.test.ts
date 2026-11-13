import { afterEach, describe, expect, it } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import WatchedRepos from "./WatchedRepos.vue";

type Account = { host: string; login: string };
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
function mockCore(accounts: Account[], options: { failWatch?: Failure; failAccounts?: Failure } = {}) {
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
      default:
        throw new Error(`unexpected command: ${cmd}`);
    }
  });
  return { calls, repos };
}

async function mountRepos() {
  const wrapper = mount(WatchedRepos);
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

  it("shows Bind and Clone, not yet available, for a repo with no local clone", async () => {
    mockCore([{ host: "github.com", login: "octocat" }]);
    const wrapper = await mountRepos();

    await watch(wrapper, "octocat/Hello-World");

    for (const label of ["Bind", "Clone"]) {
      const button = rows(wrapper)[0]
        .findAll("button")
        .find((b) => b.text() === label);
      expect(button?.attributes("disabled"), label).toBeDefined();
    }
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
