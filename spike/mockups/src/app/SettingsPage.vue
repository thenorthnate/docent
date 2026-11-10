<script setup lang="ts">
// PROTOTYPE: Settings. The drawer lists app-wide sections, then one entry per watched repo
// (PLAN.md's "Repo Settings"). Covers §5.1 accounts, profiles and permission modes (§5.6),
// trust, lens library (§5.12), steering (§5.13), verification (§5.15), notifications (M5), data.
import { computed, ref } from 'vue'
import { Bell, Bot, Database, FolderGit2, KeyRound, Library, Plus, RotateCcw, ShieldCheck, TriangleAlert, User } from '@lucide/vue'
import Shell from './Shell.vue'
import { btn, btnPrimary } from '../components/bits'

const appSections = [
  { key: 'accounts', label: 'Accounts', icon: User },
  { key: 'profiles', label: 'Agent profiles', icon: Bot },
  { key: 'lenses', label: 'Lens library', icon: Library },
  { key: 'trust', label: 'Trust', icon: ShieldCheck },
  { key: 'notifications', label: 'Notifications', icon: Bell },
  { key: 'data', label: 'Data & storage', icon: Database },
]
const repos = ['acme/payments', 'acme/infra-deploy', 'acme/web-checkout']
const section = ref('profiles')
const repoTab = ref('lenses')
const isRepo = computed(() => section.value.startsWith('repo:'))
const repoName = computed(() => section.value.slice(5))

const profiles = [
  { name: 'Kiro (work)', cmd: 'kiro-cli acp --agent docent-review', account: 'github.acme.com', mode: 'managed', tools: ['jira.search', 'jira.get_issue'], limit: 2, usage: 'from _kiro.dev/metadata' },
  { name: 'Claude Code (personal)', cmd: 'claude-agent-acp', account: 'github.com', mode: 'managed', tools: [], limit: 3, usage: 'from usage_update' },
]
const selProfile = ref(0)
const mode = ref('managed')

const library = [
  { name: 'Explain', source: 'default', state: 'locked', note: 'Can be edited, not disabled' },
  { name: 'Bugs', source: 'default', state: 'on', rules: 3 },
  { name: 'Style', source: 'your copy', state: 'on', rules: 6, newer: true },
  { name: 'Modernize & lint', source: 'default', state: 'paused', rules: 0 },
  { name: 'Migrations safety', source: 'yours', state: 'on', rules: 1 },
  { name: 'Chat', source: 'default', state: 'locked', note: 'Can be edited, not disabled' },
]
const repoLenses = [
  { name: 'Bugs', from: 'global', on: true, rules: [{ text: 'Every new retry loop must honour ctx.Done()', src: 'from comment on #4802', scope: 'repo' }, { text: 'Prefer errors.Is over ==', src: 'global', scope: 'global' }], cost: '~1.9k tokens' },
  { name: 'Style', from: 'global', on: true, rules: [{ text: 'Structured log keys use snake_case ids', src: 'from comment on #4799', scope: 'repo' }], cost: '~1.2k tokens' },
  { name: 'Modernize & lint', from: 'global', on: true, rules: [], cost: '~0.9k tokens' },
  { name: 'Secrets', from: 'repo version (.docent/lenses)', on: false, rules: [], cost: '~0.6k tokens' },
  { name: 'Ledger invariants', from: 'repo-only', on: true, rules: [], cost: '~0.8k tokens' },
]
const notif = ['New review requests', 'New commits on PRs you reviewed', 'Comments mentioning or replying to you', 'CI failed on your PR', 'Your PR approved or changes requested']
const h2 = 'text-base font-semibold'
const card = 'rounded-lg border p-4'
const label = 'text-[11px] font-medium uppercase tracking-wide text-muted-foreground'
const input = 'w-full rounded-md border bg-background px-2 py-1 font-mono text-xs'
</script>

