// PROTOTYPE: in-memory state shared between the review page, its diff and the agent panel.
import { reactive } from 'vue'
import { pr } from '../data/mock'

export const review = reactive({
  enabled: { Bugs: true, Style: true, Modernize: true, 'Secrets (repo)': false } as Record<string, boolean>,
  expanded: { Bugs: true, Style: true, Modernize: true, 'Secrets (repo)': true } as Record<string, boolean>,  // groups start open; collapse is per user
  draftLine: null as number | null,                 // gutter click opens a new comment here
  pending: [{ id: 'p1', line: 48, kind: 'General', body: 'Can we log the attempt number here? It will make on-call much easier.' }] as
    { id: string; line: number; kind: string; body: string }[],
})

// Where Explore opens. The rail opens the default branch; "Explore this branch" opens the PR head.
export const explore = reactive({ ref: 'main', sha: 'a41f0e2', fromPr: false })
export function exploreBranch() {
  Object.assign(explore, { ref: pr.head, sha: '9f2c1a7', fromPr: true })
}
