// PROTOTYPE: shared colour helpers.
import type { Severity } from '../data/mock'

export const sevDot: Record<Severity, string> = {
  high: 'bg-red-500', medium: 'bg-amber-500', low: 'bg-sky-500', info: 'bg-zinc-400',
}
export const sevText: Record<Severity, string> = {
  high: 'text-red-500', medium: 'text-amber-500', low: 'text-sky-500', info: 'text-zinc-400',
}
export const ciColor = { passing: 'text-emerald-500', failing: 'text-red-500', pending: 'text-amber-500' }
export const reviewLabel = { none: '—', approved: 'Approved', changes: 'Changes requested', commented: 'Commented' }
export const btn = 'inline-flex items-center gap-1.5 rounded-md border px-2.5 py-1 text-xs font-medium hover:bg-accent'
export const btnPrimary = 'inline-flex items-center gap-1.5 rounded-md bg-primary px-2.5 py-1 text-xs font-medium text-primary-foreground hover:opacity-90'
