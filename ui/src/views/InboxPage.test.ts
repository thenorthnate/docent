import { afterEach, describe, expect, it } from "vitest";
import { enableAutoUnmount, flushPromises, mount } from "@vue/test-utils";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { createMemoryHistory } from "vue-router";
import { createPinia } from "pinia";
import InboxPage from "./InboxPage.vue";
import { createAppRouter } from "@/router";
import type { PullSummary } from "@/stores/inbox";

const watchedRepo = {
  id: 1,
  host: "github.com",
  owner: "octocat",
  name: "Hello-World",
  default_branch: "main",
  local_path: null,
};

function pull(number: number, title: string, overrides: Partial<PullSummary> = {}): PullSummary {
  return {
    host: "github.com",
    owner: "octocat",
    repo: "Hello-World",
    number,
    title,
    author: "mona",
    created_at: "2026-11-10T12:00:00Z",
    body: null,
    ...overrides,
  };
}

const pulls = [
  pull(1347, "Fix the greeting", { body: "Say hello properly.\n\nFixes #12." }),
  pull(7, "Sharpen the knife", { repo: "Spoon-Knife", author: "hubot" }),
  pull(3, "Polish the spoon"),
];

async function mountPage() {
  mockIPC((cmd) => {
    switch (cmd) {
      case "list_watched_repos":
        return [watchedRepo];
      case "list_section":
        return { pulls, failures: [] };
      default:
        throw new Error(`unexpected command: ${cmd}`);
    }
  });
  const router = createAppRouter(createMemoryHistory());
  await router.push("/inbox");
  await router.isReady();
  const wrapper = mount(InboxPage, { global: { plugins: [router, createPinia()] }, attachTo: document.body });
  await flushPromises();
  return { wrapper, router };
}

type Page = Awaited<ReturnType<typeof mountPage>>["wrapper"];

function row(wrapper: Page, title: string) {
  const found = wrapper.findAll("[role=option]").find((r) => r.text().includes(title));
  if (!found) throw new Error(`no row titled ${title}`);
  return found;
}

function selectedTitle(wrapper: Page) {
  return wrapper.find("[role=option][aria-selected=true]").exists()
    ? wrapper.get("[role=option][aria-selected=true]").text()
    : null;
}

function preview(wrapper: Page) {
  return wrapper.get("[data-testid=inbox-preview]");
}

/** A key pressed with focus on `target` (the page body unless given), as the document sees it. */
async function press(key: string, target: Element = document.body, init: KeyboardEventInit = {}) {
  target.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true, ...init }));
  await flushPromises();
}

enableAutoUnmount(afterEach);

afterEach(() => {
  clearMocks();
  document.body.innerHTML = "";
});

describe("InboxPage", () => {
  it("shows a clicked row's title, author, repo and description in the preview", async () => {
    const { wrapper } = await mountPage();

    await row(wrapper, "Fix the greeting").trigger("click");

    expect(selectedTitle(wrapper)).toContain("Fix the greeting");
    const shown = preview(wrapper);
    expect(shown.get("h2").text()).toBe("Fix the greeting");
    expect(shown.text()).toContain("#1347");
    expect(shown.text()).toContain("mona");
    expect(shown.text()).toContain("octocat/Hello-World");
    expect(shown.get("[data-testid=pr-description]").text()).toBe("Say hello properly.\n\nFixes #12.");
  });

  it("says so when the PR has no description, and when nothing is selected", async () => {
    const { wrapper } = await mountPage();
    expect(preview(wrapper).text()).toBe("Select a pull request to preview it.");

    await row(wrapper, "Sharpen the knife").trigger("click");

    expect(preview(wrapper).find("[data-testid=pr-description]").exists()).toBe(false);
    expect(preview(wrapper).text()).toContain("No description provided.");
  });

  it("moves the selection with j and k, starting at the top and stopping at the ends", async () => {
    const { wrapper } = await mountPage();
    expect(selectedTitle(wrapper)).toBeNull();

    await press("j");
    expect(selectedTitle(wrapper)).toContain("Fix the greeting");
    await press("j");
    await press("j");
    await press("j");
    expect(selectedTitle(wrapper)).toContain("Polish the spoon");
    expect(preview(wrapper).get("h2").text()).toBe("Polish the spoon");

    await press("k");
    expect(selectedTitle(wrapper)).toContain("Sharpen the knife");
    expect(preview(wrapper).text()).toContain("hubot");
    await press("k");
    await press("k");
    expect(selectedTitle(wrapper)).toContain("Fix the greeting");
  });

  it("leaves j and k alone while typing or with a modifier held", async () => {
    const { wrapper } = await mountPage();
    const input = document.createElement("input");
    document.body.append(input);

    await press("j", input);
    await press("j", document.body, { metaKey: true });
    await press("j", document.body, { ctrlKey: true });

    expect(selectedTitle(wrapper)).toBeNull();
  });

  const reviewPath = "/review/github.com/octocat/Spoon-Knife/7";

  it("opens the selected PR's review on Enter", async () => {
    const { wrapper, router } = await mountPage();
    await row(wrapper, "Sharpen the knife").trigger("click");

    await press("Enter");

    expect(router.currentRoute.value.path).toBe(reviewPath);
  });

  it("does nothing on Enter when no PR is selected", async () => {
    const { router } = await mountPage();

    await press("Enter");

    expect(router.currentRoute.value.path).toBe("/inbox");
  });

  it("leaves Enter to a focused button", async () => {
    const { wrapper, router } = await mountPage();
    await row(wrapper, "Sharpen the knife").trigger("click");

    await press("Enter", wrapper.get("button[aria-label='Refresh review requests']").element);

    expect(router.currentRoute.value.path).toBe("/inbox");
  });

  it("opens a PR's review on double-click", async () => {
    const { wrapper, router } = await mountPage();

    await row(wrapper, "Sharpen the knife").trigger("dblclick");
    await flushPromises();

    expect(router.currentRoute.value.path).toBe(reviewPath);
  });

  it("opens the previewed PR's review from Open review", async () => {
    const { wrapper, router } = await mountPage();
    await row(wrapper, "Sharpen the knife").trigger("click");

    const open = preview(wrapper).get("button");
    expect(open.text()).toBe("Open review");

    await open.trigger("click");
    await flushPromises();

    expect(router.currentRoute.value.path).toBe(reviewPath);
  });
});
