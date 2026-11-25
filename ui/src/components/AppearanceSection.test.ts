import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import AppearanceSection from "./AppearanceSection.vue";
import { createThemeSetting, THEME_STORAGE_KEY, themeSettingKey } from "@/theme";
import { fakeDarkQuery, fakeStorage } from "@/theme.testing";

function setup({ stored, osDark = false }: { stored?: string; osDark?: boolean } = {}) {
  const root = document.createElement("html");
  const query = fakeDarkQuery(osDark);
  const storage = fakeStorage(stored ? { [THEME_STORAGE_KEY]: stored } : {});
  const items = storage.items;
  const theme = createThemeSetting({ root, query, storage });
  const wrapper = mount(AppearanceSection, { global: { provide: { [themeSettingKey]: theme } } });
  return { wrapper, root, query, items, isDark: () => root.classList.contains("dark") };
}

/** The radio button inside the `<label>` that reads `text`. */
function option(wrapper: ReturnType<typeof mount>, text: string) {
  const label = wrapper.findAll("label").find((candidate) => candidate.text() === text);
  if (!label) {
    throw new Error(`no option labelled ${text}`);
  }
  return label.get<HTMLInputElement>('input[type="radio"]');
}

describe("AppearanceSection", () => {
  it("offers System, Light and Dark, with System selected by default", () => {
    const { wrapper } = setup();

    expect(wrapper.get("h2").text()).toBe("Appearance");
    expect(wrapper.findAll("label").map((label) => label.text())).toEqual(["System", "Light", "Dark"]);
    expect(option(wrapper, "System").element.checked).toBe(true);
  });

  it("shows the stored choice", () => {
    const { wrapper } = setup({ stored: "dark" });

    expect(option(wrapper, "Dark").element.checked).toBe(true);
    expect(option(wrapper, "System").element.checked).toBe(false);
  });

  it.each([
    ["Light", "light", true, false],
    ["Dark", "dark", false, true],
  ] as const)("pins the theme when %s is chosen, whatever the OS says", async (label, value, osDark, dark) => {
    const { wrapper, query, items, isDark } = setup({ osDark });

    await option(wrapper, label).setValue(true);

    expect(isDark()).toBe(dark);
    expect(items.get(THEME_STORAGE_KEY)).toBe(value);
    query.change(!osDark);
    expect(isDark()).toBe(dark);
  });

  it("follows the OS again when switched back to System", async () => {
    const { wrapper, query, items, isDark } = setup({ stored: "light", osDark: true });
    expect(isDark()).toBe(false);

    await option(wrapper, "System").setValue(true);

    expect(items.get(THEME_STORAGE_KEY)).toBe("system");
    expect(isDark()).toBe(true);
    query.change(false);
    expect(isDark()).toBe(false);
  });
});
