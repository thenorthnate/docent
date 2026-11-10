// PROTOTYPE: fake data for the UI mockups. Shaped loosely on PLAN.md §4 and §6, not the real types.

export type Severity = 'high' | 'medium' | 'low' | 'info'

export interface Finding {
  id: string
  lens: string
  severity: Severity
  file: string
  line: number
  title: string
  rationale: string
  suggestion?: string
  steeringRule?: string
  unanchored?: boolean
}

export interface DiffLine {
  kind: 'ctx' | 'add' | 'del' | 'hunk' | 'moved'
  old?: number
  new?: number
  text: string
}

export interface FileEntry {
  path: string
  status: 'M' | 'A' | 'D' | 'R'
  add: number
  del: number
  viewed: boolean
  findings: number
  comments: number
}

export interface Lens {
  key: string
  name: string
  on: boolean
  rules: number
  state: 'done' | 'running' | 'idle' | 'off'
  count?: number
}

export interface Thread {
  id: string
  file: string
  line: number
  author: string
  pending?: boolean
  body: string
  replies?: number
  isNew?: boolean
}

export const pr = {
  number: 4821,
  title: 'feat(billing): retry failed invoice webhooks with backoff',
  repo: 'acme/payments',
  author: 'mkowalski',
  head: 'feat/webhook-retry',
  base: 'main',
  ci: 'passing' as const,
  newSinceReview: { commits: 3, comments: 5 },
  trusted: true,
  jira: ['PAY-1182'],
  body: 'Invoice webhooks that fail with a 5xx are now retried up to 5 times with exponential backoff and jitter. Adds a `webhook_attempts` table so retries survive restarts.\n\nCloses PAY-1182.',
  summary:
    'Adds a retry loop around `deliverWebhook` with exponential backoff (base 2s, cap 5 min) and persists attempts in a new `webhook_attempts` table. The scheduler now picks up due retries every 30s. Most risk is in `retry.go`: the attempt counter is incremented before the send, so a crash mid-send skips an attempt.',
}

export const files: FileEntry[] = [
  { path: 'internal/webhook/retry.go', status: 'A', add: 118, del: 0, viewed: false, findings: 3, comments: 1 },
  { path: 'internal/webhook/deliver.go', status: 'M', add: 24, del: 31, viewed: false, findings: 1, comments: 2 },
  { path: 'internal/webhook/backoff.go', status: 'A', add: 41, del: 0, viewed: true, findings: 0, comments: 0 },
  { path: 'internal/scheduler/jobs.go', status: 'M', add: 12, del: 3, viewed: true, findings: 1, comments: 0 },
  { path: 'internal/store/migrations/0042_webhook_attempts.sql', status: 'A', add: 14, del: 0, viewed: false, findings: 0, comments: 0 },
  { path: 'internal/webhook/retry_test.go', status: 'A', add: 96, del: 0, viewed: false, findings: 1, comments: 0 },
  { path: 'go.mod', status: 'M', add: 1, del: 0, viewed: true, findings: 0, comments: 0 },
]

export const currentFile = 'internal/webhook/deliver.go'

