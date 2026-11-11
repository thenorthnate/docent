import { createRouter, type RouterHistory } from "vue-router";
import InboxPage from "@/views/InboxPage.vue";
import InboxDrawer from "@/views/InboxDrawer.vue";
import ReviewPage from "@/views/ReviewPage.vue";
import ReviewDrawer from "@/views/ReviewDrawer.vue";
import SettingsPage from "@/views/SettingsPage.vue";
import SettingsDrawer from "@/views/SettingsDrawer.vue";

export type Area = "inbox" | "review" | "settings";

declare module "vue-router" {
  interface RouteMeta {
    area?: Area;
  }
}

export function createAppRouter(history: RouterHistory) {
  return createRouter({
    history,
    routes: [
      { path: "/", redirect: "/inbox" },
      {
        path: "/inbox",
        name: "inbox",
        components: { default: InboxPage, drawer: InboxDrawer },
        meta: { area: "inbox" },
      },
      {
        path: "/review",
        name: "review",
        components: { default: ReviewPage, drawer: ReviewDrawer },
        meta: { area: "review" },
      },
      {
        path: "/settings",
        name: "settings",
        components: { default: SettingsPage, drawer: SettingsDrawer },
        meta: { area: "settings" },
      },
    ],
  });
}
