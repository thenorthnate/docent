import { afterEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import SettingsPage from "./SettingsPage.vue";
import { createThemeSetting, themeSettingKey } from "@/theme";

/** Mounts the page with a theme setting that follows a light OS. */
function mountPage() {
  const theme = createThemeSetting({
    root: document.createElement("html"),
    query: { matches: false, addEventListener: () => {}, removeEventListener: () => {} },
    storage: { getItem: () => null, setItem: () => {} },
  });
  return mount(SettingsPage, { global: { provide: { [themeSettingKey]: theme } } });
}

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

    const wrapper = mountPage();
    await flushPromises();

    expect(calls).toContain("app_version");
    expect(wrapper.text()).toContain("Docent v1.2.3");
  });

  it("still renders when the version can't be read", async () => {
    vi.spyOn(console, "error").mockImplementation(() => {});
    mockIPC(() => {
      throw new Error("IPC unavailable");
    });

    const wrapper = mountPage();
    await flushPromises();

    expect(wrapper.get("h1").text()).toBe("Settings");
    expect(wrapper.findAll("h2").map((heading) => heading.text())).toEqual(["Appearance", "Accounts"]);
    expect(wrapper.text()).toContain("version unknown");
  });
});
