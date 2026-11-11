import { createApp } from "vue";
import { createPinia } from "pinia";
import { createWebHashHistory } from "vue-router";
import App from "./App.vue";
import { createAppRouter } from "./router";
import { followSystemTheme } from "./theme";
import "./style.css";

followSystemTheme();

createApp(App)
  .use(createAppRouter(createWebHashHistory()))
  .use(createPinia())
  .mount("#app");