export const diff: DiffLine[] = [
  { kind: 'hunk', text: '@@ -38,31 +38,24 @@ func (d *Deliverer) Deliver(ctx context.Context, inv Invoice) error {' },
  { kind: 'ctx', old: 38, new: 38, text: '\tpayload, err := json.Marshal(inv.Event())' },
  { kind: 'ctx', old: 39, new: 39, text: '\tif err != nil {' },
  { kind: 'ctx', old: 40, new: 40, text: '\t\treturn fmt.Errorf("marshal invoice event: %w", err)' },
  { kind: 'ctx', old: 41, new: 41, text: '\t}' },
  { kind: 'del', old: 42, text: '\tfor i := 0; i < 3; i++ {' },
  { kind: 'del', old: 43, text: '\t\tresp, err := d.client.Post(inv.WebhookURL, "application/json", bytes.NewReader(payload))' },
  { kind: 'del', old: 44, text: '\t\tif err == nil && resp.StatusCode < 500 {' },
  { kind: 'del', old: 45, text: '\t\t\treturn nil' },
  { kind: 'del', old: 46, text: '\t\t}' },
  { kind: 'del', old: 47, text: '\t\ttime.Sleep(time.Second)' },
  { kind: 'del', old: 48, text: '\t}' },
  { kind: 'add', new: 42, text: '\tattempt, err := d.store.NextAttempt(ctx, inv.ID)' },
  { kind: 'add', new: 43, text: '\tif err != nil {' },
  { kind: 'add', new: 44, text: '\t\treturn err' },
  { kind: 'add', new: 45, text: '\t}' },
  { kind: 'add', new: 46, text: '\tresp, err := d.client.Post(inv.WebhookURL, "application/json", bytes.NewReader(payload))' },
  { kind: 'add', new: 47, text: '\tif err != nil || resp.StatusCode >= 500 {' },
  { kind: 'add', new: 48, text: '\t\treturn d.retry.Schedule(ctx, inv.ID, attempt)' },
  { kind: 'add', new: 49, text: '\t}' },
  { kind: 'ctx', old: 49, new: 50, text: '\tdefer resp.Body.Close()' },
  { kind: 'ctx', old: 50, new: 51, text: '' },
  { kind: 'moved', text: '⇄ Moved from internal/webhook/client.go:12–19 · exact move · 8 lines collapsed' },
  { kind: 'ctx', old: 59, new: 60, text: '\tif resp.StatusCode >= 400 {' },
  { kind: 'del', old: 60, text: '\t\tlog.Printf("webhook rejected: %d", resp.StatusCode)' },
  { kind: 'add', new: 61, text: '\t\td.log.Warn("webhook rejected", "invoice", inv.ID, "status", resp.StatusCode)' },
  { kind: 'ctx', old: 61, new: 62, text: '\t\treturn ErrRejected' },
  { kind: 'ctx', old: 62, new: 63, text: '\t}' },
  { kind: 'ctx', old: 63, new: 64, text: '\treturn nil' },
  { kind: 'ctx', old: 64, new: 65, text: '}' },
]

export const findings: Finding[] = [
  {
    id: 'f1', lens: 'Bugs', severity: 'high', file: 'internal/webhook/deliver.go', line: 47,
    title: 'Response body leaks on the retry path',
    rationale: 'When StatusCode >= 500 the function returns before `defer resp.Body.Close()` is registered, so the connection is never released. Under sustained 5xx this exhausts the client pool.',
    suggestion: 'resp, err := d.client.Post(...)\nif resp != nil {\n\tdefer resp.Body.Close()\n}',
  },
  {
    id: 'f2', lens: 'Bugs', severity: 'medium', file: 'internal/webhook/retry.go', line: 33,
    title: 'Attempt counter increments before send',
    rationale: 'A crash between NextAttempt and Post skips an attempt; with max 5 attempts a flaky host can lose a delivery.',
  },
  {
    id: 'f3', lens: 'Style', severity: 'low', file: 'internal/webhook/deliver.go', line: 61,
    title: 'Use the package logger key names',
    rationale: 'Other call sites in internal/webhook use "invoice_id", not "invoice" (see deliver.go:88).',
    steeringRule: 'Structured log keys use snake_case ids',
  },
  {
    id: 'f4', lens: 'Modernize', severity: 'low', file: 'internal/webhook/retry.go', line: 71,
    title: 'Use min() builtin (Go 1.21)',
    rationale: 'go.mod targets 1.22; `if d > max { d = max }` can be `d = min(d, max)`.',
    suggestion: 'd = min(d, maxBackoff)',
  },
  {
    id: 'f5', lens: 'Bugs', severity: 'medium', file: 'internal/scheduler/jobs.go', line: 22,
    title: 'Retry job ignores context cancellation',
    rationale: 'The ticker loop never selects on ctx.Done(), so shutdown waits for the 30s tick.',
  },
  {
    id: 'f6', lens: 'Bugs', severity: 'info', file: 'internal/webhook/retry_test.go', line: 0,
    title: 'No test for crash between attempt and send',
    rationale: 'The agent cited line 140, which does not exist in this file.',
    unanchored: true,
  },
]

export const threads: Thread[] = [
  { id: 't1', file: 'internal/webhook/deliver.go', line: 42, author: 'jdiaz', body: 'Should NextAttempt be inside the same tx as the send record?', replies: 2, isNew: true },
  { id: 't2', file: 'internal/webhook/deliver.go', line: 48, author: 'you', pending: true, body: 'Can we log the attempt number here? It will make on-call much easier.' },
  { id: 't3', file: 'internal/webhook/retry.go', line: 12, author: 'mkowalski', body: 'Backoff constants come from the PAY-1182 spec.', isNew: true },
]

