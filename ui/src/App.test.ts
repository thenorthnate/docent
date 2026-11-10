import { afterEach, describe, expect, it } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import App from "./App.vue";

afterEach(() => {
  clearMocks();
});

describe("App", () => {
  it("shows the app version reported by the Rust core", async () => {
    const calls: string[] = [];
    mockIPC((cmd) => {
      calls.push(cmd);
      if (cmd === "app_version") {
        return "1.2.3";
      }
      throw new Error(`unexpected command: ${cmd}`);
    });

    const wrapper = mount(App);
    await flushPromises();

    expect(calls).toEqual(["app_version"]);
    expect(wrapper.get("h1").text()).toBe("Docent");
    expect(wrapper.text()).toContain("v1.2.3");
  });
});
