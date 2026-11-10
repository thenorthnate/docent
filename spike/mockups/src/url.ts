// PROTOTYPE: URL-backed state for the switcher (?screen=&variant=&trust=&theme=).
import { reactive } from 'vue'

const p = new URLSearchParams(location.search)
export const state = reactive({
  screen: p.get('screen') ?? 'app',
  variant: p.get('variant') ?? 'A',
  trust: p.get('trust') ?? 'trusted',
  theme: p.get('theme') ?? 'dark',
})

export function set(patch: Partial<typeof state>) {
  Object.assign(state, patch)
  const q = new URLSearchParams(state as Record<string, string>)
  history.replaceState(null, '', `?${q}`)
}

// Page inside the app shell (?screen=app).
export const page = reactive({ current: p.get('page') ?? 'inbox' })
export function go(name: string) {
  page.current = name
  const q = new URLSearchParams(location.search)
  q.set('page', name)
  history.replaceState(null, '', `?${q}`)
}