<template>
  <Shell>
    <template #drawer>
      <div class="px-3 py-2 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">Settings</div>
      <div class="px-1.5">
        <button v-for="s in appSections" :key="s.key" class="flex w-full items-center gap-2 rounded-md px-2 py-1" :class="section === s.key ? 'bg-accent' : 'text-muted-foreground hover:bg-accent/50'" @click="section = s.key">
          <component :is="s.icon" class="size-3.5" />{{ s.label }}
        </button>
      </div>
      <div class="mt-4 px-3 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">Repos</div>
      <div class="px-1.5">
        <button v-for="r in repos" :key="r" class="flex w-full items-center gap-2 rounded-md px-2 py-1" :class="section === 'repo:' + r ? 'bg-accent' : 'text-muted-foreground hover:bg-accent/50'" @click="section = 'repo:' + r">
          <FolderGit2 class="size-3.5" />{{ r.split('/')[1] }}
          <ShieldCheck v-if="r !== 'acme/web-checkout'" class="ml-auto size-3 text-emerald-500" /><span v-else class="ml-auto text-[10px] text-amber-500">untrusted</span>
        </button>
        <button class="flex items-center gap-1 px-2 py-1 text-[11px] text-sky-400"><Plus class="size-3" /> Watch a repo</button>
      </div>
    </template>

    <main class="min-h-0 flex-1 overflow-auto">
      <div class="mx-auto max-w-3xl space-y-4 p-6 pb-24">
        <!-- ACCOUNTS -->
        <template v-if="section === 'accounts'">
          <h2 :class="h2">Accounts</h2>
          <div v-for="a in [{ host: 'github.acme.com', user: 'nnorth-acme', via: 'gh auth token', ok: true }, { host: 'github.com', user: 'nnorth', via: 'gh auth token', ok: true }]" :key="a.host" :class="card" class="flex items-center gap-3">
            <KeyRound class="size-4 text-muted-foreground" />
            <div><div class="font-medium">{{ a.host }} · {{ a.user }}</div><div class="text-xs text-muted-foreground">Token from <code>{{ a.via }}</code>, held in memory only · SSO ok</div></div>
            <span class="ml-auto text-xs text-emerald-500">Connected</span><button :class="btn">Re-check</button>
          </div>
          <div :class="card"><div class="font-medium">Add an account</div>
            <p class="mt-1 text-xs text-muted-foreground">Docent uses the <code>gh</code> CLI if it's signed in to the host. Otherwise, use a personal access token, stored in the OS keychain.</p>
            <div class="mt-2 flex gap-2"><input :class="input" class="max-w-xs" placeholder="host, e.g. github.com" /><button :class="btn">Use gh</button><button :class="btn">Use a PAT…</button></div></div>
        </template>

        <!-- PROFILES -->
        <template v-else-if="section === 'profiles'">
          <div class="flex items-center"><h2 :class="h2">Agent profiles</h2><button :class="btn" class="ml-auto"><Plus class="size-3.5" /> New profile</button><button :class="btn" class="ml-2">Detect installed agents</button></div>
          <p class="text-xs text-muted-foreground">How Docent launches an ACP agent. Each forge account uses one profile; repos can override it. If nothing matches, no agent runs.</p>
          <div class="flex gap-2">
            <button v-for="(p, i) in profiles" :key="p.name" class="rounded-md border px-3 py-1.5 text-left" :class="selProfile === i ? 'border-ring bg-accent' : ''" @click="selProfile = i">
              <div class="font-medium">{{ p.name }}</div><div class="text-[11px] text-muted-foreground">default for {{ p.account }}</div></button>
          </div>
          <div :class="card" class="space-y-3">
            <div><div :class="label">Command</div><input :class="input" :value="profiles[selProfile].cmd" /></div>
            <div class="grid grid-cols-2 gap-3">
              <div><div :class="label">Environment</div><input :class="input" value="KIRO_LOG=warn" /><div class="mt-1 text-[11px] text-muted-foreground">Forge tokens are always removed.</div></div>
              <div><div :class="label">Concurrent runs</div><input :class="input" :value="profiles[selProfile].limit" /></div>
            </div>
            <div>
              <div :class="label">Permission mode</div>
              <label class="mt-1 flex gap-2 rounded-md border p-2" :class="mode === 'managed' ? 'border-ring' : ''"><input type="radio" value="managed" v-model="mode" />
                <span><b>Docent-managed</b> (recommended)<br /><span class="text-xs text-muted-foreground">Docent answers every permission request: read-only tools, the MCP allowlist below, and allowlisted commands in the verify worktree. Web search and fetch are off.</span></span></label>
              <label class="mt-1.5 flex gap-2 rounded-md border p-2" :class="mode === 'inherit' ? 'border-amber-500' : ''"><input type="radio" value="inherit" v-model="mode" />
                <span><b>Inherit harness settings</b> (user level only)<br /><span class="text-xs text-muted-foreground">Uses your own agent settings, never the repo's. Docent only denies writes outside the verify worktree.</span></span></label>
              <div v-if="mode === 'inherit'" class="mt-1.5 flex gap-2 rounded-md bg-amber-500/10 p-2 text-xs text-amber-500"><TriangleAlert class="size-4 shrink-0" />Anything your harness auto-allows never reaches Docent, so Docent can't limit it.</div>
            </div>
            <div><div :class="label">MCP tool allowlist</div>
              <div class="mt-1 flex flex-wrap gap-1.5"><span v-for="t in profiles[selProfile].tools" :key="t" class="rounded-full border px-2 py-0.5 font-mono text-[11px]">{{ t }} ×</span>
                <span v-if="!profiles[selProfile].tools.length" class="text-xs text-muted-foreground">None. The agent gets no MCP tools.</span><button class="text-[11px] text-sky-400">+ Add tool</button></div></div>
            <div class="flex items-center gap-2 border-t pt-3 text-xs text-muted-foreground">Usage reported {{ profiles[selProfile].usage }} <button :class="btn" class="ml-auto">Test launch</button></div>
          </div>
        </template>

        <!-- LENS LIBRARY -->
        <template v-else-if="section === 'lenses'">
          <div class="flex items-center"><h2 :class="h2">Lens library</h2><button :class="btn" class="ml-auto"><Plus class="size-3.5" /> New lens</button><button :class="btn" class="ml-2"><RotateCcw class="size-3.5" /> Restore defaults</button></div>
          <p class="text-xs text-muted-foreground">Global lenses, available in every repo. Turn them on or off per repo under Repos. Global steering rules show here too.</p>
          <div class="divide-y rounded-lg border">
            <div v-for="l in library" :key="l.name" class="flex items-center gap-3 px-4 py-2.5">
              <div class="min-w-0"><div class="font-medium">{{ l.name }}</div>
                <div class="text-[11px] text-muted-foreground">{{ l.source }}<template v-if="l.rules"> · {{ l.rules }} global rules</template><template v-if="l.note"> · {{ l.note }}</template></div>
                <div v-if="l.newer" class="mt-0.5 text-[11px] text-sky-400">A newer default is available · <u>View diff</u></div></div>
              <div class="ml-auto flex items-center gap-2 text-xs">
                <span v-if="l.state === 'paused'" class="rounded bg-amber-500/15 px-1.5 text-amber-500">paused everywhere</span>
                <button :class="btn">Edit</button>
                <button v-if="l.state !== 'locked'" :class="btn">{{ l.state === 'paused' ? 'Resume' : 'Pause everywhere' }}</button>
                <button v-if="l.state !== 'locked'" :class="btn">Delete</button>
              </div>
            </div>
          </div>
          <label class="flex items-center gap-2 text-xs"><input type="checkbox" checked /> Generate an AI summary when a PR opens</label>
          <div class="text-[11px] text-muted-foreground">Deleted defaults: <span class="rounded bg-muted px-1">Perf hints</span> (tombstoned, won't return on update)</div>
        </template>

        <!-- TRUST -->
        <template v-else-if="section === 'trust'">
          <h2 :class="h2">Trust</h2>
          <p class="text-xs text-muted-foreground">Agents and linters only run on code you trust. PR content is treated as hostile until then. Fork PRs need trust per PR, and it resets on each push.</p>
          <div class="divide-y rounded-lg border">
            <div v-for="t in [{ what: 'acme/payments', kind: 'repo', verify: true }, { what: 'acme/infra-deploy', kind: 'repo', verify: false }, { what: 'acme/ledger · branch release/*', kind: 'branch', verify: false }, { what: 'acme/web-checkout #61', kind: 'PR (fork) · until next push', verify: false }]" :key="t.what" class="flex items-center gap-3 px-4 py-2.5">
              <ShieldCheck class="size-4 text-emerald-500" /><div><div class="font-medium">{{ t.what }}</div><div class="text-[11px] text-muted-foreground">{{ t.kind }}</div></div>
              <label class="ml-auto flex items-center gap-1.5 text-xs" :class="t.kind.startsWith('PR') ? 'opacity-40' : ''"><input type="checkbox" :checked="t.verify" :disabled="t.kind.startsWith('PR')" /> Allow verification</label>
              <button :class="btn">Revoke</button>
            </div>
          </div>
        </template>

        <!-- NOTIFICATIONS -->
        <template v-else-if="section === 'notifications'">
          <h2 :class="h2">Notifications</h2>
          <div :class="card" class="space-y-2">
            <label v-for="(n, i) in notif" :key="n" class="flex items-center gap-2"><input type="checkbox" :checked="i !== 1" /> {{ n }}</label>
          </div>
          <div :class="card" class="space-y-2">
            <label class="flex items-center gap-2"><input type="radio" name="close" checked /> Closing the window keeps Docent in the tray</label>
            <label class="flex items-center gap-2"><input type="radio" name="close" /> Closing the window quits Docent</label>
            <label class="flex items-center gap-2"><input type="checkbox" checked /> Sync "Viewed" files with GitHub</label>
          </div>
        </template>

        <!-- DATA -->
        <template v-else-if="section === 'data'">
          <h2 :class="h2">Data & storage</h2>
          <div :class="card" class="space-y-2">
            <div class="flex items-center gap-2">Keep agent transcripts for <input :class="input" class="!w-14" value="30" /> days, or until their worktree is removed</div>
            <div class="text-xs text-muted-foreground">Findings are kept. Transcripts include prompts, tool calls and permission decisions.</div>
          </div>
          <div :class="card"><div class="font-medium">Worktrees</div>
            <div class="mt-2 divide-y text-xs">
              <div v-for="w in [['payments #4821', 'review + verify', '412 MB'], ['payments #4830', 'review', '206 MB'], ['payments @ main', 'explore', '205 MB'], ['infra-deploy @ v2.14.0', 'explore', '38 MB']]" :key="w[0]" class="flex items-center gap-3 py-1.5">
                <span class="font-medium">{{ w[0] }}</span><span class="text-muted-foreground">{{ w[1] }}</span><span class="ml-auto text-muted-foreground">{{ w[2] }}</span><button :class="btn">Remove</button></div>
            </div></div>
          <div class="flex items-center gap-3 rounded-lg border border-red-500/40 p-4"><div><div class="font-medium">Clear all</div><div class="text-xs text-muted-foreground">Deletes transcripts, cached results and worktrees. Your clones keep their <code>refs/docent/*</code> until the next prune.</div></div><button class="ml-auto rounded-md border border-red-500/50 px-2.5 py-1 text-xs text-red-400">Clear all…</button></div>
        </template>

        <!-- REPO SETTINGS -->
        <template v-else-if="isRepo">
          <div class="flex items-center gap-2"><FolderGit2 class="size-5" /><h2 :class="h2">{{ repoName }}</h2>
            <span v-if="repoName === 'acme/web-checkout'" class="rounded bg-amber-500/15 px-1.5 text-xs text-amber-500">untrusted</span>
            <span v-else class="rounded bg-emerald-500/15 px-1.5 text-xs text-emerald-500">trusted</span></div>
          <div class="flex gap-1 border-b text-xs">
            <button v-for="t in [['binding', 'Clone & agent'], ['lenses', 'Lenses & steering'], ['verify', 'Trust & verification']]" :key="t[0]" class="border-b-2 px-2 py-1.5" :class="repoTab === t[0] ? 'border-primary' : 'border-transparent text-muted-foreground'" @click="repoTab = t[0]">{{ t[1] }}</button>
          </div>

          <template v-if="repoTab === 'binding'">
            <div :class="card" class="space-y-3">
              <div><div :class="label">Local clone</div><div class="mt-1 flex gap-2"><input :class="input" value="~/code/acme/payments" /><button :class="btn">Change…</button></div></div>
              <div><div :class="label">Agent profile</div><select class="mt-1 rounded-md border bg-background px-2 py-1 text-xs"><option>Use account default (Kiro (work))</option><option>Claude Code (personal)</option></select></div>
              <div><div :class="label">Auto-run lenses on PRs up to</div><div class="mt-1 flex items-center gap-2 text-xs"><input :class="input" class="!w-20" value="1500" /> changed lines and <input :class="input" class="!w-14" value="50" /> files</div></div>
            </div>
          </template>

          <template v-else-if="repoTab === 'lenses'">
            <p class="text-xs text-muted-foreground">Which lenses run in this repo, and the steering rules each one carries. A PR can override these from its review panel.</p>
            <div v-for="l in repoLenses" :key="l.name" class="rounded-lg border">
              <div class="flex items-center gap-2 px-4 py-2.5">
                <span class="font-medium">{{ l.name }}</span><span class="text-[11px] text-muted-foreground">{{ l.from }} · {{ l.cost }}</span>
                <button role="switch" class="relative ml-auto h-4 w-7 shrink-0 rounded-full" :class="l.on ? 'bg-emerald-500' : 'bg-muted'"><span class="absolute top-0.5 size-3 rounded-full bg-white" :class="l.on ? 'left-3.5' : 'left-0.5'" /></button>
              </div>
              <div v-for="r in l.rules" :key="r.text" class="flex items-center gap-2 border-t px-4 py-1.5 text-xs">
                <span>{{ r.text }}</span><span class="text-[11px] text-muted-foreground">{{ r.src }}</span>
                <span class="ml-auto flex gap-1"><button :class="btn" class="!py-0">Edit</button><button v-if="r.scope === 'repo'" :class="btn" class="!py-0">Promote to global</button><button :class="btn" class="!py-0">Delete</button></span>
              </div>
            </div>
            <div class="rounded-lg border p-3 text-xs"><b>General</b> rules (all review lenses): 1 · <button class="text-sky-400">Show</button><span class="ml-3 text-muted-foreground">Repo lenses from <code>.docent/lenses/</code> are read from the default branch only.</span></div>
          </template>

          <template v-else>
            <div :class="card" class="space-y-2">
              <label class="flex items-center gap-2"><input type="checkbox" :checked="repoName !== 'acme/web-checkout'" /> <b>Trust this repo</b><span class="text-xs text-muted-foreground">agents and linters may run on its PRs (fork PRs still need per-PR trust)</span></label>
              <label class="flex items-center gap-2"><input type="checkbox" :checked="repoName === 'acme/payments'" /> <b>Allow verification</b><span class="text-xs text-muted-foreground">agents may build and test PR code in a disposable worktree</span></label>
              <label class="flex items-center gap-2 pl-6"><input type="checkbox" /> Include verification in auto-runs</label>
            </div>
            <div :class="card"><div class="flex items-center"><div class="font-medium">Allowed commands</div><span class="ml-2 text-[11px] text-muted-foreground">detected from go.mod</span><button :class="btn" class="ml-auto">Re-detect</button></div>
              <p class="mt-1 text-xs text-muted-foreground">The agent may run only commands that start with one of these. Linters use the same list. Limit 10 minutes per command; Docent never installs toolchains.</p>
              <div class="mt-2 space-y-1"><div v-for="c in ['go build ./...', 'go test ./...', 'go vet ./...', 'golangci-lint run']" :key="c" class="flex items-center gap-2"><input :class="input" :value="c" /><button :class="btn" class="!py-0.5">×</button></div>
                <button class="text-[11px] text-sky-400">+ Add command</button></div></div>
          </template>
        </template>
      </div>
    </main>
  </Shell>
</template>
