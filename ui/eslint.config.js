import pluginVue from "eslint-plugin-vue";
import {
  defineConfigWithVueTs,
  vueTsConfigs,
} from "@vue/eslint-config-typescript";

export default defineConfigWithVueTs(
  { ignores: ["dist/**"] },
  pluginVue.configs["flat/recommended"],
  vueTsConfigs.recommended,
  {
    // shadcn-vue components are generator output: keep them as generated.
    files: ["src/components/ui/**/*.vue"],
    rules: {
      "vue/multi-word-component-names": "off",
      "vue/require-default-prop": "off",
    },
  },
);