export const lenses: Lens[] = [
  { key: 'explain', name: 'Explain', on: true, rules: 0, state: 'idle' },
  { key: 'bugs', name: 'Bugs', on: true, rules: 2, state: 'done', count: 4 },
  { key: 'style', name: 'Style', on: true, rules: 4, state: 'done', count: 1 },
  { key: 'modernize', name: 'Modernize', on: true, rules: 0, state: 'running', count: 1 },
  { key: 'security', name: 'Secrets (repo)', on: false, rules: 0, state: 'off' },
  { key: 'chat', name: 'Chat', on: true, rules: 0, state: 'idle' },
]

export const explain = {
  target: 'deliver.go:42–49',
  what: 'Replaces the in-process 3× retry loop with a single attempt. On failure it hands the invoice to `retry.Schedule`, which persists the next attempt time so retries survive a restart.',
  why: 'PAY-1182: customers saw duplicate invoices when the old loop retried after a slow 200. Persisting attempts lets the scheduler dedupe.',
  related: [
    { label: 'retry.go:28 Schedule', kind: 'callee' },
    { label: 'jobs.go:19 runDueRetries', kind: 'caller' },
    { label: '#4310 introduced the 3× loop', kind: 'history' },
  ],
}

export const activity = [
  { t: '10:42:03', who: 'Bugs', text: 'read internal/webhook/retry.go', kind: 'tool' },
  { t: '10:42:05', who: 'Bugs', text: 'permission: shell `go test ./internal/webhook` → denied (not trusted for verify)', kind: 'deny' },
  { t: '10:42:19', who: 'Bugs', text: 'returned 4 findings (1 unanchored)', kind: 'ok' },
  { t: '10:42:20', who: 'Modernize', text: 'running gofmt -l (allowlisted)', kind: 'tool' },
]

export const steeringSuggestions = 2

export interface InboxPr {
  n: number
  title: string
  repo: string
  author: string
  ci: 'passing' | 'failing' | 'pending'
  review: 'none' | 'approved' | 'changes' | 'commented'
  size: string
  lines: number
  age: string
  updated?: string
  draft?: boolean
  findings?: number
  trusted: boolean
  fork?: boolean
}

export const inbox: Record<string, InboxPr[]> = {
  'Review requested': [
    { n: 4821, title: 'feat(billing): retry failed invoice webhooks with backoff', repo: 'acme/payments', author: 'mkowalski', ci: 'passing', review: 'commented', size: 'M', lines: 305, age: '2d', updated: '+3 commits · 5 comments', findings: 6, trusted: true },
    { n: 4830, title: 'fix(api): clamp page size to 200', repo: 'acme/payments', author: 'tlee', ci: 'pending', review: 'none', size: 'XS', lines: 8, age: '3h', findings: 0, trusted: true },
    { n: 212, title: 'chore: bump terraform aws provider to 5.80', repo: 'acme/infra-deploy', author: 'renovate[bot]', ci: 'failing', review: 'none', size: 'S', lines: 44, age: '1d', trusted: true },
    { n: 77, title: 'Add Dutch translations', repo: 'acme/web-checkout', author: 'outside-contrib', ci: 'passing', review: 'none', size: 'L', lines: 1210, age: '5d', trusted: false, fork: true },
  ],
  Assigned: [
    { n: 4799, title: 'refactor(store): split invoice repo into read/write', repo: 'acme/payments', author: 'jdiaz', ci: 'passing', review: 'changes', size: 'XL', lines: 2240, age: '6d', updated: '+1 commit', trusted: true },
  ],
  'Authored by me': [
    { n: 4815, title: 'feat(ledger): export monthly statements as CSV', repo: 'acme/payments', author: 'you', ci: 'failing', review: 'approved', size: 'M', lines: 380, age: '4d', updated: '2 comments', trusted: true },
    { n: 210, title: 'ci: cache go modules in deploy job', repo: 'acme/infra-deploy', author: 'you', ci: 'passing', review: 'none', size: 'XS', lines: 12, age: '1h', draft: true, trusted: true },
  ],
  Mentioned: [
    { n: 4788, title: 'docs: on-call runbook for webhook failures', repo: 'acme/payments', author: 'sra', ci: 'passing', review: 'none', size: 'S', lines: 90, age: '1w', trusted: true },
  ],
  Pinned: [],
}

export const filtered = { count: 2, repo: 'acme/legacy-billing' }
export const unbound = { repo: 'acme/web-checkout' }
