// PROTOTYPE: fake data for Explore and Projects (PLAN.md §5.9).

export interface TreeNode { name: string; dir?: boolean; open?: boolean; children?: TreeNode[]; active?: boolean; bookmark?: boolean }

export const paymentsTree: TreeNode[] = [
  { name: 'cmd', dir: true, children: [{ name: 'server', dir: true }] },
  { name: 'internal', dir: true, open: true, children: [
    { name: 'billing', dir: true },
    { name: 'config', dir: true, open: true, children: [{ name: 'env.go' }, { name: 'env_test.go' }] },
    { name: 'scheduler', dir: true },
    { name: 'store', dir: true },
    { name: 'webhook', dir: true, open: true, children: [
      { name: 'client.go' }, { name: 'deliver.go', active: true, bookmark: true }, { name: 'deliver_test.go' }, { name: 'signing.go' },
    ] },
  ] },
  { name: 'migrations', dir: true },
  { name: 'go.mod' }, { name: 'go.sum' }, { name: 'Makefile' }, { name: 'README.md' },
]

export const infraTree: TreeNode[] = [
  { name: 'k8s', dir: true, open: true, children: [
    { name: 'payments', dir: true, open: true, children: [{ name: 'deployment.yaml', active: true }, { name: 'service.yaml' }, { name: 'configmap.yaml' }] },
    { name: 'ledger', dir: true },
  ] },
  { name: 'terraform', dir: true },
  { name: 'README.md' },
]

export interface CodeLine { n: number; text: string; mark?: 'focus' | 'link' }

export const deliverGo: CodeLine[] = [
  'package webhook', '', 'import (', '\t"bytes"', '\t"context"', '\t"encoding/json"', '\t"fmt"', '\t"net/http"', ')', '',
  '// Deliverer posts invoice events to a customer\'s webhook URL.', 'type Deliverer struct {', '\tclient *http.Client', '\tstore  AttemptStore',
  '\tretry  *Retrier', '\tlog    *slog.Logger', '}', '',
  'func (d *Deliverer) Deliver(ctx context.Context, inv Invoice) error {', '\tpayload, err := json.Marshal(inv.Event())', '\tif err != nil {',
  '\t\treturn fmt.Errorf("marshal invoice event: %w", err)', '\t}', '\tattempt, err := d.store.NextAttempt(ctx, inv.ID)', '\tif err != nil {', '\t\treturn err', '\t}',
  '\tresp, err := d.client.Post(inv.WebhookURL, "application/json", bytes.NewReader(payload))',
].map((text, i) => ({ n: i + 1, text, mark: i + 1 === 28 ? 'focus' : undefined }))

export const envGo: CodeLine[] = [
  'package config', '', 'type Env struct {', '\tDatabaseURL   string `env:"DATABASE_URL"`', '\tWebhookTimeout time.Duration `env:"WEBHOOK_TIMEOUT" default:"10s"`',
  '\tRetryMax      int    `env:"WEBHOOK_RETRY_MAX" default:"5"`', '\tListenAddr    string `env:"LISTEN_ADDR" default:":8080"`', '}', '',
  'func Load() (Env, error) {', '\tvar e Env', '\treturn e, envconfig.Process("", &e)', '}',
].map((text, i) => ({ n: i + 1, text, mark: i + 1 === 6 ? 'link' : i + 1 === 5 ? 'focus' : undefined }))

export const deploymentYaml: CodeLine[] = [
  'apiVersion: apps/v1', 'kind: Deployment', 'metadata:', '  name: payments', 'spec:', '  replicas: 3', '  template:', '    spec:', '      containers:',
  '        - name: payments', '          image: ghcr.io/acme/payments:v2.14.0', '          env:', '            - name: WEBHOOK_TIMEOUT', '              value: "30s"',
  '            - name: WEBHOOK_RETRY_MAX', '              valueFrom:', '                configMapKeyRef: { name: payments, key: retry_max }',
].map((text, i) => ({ n: i + 1, text, mark: i + 1 === 13 || i + 1 === 14 ? 'link' : undefined }))

export const blame = { line: 28, text: 'mkowalski · 2d · #4821 retry failed invoice webhooks' }

export interface TraceStep { repo: string; loc: string; code: string; note: string; inferred?: boolean; broken?: boolean }

export const trace: TraceStep[] = [
  { repo: 'payments', loc: 'internal/webhook/deliver.go:28', code: 'd.client.Post(inv.WebhookURL, …)', note: 'client timeout comes from d.client' },
  { repo: 'payments', loc: 'cmd/server/main.go:41', code: '&http.Client{Timeout: cfg.WebhookTimeout}', note: 'set from config at startup' },
  { repo: 'payments', loc: 'internal/config/env.go:5', code: 'WebhookTimeout … env:"WEBHOOK_TIMEOUT" default:"10s"', note: 'read from the environment, default 10s' },
  { repo: 'infra-deploy', loc: 'k8s/payments/deployment.yaml:13', code: '- name: WEBHOOK_TIMEOUT  value: "30s"', note: 'matched on env var name', inferred: true },
]

export const projects = [
  { name: 'checkout-stack', repos: [{ name: 'acme/payments', ref: 'main' }, { name: 'acme/infra-deploy', ref: 'v2.14.0' }] },
  { name: 'ledger', repos: [{ name: 'acme/ledger', ref: 'main' }, { name: 'acme/infra-deploy', ref: 'main' }] },
]

export const bookmarks = [
  { repo: 'payments', loc: 'internal/webhook/deliver.go:28', snippet: 'resp, err := d.client.Post(' },
  { repo: 'payments', loc: 'internal/store/attempts.go:61', snippet: 'func (s *Store) NextAttempt(', stale: true },
  { repo: 'infra-deploy', loc: 'k8s/payments/deployment.yaml:11', snippet: 'image: ghcr.io/acme/payments' },
]

export const steering = [
  { lens: 'Style', text: 'Structured log keys use snake_case ids (invoice_id, not invoice).', from: 'your comment on #4799', merge: undefined as string | undefined },
  { lens: 'General', text: 'Every new retry loop must honour ctx.Done().', from: 'your comment on #4802', merge: 'Merge with: "Long-running loops select on ctx.Done()"' },
]
