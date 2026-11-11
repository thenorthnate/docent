import { defineStore } from "pinia";
import { ref } from "vue";
import { useRouter } from "vue-router";
import type { Area } from "@/router";

export const useShellStore = defineStore("shell", () => {
  const router = useRouter();
  const drawerOpen = ref(true);

  /** Rail click: the active area toggles its drawer; another area opens with its drawer. */
  async function selectArea(area: Area) {
    if (router.currentRoute.value.meta.area === area) {
      drawerOpen.value = !drawerOpen.value;
      return;
    }
    await router.push({ name: area });
    drawerOpen.value = true;
  }

  return { drawerOpen, selectArea };
});
