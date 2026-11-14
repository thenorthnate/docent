import { describe, expect, it } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { createMemoryHistory } from "vue-router";
import ReviewPage from "./ReviewPage.vue";
import { createAppRouter } from "@/router";

async function mountPage(path: string) {
  const router = createAppRouter(createMemoryHistory());
  await router.push(path);
  await router.isReady();
  const wrapper = mount(ReviewPage, { global: { plugins: [router] } });
  return { wrapper, router };
}

describe("ReviewPage", () => {
  it("shows the opened PR's breadcrumb", async () => {
    const { wrapper } = await mountPage("/review/github.com/octocat/Hello-World/1347");

    const crumbs = wrapper.get("nav[aria-label=Breadcrumb]");
    expect(crumbs.findAll("li").map((c) => c.text())).toEqual(["Inbox", "octocat/Hello-World", "#1347"]);
    expect(crumbs.get("[aria-current=page]").text()).toBe("#1347");
  });

  it("leads back to the inbox from the breadcrumb", async () => {
    const { wrapper, router } = await mountPage("/review/github.com/octocat/Hello-World/1347");

    await wrapper.get("nav[aria-label=Breadcrumb] a").trigger("click");
    await flushPromises();

    expect(router.currentRoute.value.path).toBe("/inbox");
  });

  it("asks for a PR when none is open", async () => {
    const { wrapper } = await mountPage("/review");

    expect(wrapper.find("nav[aria-label=Breadcrumb]").exists()).toBe(false);
    expect(wrapper.text()).toContain("Open a pull request from the Inbox");
  });
});
