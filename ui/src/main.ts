import { createApp } from "vue";
import { createPinia } from "pinia";
import { createWebHashHistory } from "vue-router";
import App from "./App.vue";
import { createAppRouter } from "./router";
import { createThemeSetting, themeSettingKey } from "./theme";
import "./style.css";

// Applied before mounting, so the first paint already has the chosen theme.
const theme = createThemeSetting();

createApp(App)
  .provide(themeSettingKey, theme)
  .use(createAppRouter(createWebHashHistory()))
  .use(createPinia())
  .mount("#app");
