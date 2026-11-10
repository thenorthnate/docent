// PROTOTYPE: URL-backed state for the switcher (?screen=&variant=&trust=&theme=).
import { reactive } from 'vue'

const p = new URLSearchParams(location.search)
export const state = reactive({
  screen: p.get('screen') ?? 'pr',
  variant: p.get('variant') ?? 'A',
  trust: p.get('trust') ?? 'trusted',
  theme: p.get('theme') ?? 'dark',
})

export function set(patch: Partial<typeof state>) {
  Object.assign(state, patch)
  const q = new URLSearchParams(state as Record<string, string>)
  history.replaceState(null, '', `?${q}`)
}
