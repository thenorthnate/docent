import { afterEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import SettingsPage from "./SettingsPage.vue";

afterEach(() => {
  clearMocks();
  vi.restoreAllMocks();
});

describe("SettingsPage", () => {
  it("shows the app version reported by the Rust core", async () => {
    const calls: string[] = [];
    mockIPC((cmd) => {
      calls.push(cmd);
      if (cmd === "app_version") {
        return "1.2.3";
      }
      if (cmd === "list_accounts") {
        return [];
      }
      throw new Error(`unexpected command: ${cmd}`);
    });

    const wrapper = mount(SettingsPage);
    await flushPromises();

    expect(calls).toContain("app_version");
    expect(wrapper.text()).toContain("Docent v1.2.3");
  });

  it("still renders when the version can't be read", async () => {
    vi.spyOn(console, "error").mockImplementation(() => {});
    mockIPC(() => {
      throw new Error("IPC unavailable");
    });

    const wrapper = mount(SettingsPage);
    await flushPromises();

    expect(wrapper.get("h1").text()).toBe("Settings");
    expect(wrapper.get("h2").text()).toBe("Accounts");
    expect(wrapper.text()).toContain("version unknown");
  });
});
