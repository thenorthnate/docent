import { afterEach, describe, expect, it } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import AccountsSection from "./AccountsSection.vue";

type Account = { host: string; login: string };

afterEach(() => {
  clearMocks();
});

/** A fake Rust core: `connect_host` answers from `logins`, or fails as given. */
function mockCore(logins: Record<string, string>, failure?: { kind: string; message: string }) {
  const connected: Account[] = [];
  const calls: Array<{ cmd: string; args: unknown }> = [];
  mockIPC((cmd, args) => {
    calls.push({ cmd, args });
    if (cmd === "list_accounts") {
      return [...connected];
    }
    if (cmd === "connect_host") {
      const { host } = args as { host: string };
      if (failure) {
        throw failure;
      }
      const account = { host, login: logins[host] };
      connected.push(account);
      return account;
    }
    throw new Error(`unexpected command: ${cmd}`);
  });
  return { calls, connected };
}

async function mountSection() {
  const wrapper = mount(AccountsSection);
  await flushPromises();
  return wrapper;
}

function rows(wrapper: Awaited<ReturnType<typeof mountSection>>) {
  return wrapper.findAll("[data-testid=account]").map((row) => row.text());
}

async function connect(wrapper: Awaited<ReturnType<typeof mountSection>>, host: string) {
  await wrapper.get("input[name=host]").setValue(host);
  await wrapper.get("form").trigger("submit");
  await flushPromises();
}

describe("AccountsSection", () => {
  it("lists the hosts already connected", async () => {
    const core = mockCore({});
    core.connected.push({ host: "github.com", login: "octocat" });

    const wrapper = await mountSection();

    expect(wrapper.get("h2").text()).toBe("Accounts");
    expect(rows(wrapper)).toEqual([expect.stringContaining("github.com")]);
    expect(rows(wrapper)[0]).toContain("octocat");
  });

  it("offers github.com and shows the host and login once connected", async () => {
    const core = mockCore({ "github.com": "octocat" });
    const wrapper = await mountSection();
    expect((wrapper.get("input[name=host]").element as HTMLInputElement).value).toBe("github.com");

    await wrapper.get("form").trigger("submit");
    await flushPromises();

    expect(core.calls).toContainEqual({ cmd: "connect_host", args: { host: "github.com" } });
    expect(rows(wrapper)).toEqual([expect.stringMatching(/github\.com.*octocat/)]);
  });

  it("keeps two hosts connected together", async () => {
    mockCore({ "github.com": "octocat", "ghe.corp.test": "mona" });
    const wrapper = await mountSection();

    await connect(wrapper, "github.com");
    await connect(wrapper, "ghe.corp.test");

    expect(rows(wrapper)).toHaveLength(2);
    expect(rows(wrapper)[1]).toMatch(/ghe\.corp\.test.*mona/);
  });

  it("shows the core's actionable message when connecting fails", async () => {
    const message =
      "`gh` is not logged in to ghe.corp.test. Run `gh auth login --hostname ghe.corp.test` in a terminal, then try again.";
    mockCore({}, { kind: "not_logged_in", message });
    const wrapper = await mountSection();

    await connect(wrapper, "ghe.corp.test");

    expect(wrapper.get("[role=alert]").text()).toBe(message);
    expect(rows(wrapper)).toEqual([]);
  });
});
