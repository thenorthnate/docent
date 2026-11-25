<script setup lang="ts">
import { inject } from "vue";
import { type ThemeChoice, themeSettingKey } from "@/theme";

/** Set up in `main.ts`, which applies the stored choice before mounting. */
const theme = inject(themeSettingKey);
if (!theme) {
  throw new Error("AppearanceSection needs the theme setting provided");
}

const options: { value: ThemeChoice; label: string }[] = [
  { value: "system", label: "System" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
];
</script>

<template>
  <section
    aria-labelledby="appearance-heading"
    class="space-y-3"
  >
    <h2
      id="appearance-heading"
      class="text-base font-semibold"
    >
      Appearance
    </h2>
    <fieldset class="flex w-fit rounded-lg border p-0.5 text-sm">
      <legend class="sr-only">
        Theme
      </legend>
      <label
        v-for="option in options"
        :key="option.value"
        class="cursor-pointer rounded-md px-3 py-1 has-checked:bg-accent has-checked:font-medium has-focus-visible:ring-2 has-focus-visible:ring-ring"
      >
        <input
          type="radio"
          name="theme"
          class="sr-only"
          :value="option.value"
          :checked="theme.choice.value === option.value"
          @change="theme.set(option.value)"
        >
        {{ option.label }}
      </label>
    </fieldset>
    <p class="text-sm text-muted-foreground">
      System follows your operating system's light or dark setting.
    </p>
  </section>
</template>
