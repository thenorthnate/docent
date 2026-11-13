import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { createMemoryHistory } from "vue-router";
import { createPinia } from "pinia";
import App from "./App.vue";
import { createAppRouter } from "./router";

beforeEach(() => {
  mockIPC((cmd) => {
    if (cmd === "app_version") return "1.2.3";
    if (cmd === "list_accounts") return [];
    if (cmd === "list_watched_repos") return [];
    return undefined;
  });
});

afterEach(() => {
  clearMocks();
});

async function mountApp(path = "/") {
  const router = createAppRouter(createMemoryHistory());
  await router.push(path);
  await router.isReady();
  const wrapper = mount(App, {
    global: { plugins: [router, createPinia()] },
  });
  await flushPromises();
  return { wrapper, router };
}

function railButton(wrapper: Awaited<ReturnType<typeof mountApp>>["wrapper"], name: string) {
  return wrapper.get(`nav[aria-label="Activity bar"] button[aria-label="${name}"]`);
}

describe("App shell", () => {
  it("shows Inbox, Review and Settings on the rail and opens on the Inbox", async () => {
    const { wrapper, router } = await mountApp("/");

    for (const name of ["Inbox", "Review", "Settings"]) {
      expect(() => railButton(wrapper, name)).not.toThrow();
    }
    expect(router.currentRoute.value.path).toBe("/inbox");
    expect(railButton(wrapper, "Inbox").attributes("aria-current")).toBe("page");
    expect(wrapper.get('[data-testid="drawer"]').text()).toContain("Inbox");
    expect(wrapper.get("main h1").text()).toBe("Inbox");
  });

  it.each([
    ["Review", "/review"],
    ["Settings", "/settings"],
    ["Inbox", "/inbox"],
  ])("clicking %s routes to its page and drawer", async (name, path) => {
    const { wrapper, router } = await mountApp(name === "Inbox" ? "/review" : "/inbox");

    await railButton(wrapper, name).trigger("click");
    await flushPromises();

    expect(router.currentRoute.value.path).toBe(path);
    expect(railButton(wrapper, name).attributes("aria-current")).toBe("page");
    expect(wrapper.get('[data-testid="drawer"] h2').text()).toBe(name);
    expect(wrapper.get("main h1").text()).toBe(name);
  });

  it("clicking the active icon closes its drawer and keeps the page", async () => {
    const { wrapper, router } = await mountApp("/review");
    expect(railButton(wrapper, "Review").attributes("aria-expanded")).toBe("true");

    await railButton(wrapper, "Review").trigger("click");
    await flushPromises();

    expect(wrapper.find('[data-testid="drawer"]').exists()).toBe(false);
    expect(railButton(wrapper, "Review").attributes("aria-expanded")).toBe("false");
    expect(router.currentRoute.value.path).toBe("/review");
    expect(wrapper.get("main h1").text()).toBe("Review");
  });

  it("clicking the active icon again reopens its drawer", async () => {
    const { wrapper } = await mountApp("/review");

    await railButton(wrapper, "Review").trigger("click");
    await railButton(wrapper, "Review").trigger("click");
    await flushPromises();

    expect(wrapper.get('[data-testid="drawer"] h2').text()).toBe("Review");
  });

  it("clicking another icon while the drawer is closed opens that area's drawer", async () => {
    const { wrapper, router } = await mountApp("/inbox");

    await railButton(wrapper, "Inbox").trigger("click");
    await flushPromises();
    await railButton(wrapper, "Settings").trigger("click");
    await flushPromises();

    expect(router.currentRoute.value.path).toBe("/settings");
    expect(wrapper.get('[data-testid="drawer"] h2').text()).toBe("Settings");
  });
});
