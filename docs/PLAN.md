# Docent — Project Plan (Draft 3)

> **Docent**: a guide who explains what you're looking at. A desktop app for AI-guided code review **and** code exploration, which drives whatever coding agent you already use.

**Status:** Agreed design, not yet built. Formerly named GAIT. Decisions and remaining unknowns are logged in [`OPEN-QUESTIONS.md`](./OPEN-QUESTIONS.md), and the evidence behind them is in [`RESEARCH.md`](./RESEARCH.md).

**First target:** one user at work: Apple silicon Mac, GitHub Enterprise Cloud with SSO, Kiro, and Jira through Kiro's MCP server. Public release comes after that works.

---

## 1. Goals

1. Review GitHub PRs from a desktop app without opening the browser (GitLab follows in 0.2).
2. Know immediately what changed since **you** last reviewed a PR: new commits, comments, review state.
3. Click the `?` on any line and get an explanation of *why* it is there and what it relates to, grounded in the **whole repository** (local clone), not just the diff.
4. Have a small team of agents (explain, bugs, style consistency, modernization/lint) do the legwork automatically in repos you trust. Add your own lenses, remove or pause the ones you don't want, and steer them with rules learned from your own review comments.
5. Write comments and submit reviews from the app.
6. Explore any branch or commit with the same line-level experience, even with no PR.
7. Trace a call path or data flow in one view, including across related repos (for example a service and its infrastructure repo).
8. **Agent-agnostic**: Kiro at work, Claude Code at home, Codex/Copilot/Gemini/anything ACP-capable.
9. Show moved code as **moved**, within and across files, not as a deletion plus an addition.
10. In repos you opt in, let agents **verify** their findings: build, run tests and linters, and write and run their own tests.

### Non-goals (v1)

- Not a code editor or IDE. Docent never edits your working tree. (Suggested fixes become review comments or suggestions.) Agent-written tests live only in a Docent-owned verify worktree and are reset before each run.
- No hosted backend or Docent server. Everything runs locally; Docent talks only to your forge, locally spawned agents, and GitHub Releases for updates.
- No model API keys inside Docent. Billing and auth stay with each agent.
- No Jira credentials inside Docent. Jira access goes through the agent's own MCP server.

---

## 2. Decisions

| Area | Decision |
|---|---|
| App framework | Tauri 2 (Rust backend, system webview) |
| Frontend | Vue 3 + TypeScript + Vite, Tailwind CSS v4, shadcn-vue |
| Diff/code viewer | CodeMirror 6 (`@codemirror/merge`), fed git's own hunks. Chosen by the M0 spike ([write-up](spikes/viewer-choice.md)) |
| Agent integration | ACP client behind an `AgentBackend` interface, so the way Docent talks to agents can change |
| Forges | GitHub first (`Forge` interface); GitLab in 0.2 |
| Forge auth | Token from `gh auth token --hostname <host>`, held in memory. PAT in the keychain as fallback and device flow, both at M8 |
| Issue tracking | `IssueTracker` interface, separate from `Forge`. Jira first, through the agent's MCP server |
| Local repos | Your existing clones, or a normal full clone made by Docent using your own git credentials; one git worktree per PR |
| Inbox scope | Watched repos only |
| Hostile content | PR content is hostile until you trust the repo, branch or PR |
| Agent permissions | Best-effort read-only, with an explicit MCP tool allowlist per profile. Per profile: **Docent-managed** (default) or **Inherit harness settings** (user level only) |
| Agent profiles | Per forge account with per-repo overrides; no global fallback |
| Lenses | Global library plus per-repo lens sets; custom lenses; defaults can be deleted (tombstoned); toggles per repo, per PR and global pause |
| Steering | Rules drawn from your submitted comments, **staged** for your approval, per repo or global |
| Verification | Opt-in per trusted repo; separate verify worktree; allowlisted commands; never installs toolchains; never on fork PRs |
| Moved code | Detected in `docent-git`, within and across files |
| Auto-run | Trusted, open, non-draft PRs that request your review or that you pinned, up to a size limit |
| Findings | Private. No auto-post in v1 |
| Audience | Open source (MIT), no telemetry by default |
| Identifiers | `com.thenorthnate.docent`, crates `docent-*`, repo config in `.docent/` |

---

## 3. Architecture overview

```
┌──────────────────────────── Tauri 2 app ─────────────────────────────┐
│  Vue 3 UI (webview)                                                   │
│   Inbox · PR view · Diff viewer · Explore view · Agent side panel     │
│        ▲ invoke() commands / events (tauri::ipc)                      │
│  ──────┼────────────────────────────────────────────────────────────  │
│  Rust core                                                            │
│   ┌──────────────┐ ┌─────────────┐ ┌──────────────┐ ┌──────────────┐  │
│   │ docent-forge │ │ docent-git  │ │ docent-agents│ │ docent-store │  │
│   │ GitHub API   │ │ worktrees,  │ │ AgentBackend,│ │ SQLite: state│  │
│   │ + poller     │ │ diffs, blame│ │ ACP, lenses  │ │ trust, cache │  │
│   └─────┬────────┘ └─────┬───────┘ └─────┬────────┘ └──────────────┘  │
└─────────┼────────────────┼───────────────┼────────────────────────────┘
          │HTTPS           │local          │stdio JSON-RPC (ACP)
     GitHub            your clones     kiro-cli acp · claude-agent-acp ·
                       + worktrees     codex-acp · copilot --acp · …
                                            │ agent's own MCP servers
                                            ▼
                                          Jira
```

### 3.1 Repository layout (Cargo workspace + pnpm)

```
docent/
├─ src-tauri/            # Tauri app shell: commands, events, window setup, plugins
├─ crates/
│  ├─ docent-core/       # shared domain types (Repo, PullRequest, Finding, Lens…)
│  ├─ docent-forge/      # Forge trait + github/ impl + test fake, poller
│  ├─ docent-git/        # worktree mgmt, diff/interdiff, blame, file reads
│  ├─ docent-agents/     # AgentBackend trait, ACP impl, profiles, lens runner, output parsing
│  └─ docent-store/      # SQLite schema + migrations + queries
├─ ui/                   # Vue 3 + TS + Vite + Tailwind
│  ├─ src/views/         # Inbox, PullRequest, Explore, Settings
│  ├─ src/components/    # DiffViewer, FileTree, AgentPanel, FindingCard, CommentBox…
│  └─ src/stores/        # Pinia stores
├─ lenses/               # built-in agent definitions (markdown + frontmatter)
├─ docs/
└─ .github/workflows/    # CI + release builds (tauri-action)
```

Splitting the Rust side into small crates keeps the forge, git and agent logic testable without Tauri, and lets a future CLI reuse them.

### 3.2 Interfaces that keep options open

- **`AgentBackend`**: start session (with one or more roots and a tool policy), stream a prompt, cancel. ACP is the only implementation for now. If Kiro can't run over ACP at work, only this module changes.
- **`Forge`**: GitHub now, GitLab in 0.2. A test fake keeps it from being shaped only around GitHub.
- **`IssueTracker`**: read and write tickets. The first implementation goes through an agent's MCP tools; direct Jira REST can be added later.

### 3.3 Off-the-shelf building blocks

| Need | Choice | Why |
|---|---|---|
| Scaffold | `create-tauri-app` (Vue + TS template) | Official, zero custom glue |
| UI state / routing | Pinia, Vue Router | Vue standard |
| Components | shadcn-vue on Reka UI | Tailwind-native, copy-in components, accessible |
| Diff & code view | CodeMirror 6 (`@codemirror/merge`) | Accepts an external diff (`diffConfig.override`), so it shows git's hunks; light and fast on large files. Chosen in M0 |
| Markdown in panels | `markdown-it` + Shiki for code blocks | Agent output is markdown |
| GitHub API | `octocrab` (REST + GraphQL) | Mature Rust client |
| Git operations | System `git` CLI via `tokio::process` | Worktrees, range-diff and blame are first-class in git |
| ACP | `agent-client-protocol` crate | Official Rust SDK |
| Persistence | SQLite via `sqlx` (migrations) | Local, zero-config |
| Secrets | `keyring` crate | Never store tokens in plaintext |
| OS notifications | `tauri-plugin-notification` | Official plugin |
| Auto-update | `tauri-plugin-updater` | Official plugin, has its own signing key |
| Settings files | `tauri-plugin-store` or TOML in app config dir | Simple |
| Logging | `tracing` + `tauri-plugin-log` | Debuggable agent sessions |

Apps launched from Finder don't inherit your shell's `PATH`, so Docent reads `PATH` from the login shell to find `gh`, `git` and `kiro-cli`. The probe uses POSIX syntax, so a fish login shell falls back to the app's `PATH` plus Homebrew's directories until M8.

---

## 4. Core concepts

- **Watched repo**: a forge repo you've told Docent to pay attention to. Everything else stays out of the inbox and notifications.
- **Repo binding**: a watched repo mapped to a local clone. **Bind** points at an existing clone; **Clone** makes a normal full clone into a folder you pick, using your existing git credentials (for example `gh auth setup-git`). Docent never hands a forge token to git. Diffs always come from local git.
- **Worktree**: a git worktree Docent creates under its own data dir, in detached HEAD at the PR's head SHA. Agents run with `cwd` set to it, so they see the full repo at the reviewed commit. Your working tree is untouched, though your clone's `.git` gains `refs/docent/*` refs and worktree metadata.
- **Trust**: your local decision that a repo, branch or PR may be given to agents and linters (see 5.6).
- **Agent profile**: how to launch an ACP agent (command, args, env), plus its MCP tool allowlist. Assigned per forge account, with optional per-repo overrides. If nothing matches, no agent runs and Docent asks you to choose.
- **Lens**: a reusable review job: a prompt template, an expected output schema, and when it runs. Lenses are markdown files with frontmatter. They come from the **global library** (defaults and your own) or are **repo-only** (see 5.12).
- **Lens set**: which lenses are on for a repo, with per-PR overrides.
- **Steering rule**: a short, general instruction attached to one lens (or to **General**, which applies to all review lenses), usually drawn from one of your review comments (see 5.13).
- **Verify worktree**: a second, disposable Docent-owned worktree per PR where agents may build, test and write files (see 5.15).
- **Finding**: structured lens output pinned to `repo:file:line(s)` with severity, category, rationale, optional suggested replacement, and evidence. Private until you promote it to a comment.
- **Review state**: per-PR record of the last head SHA you reviewed, which files you marked viewed, and which comments/events you've seen.
- **Project**: a named set of bound repos, each at its own ref, explored together (see 5.9).

---

## 5. Features in detail

### 5.1 Accounts and connection

- **MVP**: when you connect a host in Settings → Accounts, Docent runs `gh auth token --hostname <host>` and holds the token in memory. Docent remembers each connected host's name and login (never the token) and, at startup, reads the token from `gh` again in the background. A host that fails to reconnect stays listed in Settings → Accounts with its error and a **Reconnect** button. If `gh` already reaches your SSO org, no OAuth app registration or org approval is needed. API calls are made directly from Rust.
- **M8, fallback**: a personal access token stored in the OS keychain, for machines without `gh`. Moved out of M1, since the first target has `gh`.
- **M8**: OAuth device flow for github.com users who don't have `gh`. Each GitHub Enterprise Server instance would need its own registered app, so GHES uses `gh`, a PAT, or a user-supplied client ID.
- Multiple accounts at once, e.g. work and personal.

### 5.2 Inbox (M1)

- Layout: mail split. Sections and watched repos in the drawer (see 5.4), the PR list in the middle, and a preview of the selected PR on the right: what changed since your last review, the AI summary, lens results, and trust. **Open review** or a double-click opens it ([UI mockups](spikes/ui-mockups.md)).
- Sections: **Review requested**, **Assigned**, **Authored by me**, **Mentioned**, **Pinned**, all limited to watched repos.
- Two quiet groups sit under Review requested: **Waiting on author** and **Could review**. Together with Review requested they make the **review lifecycle** below, and each PR is in exactly one of the three. The other sections are separate views and may repeat a PR.
- One quiet line for what's filtered out, e.g. "2 review requests in unwatched repos", with a **Watch** button.
- A watched repo with no local clone shows **Bind** and **Clone**.
- Per row: title, repo, author, CI status, review state, size and age.
- **M4**: an **"updated since your last review"** badge with counts (`+3 commits · 5 comments`). It needs the review state from 5.3, which arrives in M4.
- Filters and saved views. Keyboard navigation (j/k, enter).

#### Review lifecycle (M1)

GitHub's `review-requested:@me` drops a PR as soon as you submit any review, and only brings it back on an explicit **Re-request review**. Team requests vanish when a teammate reviews or auto-assignment picks someone else. So Docent doesn't trust the request alone. Every open PR in a watched repo that you didn't author goes in the first group that fits:

1. **Review requested**: you or one of your teams is currently requested, **or** you reviewed it and the **author has acted since**. A PR that's here because the author acted, not because of a request, shows an **"updated since your review"** marker (the M4 badge adds the counts).
2. **Waiting on author**: you reviewed it (approve, request changes, or comment) and the author hasn't acted since, **or** you moved it here by hand and the author hasn't acted since.
3. **Could review**: every other open, non-draft PR.

**The author has acted** means at least one of these happened after your latest review (or after you moved the PR by hand):

- the head commit is not the one your latest review was on (`commit_id` of the review), or new commits arrived after the manual move;
- the author commented or replied on the PR;
- for a manual move only: a review request to you or your team made after the move (GraphQL `ReviewRequestedEvent.createdAt`). For a submitted review this is already covered, since GitHub cleared the old request.

Other reviewers' comments and approvals don't count. They show as review state on the row.

**Moving a PR by hand.** Rows in Review requested and Could review have a **Waiting on author** button, for when the conversation happened offline or the author said "will do". Docent stores the time and head commit locally, per PR. It's never written to GitHub and doesn't sync between machines. A PR moved by hand shows **Back to inbox** in Waiting on author, which deletes the record. A PR waiting because of a real review has no such button: only the author acting brings it back.

Merged and closed PRs leave all three groups, since every search is `is:open`. Manual-move records for them can be pruned.

**Queries, per host** (all paginated as in #36):

- `is:pr is:open archived:false review-requested:@me` (already in place)
- `is:pr is:open archived:false reviewed-by:@me -author:@me`
- `is:pr is:open archived:false draft:false -author:@me repo:…` over the watched repos on that host, for Could review

For PRs in the second search, Docent fetches your latest review's commit and time, the head commit, and the author's latest comment time, to apply the rule above.

### 5.3 Change tracking

**In the MVP (M1–M4), refreshed when you open a PR or the inbox:**

- **"Reviewed" means** you submitted a review or clicked **Mark reviewed**. Opening a PR never counts.
- On first sight of a PR, `last_reviewed_sha` comes from the commit of your latest submitted review on GitHub, so existing PRs and other machines start correctly.
- **"Since last review" diff**: an interdiff (`git diff <lastReviewed>..<newHead>`). If history was rewritten, it falls back to `git range-diff`.
- New and unseen comments are highlighted. "Mark all seen" is available.

**In M5:**

- **Poller**: polls REST `/notifications` with conditional requests, obeying `X-Poll-Interval`, as the cheap change signal. GraphQL then runs only for PRs that changed. GraphQL has no ETag support, so it can't be the polling path. CI changes may not appear in notifications, so a slower GraphQL check covers your open PRs.
- **Tray**: closing the window leaves Docent in the tray, with a setting to quit instead.
- **OS notifications** with per-event toggles: new review requests, new commits on PRs you've reviewed, comments mentioning or replying to you, CI failed on your PR, your PR approved or changes requested. Clicking one opens the PR.
- Sync with GitHub's **"Viewed" file checkboxes** (GraphQL `markFileAsViewed`).

### 5.4 PR review view (M1–M4)

The app shell is an **activity bar** down the far left: Inbox, Review, Explore, Projects, Bookmarks, Steering suggestions, and Settings at the bottom. Each icon opens a **drawer** beside it with options for that area; clicking the active icon closes it. Bookmarks and Steering open over any page. Chosen in the [UI mockups spike](spikes/ui-mockups.md).

```
┌─┬ Drawer ───────────────┬ Diff ────────────────────────┬ Agent panel ───────────────┐
│▣│ Open reviews           │  gutter: ● findings          │ [Findings] [Explain] [Chat]│
│▣│ Files changed          │          💬 comments          │ Your review (pending)      │
│▣│  (viewed ✓, finding    │  click line number → comment │ ▾ Bugs · 2 rules   4  [on] │
│▣│   and comment counts)  │  hover line → ? at far right │ ▾ Style · 4 rules  1  [on] │
│⚙│                        │                              │ ▸ Secrets          off     │
└─┴────────────────────────┴──────────────────────────────┴────────────────────────────┘
 Top bar: Inbox › repo › PR title · CI · "3 new commits" · [Explore this branch] · [Submit review ▾]
 Bottom: Agent activity drawer · Verify
```

- Overview tab: description, checks, timeline, and an AI summary from the explainer lens.
- Diff modes: unified or split, full PR or since-last-review, ignore-whitespace. Moved code shows as moved (see 5.14).
- The viewer shows git's hunks exactly. CodeMirror's `makePresentable` pass can slide a hunk by a line or merge two close hunks (2 of 133 files in M0), so Docent wraps the changes it passes in, or upstreams an option to skip that pass, when the viewer lands in M1.
- Findings on deleted lines in unified view: CodeMirror can't mark individual deleted lines there, because it owns their DOM (M0). M3 decides between markers in split view only and a marker on the nearest new-side line.
- Show/hide findings by lens and severity.
- Your pending comments have their own gutter marker, and the file tree counts them beside agent findings.
- Findings appear only in the agent panel (plus gutter dots and tree counts), **grouped by lens**. Each group shows its steering count (`Style · 4 rules`), its finding count, and an on/off switch for this PR (a switched-off lens offers **Run anyway** and "apply to repo"). Groups start expanded and can be collapsed; a collapsed group shows how many findings it hides. Switching a lens off also hides its gutter dots.
- Your pending comments are listed above the lens groups as **Your review**. On an untrusted PR they stay visible, and the trust card takes the place of the lens groups.
- You can click and move the cursor anywhere freely. Agents are only triggered by the `?`.

### 5.5 Agents ("lenses") (M2–M3)

All lenses run through agent sessions with `cwd` set to the PR's worktree, so the agent can read anything in the repo using its own tools.

| Lens | Trigger | Input Docent provides | Output |
|---|---|---|---|
| **Explain** | The `?` on a line, plus a PR summary on open | PR title/body, the hunk, blame and log, the PR that introduced the code, Jira keys, related-change hints | Markdown: *what* it does, *why* it's there, **related changes** as clickable links, callers/impact |
| **Bugs** | Auto-run rules below | Full diff + PR context | JSON findings: correctness, edge cases, security, concurrency, error handling |
| **Style** | Auto-run rules | Diff + repo conventions (`CONTRIBUTING.md`, `AGENTS.md`, `CLAUDE.md`, `.editorconfig`) | JSON findings citing the in-repo example |
| **Modernize & lint** | Auto-run rules | Diff + detected language/framework versions + linter output, when available | JSON findings: newer idioms, deprecated APIs, linter violations, each with a suggested replacement |
| **Chat** | Manual | Current focus + prior panel content | Free-form follow-up in the same session |
| **Custom** | Auto-run rules or on demand (set per lens) | Diff + PR context, optional file globs | JSON findings (fixed Finding schema, own categories allowed) |

Each review lens also receives its steering rules and the General rules (see 5.13). Moved, unchanged lines are left out of lens prompts (see 5.14).

**Explain behaviour**: a `?` appears at the far right of the hovered line, and clicking it triggers Explain. Explain runs per **hunk or enclosing symbol**, and line-specific questions are answered from that cached result. One Explain runs at a time per PR, and clicking elsewhere cancels the older request.

**"Why" context**: agents have no forge access, so Docent looks up the PR behind each blamed commit (with its linked issues) and includes it in the prompt, cached.

**Jira context**: Docent extracts keys like `ABC-123` from commit messages, PR titles and branch names, shows them as links, and passes them to the agent, which looks them up through its Jira MCP server. If there's no key, or the profile has no Jira tools, Explain carries on without it.

**Auto-run rules**: lenses run when you open a PR at a new head SHA, if all of these hold:
- The repo, branch or PR is trusted.
- The PR is open and not a draft.
- It requests your review or you pinned it.
- It's within 1,500 changed lines and 50 files (configurable), not counting lockfiles and generated files.

Otherwise lenses are on demand. Each lens uses one session per PR on the whole diff.

**New commits**: after a push, lenses re-run on the interdiff only, and findings on unchanged code carry forward. Each finding has a fingerprint (lens, file, normalised code, category), so dismissals survive new commits.

**Related changes**: Docent pre-computes cheap structural hints (other hunks touching the same symbols via ripgrep and optional tree-sitter, co-changed files, test files). The agent confirms or expands them.

**Structured output**: each lens prompt ends with a required JSON block (schema in the lens frontmatter). Docent validates it, retries once with the validation error, and otherwise shows the raw markdown. Every agent-reported line is checked against the real file, and a finding that fails is marked **unanchored**.

**Caching and cost control**: results are cached in SQLite keyed by `(lens, lens_hash, profile, repo, head_sha, base_sha, scope)`, so editing a lens or switching agents invalidates old results. `lens_hash` includes the lens's steering rules, but a steering change never re-runs existing results: it applies at the next new head SHA or on-demand run, and the PR view shows "steering changed since this run · Re-run". There's a concurrency limit per profile, a cancel button (`session/cancel`), and a per-PR "run agents" toggle.

**Customizing lenses**: see 5.12.

**Usage display**: per-lens usage is shown where the agent reports it, through a per-agent adapter: ACP `usage_update` where the agent sends it (claude-agent-acp), and `_meta` plus the `_kiro.dev/metadata` notification for Kiro, which sends no `usage_update` ([M0.5](spikes/work-laptop-proof.md)).

### 5.6 Trust and agent safety (M2)

**Threat model**: PR content is attacker-controlled input. Code comments and `AGENTS.md` can carry prompt injection. Repo agent config (such as `.claude/settings.json` hooks) can run code when an agent starts. Linters such as `eslint` and `cargo clippy` execute repo code. The requirement: **opening an untrusted PR can't execute its code.**

**Two states:**

- **Untrusted** (default): code and diffs only. No agents, linters or repo config.
- **Trusted**: everything runs under the rules in this plan.

On top of trust, a repo can be marked **allow verification** (a separate checkbox in Trust settings), which lets agents run the PR's code (see 5.15). Fork PRs never get verification in v1.

**Granting trust**: clicking `?` in an untrusted repo opens a dialog with two choices, and Docent remembers the answer:

- **Trust repo and all future branches**: covers all branches and future commits. Later PRs and branches auto-run under the rules in 5.5.
- **Trust this branch only**: resets on each new push.

Fork PRs always need **Trust this PR**, even in a trusted repo, and that resets on each push. Trust is stored locally and can be revoked in Settings.

**What trust means**: ACP permission requests are optional for agents, and not advertising `fs/write_text_file` doesn't stop an agent's built-in tools. So read-only is **best-effort**, not guaranteed, and the Trust dialog says so plainly. Controls:

- **Kiro**: a restricted custom agent with a tool allowlist, always selected by name (`--agent`). M0.5 showed that Kiro applies no workspace trust under ACP: a repo's `.kiro/agents/kiro_default.json` replaces the default agent, and its hooks and MCP servers run at spawn ([write-up](spikes/work-laptop-proof.md)). So Docent never lets Kiro fall back to a default agent: a run where the named agent didn't load (`_kiro.dev/agent/not_found`) fails and is shown as an error. Whether a workspace agent can shadow Docent's named agent is still open (`OPEN-QUESTIONS.md`).
- **Claude Code**: `settingSources: []` (or `["user"]` in Inherit mode, never `"project"`), a restricted `tools` list, `strictMcpConfig`, `disableAllHooks`, and an explicit `session/set_mode`.
- **All agents**: forge tokens stripped from the environment at spawn; Docent denies any permission request outside the policy.
- **MCP tools**: denied unless on the profile's allowlist, including any new tools a server adds later. Lens sessions get Jira **read** tools only (get, search), so injected content can't write to Jira.
- A full OS sandbox around the agent process is decided at M8, before public release.
- **Content Security Policy**: the scaffold ships with the generator's `csp: null`. Forge content (PR bodies, comments) and agent output are hostile, so a real CSP lands with the first M1 slice that renders them as markdown, and they are never rendered as raw HTML.

**Permission modes** (per agent profile):

- **Docent-managed** (default): Docent turns its capability toggles into agent-specific config (a Kiro agent tool allowlist, Claude `allowedTools`) and answers ACP permission requests against them. Toggles: file writes (verify worktree only), shell commands (the repo's verify allowlist, see 5.15), web search, web fetch. Web search and fetch are **off** by default, because prompt injection plus web access is a way to send code out.
- **Inherit harness settings**: Docent uses your **user-level** agent settings, never the repo's, and only denies requests that fall outside the verify worktree. The UI warns that anything your harness auto-allows never reaches Docent, so Docent can't limit it.

ACP gives Docent no way to read an agent's settings; Inherit mode works by not overriding them.

**Linters**: Docent detects and runs linters on changed files, in trusted repos only, using the same per-repo command list as verification (5.15). Without verification allowed, linters never install dependencies. A linter that can't run shows "skipped: dependencies missing", and Modernize continues without it.

**Transparency and retention**: every transcript (prompts, tool calls, permission decisions) is viewable in an "Agent activity" drawer. Transcripts are kept for 30 days (configurable) or until their worktree is removed. Findings are kept. Settings has a **Clear all** button.

### 5.7 Worktree lifecycle (M1)

- Fetch PR heads, including `refs/pull/N/head` for forks, into `refs/docent/*` only. Old SHAs stay reachable for interdiffs after a force-push.
- Check out in detached HEAD, so your branches stay clean.
- Delete a worktree when its PR closes or merges, and enforce a disk cap with least-recently-used eviction.

### 5.8 Commenting and submitting reviews (M4)

- Click a line number or select lines → a new comment box at the top of the agent panel (markdown, preview, GitHub `suggestion` blocks). It has a **kind**: General or one of the review lenses. Comments you write yourself can become steering suggestions after you submit, filed under their kind (see 5.13).
- **Promote a finding** to a comment in one click. Docent pre-fills the text and you edit it before posting.
- Comments collect into a **pending review**, then go out together with **Comment / Approve / Request changes**.
- Reply to, resolve and unresolve existing threads. Single comments can also post immediately.
- Optional: "draft with agent". You write the gist and the explainer lens polishes it. The draft appears under your text to use or ignore, and never posts automatically.

### 5.9 Explore mode (M6, M6b)

**M6:**

- Pick any bound repo, then a branch, tag or commit. Docent creates or refreshes a worktree at that ref. From the rail, Explore starts on the default branch.
- **Explore this branch** on a PR opens Explore at the PR's head, with the files and lines the PR changed marked in the tree and the gutter, so you can read the change inside the rest of the code.
- Full file tree and read-only code viewer with the same `?` agent panel. Explain uses blame and log ("introduced in #1234 to fix …").
- Bugs, Style and Modernize run on demand for the current file or selection.
- If the selected branch has an open PR, Docent offers to jump into Review mode.
- **Findings**: save, or copy as markdown.
- **Self-review**: run all lenses on your own branch before opening a PR.
- **Trace lens**: pick a line and a direction, either "how do we get here" or "where does this data come from". It returns validated JSON steps, shown as a vertical path of code snippets. Each step links to its source, and any step whose location doesn't check out is flagged. LSP or tree-sitter accuracy can be added later.
- **Bookmarks**: a sidebar panel of saved locations, stored locally. Each stores repo, file, line and a snippet of surrounding code, opens on the default branch by re-locating the snippet, and is flagged with a link to the original commit if the snippet can't be found.

**M6b, Projects:**

- A Project is a named set of bound repos, each at its own ref. The main use case is a service and the infrastructure repo that deploys it.
- Docent puts the worktrees side by side in one parent folder and uses that as `cwd`, which should work with any agent.
- Every repo in a Project must be trusted before agents run.
- Trace works across repos. Links between repos are usually matches on names (image tags, routes, env vars), so those steps are marked **inferred**.
- Explore only for now, not PR review.

Built from the start so this doesn't need a rework: `AgentBackend` takes multiple roots, and every finding and trace step stores its repo.

### 5.10 Jira (after M6b)

- **Create issue** and **comment on ticket** from a finding, through the agent's Jira MCP server.
- You edit and approve the exact text in Docent first. Docent then sends it in a new single-purpose session with an empty working folder, no repo content, and only the matching write tool plus search for duplicate checks.
- Docent shows the resulting ticket key and link.
- Profiles without Jira tools don't show the option.

### 5.11 GitLab (0.2)

- Implement `Forge` for GitLab: MR inbox, MR diffs and versions, discussions, draft notes, approvals.
- Self-managed GitLab via base URL. Personal access token first, OAuth (PKCE) later.

### 5.12 Lens management (M3)

**Layers:**

- **Global library**, in Docent's config folder: the defaults plus any lenses you want everywhere.
- **Per-repo lens set**: picks which library lenses are on and can add **repo-only** lenses, also stored locally.
- **`.docent/lenses/`** in a trusted repo (default branch only, never a PR head): optional, for sharing with a team. Shown as "from repo" and switchable like any other.
- A **Project** uses the union of its repos' lens sets.
- A repo lens with the same `id` as a global one **replaces** it, and the agent panel marks it "repo version". Lenses with different ids both show, even if they look alike; switch one off per PR or per repo.

**Defaults:**

- Editing a default makes it your copy, and Docent stops updating it. Settings shows "a newer default is available" with a diff.
- Deleting a default **tombstones** it, so updates can't bring it back unannounced. **Restore defaults** undoes this.
- **Explain** and **Chat** can be edited but not deleted or disabled. The automatic **PR summary** has its own on/off switch.

**Enable and disable:**

- Toggles per repo, per PR (from the agent panel, with "apply to repo"), and a global **pause this lens everywhere**.
- A disabled lens starts no new runs. Its cached findings are hidden but kept, and come back when you turn it on.
- A disabled lens has a one-click **Run anyway** on a PR.

**Custom lenses** use the fixed Finding schema so the gutter, counts, fingerprints and promote-to-comment work. A lens defines name, prompt, trigger (auto or on demand), optional file globs, default severity, and may add its own `category` values. A free-form report lens is a possible later addition.

### 5.13 Steering (M4)

**Staging:**

- After you submit a review in a trusted repo, one short background session reads the **new review comments you wrote yourself**. Replies to existing threads and unedited promoted findings don't count; comments deleted before submitting never count.
- Only comments that generalize into a rule become candidates (most will be style). One-off comments produce nothing.
- Each candidate is filed under its comment's **kind** (see 5.8): a lens, or **General** (all review lenses), then compared with existing rules. A near-duplicate is offered as **merge with existing rule X**, with the merged wording.
- Candidates go to a **Steering suggestions** list (the Steering icon on the activity bar, with a badge), never a pop-up. Each stays until you **accept**, **edit**, **reassign**, choose **repo** or **global** scope, or **dismiss**. Nothing applies without your approval.

**Scope**: accepted rules are per repo by default. **Promote to global** moves a rule into the global library, applying it wherever that lens is on, including Explore and self-review.

**Visibility:**

- Lens groups in the agent panel show a rule count. Clicking it lists the rules to edit, reassign or delete.
- Repo Settings → **Lenses & steering** lists every lens with its rules, marked "from comment on #123", "global" or "repo", plus a rough token cost per lens.
- A finding that a rule triggered cites it ("per your rule: prefer `errors.Is`") through an optional `steering_rule_id` field.

**Cost**: rules are part of `lens_hash`, but changing them never re-runs existing results (see 5.5).

### 5.14 Moved code (M4)

- `docent-git` detects moved blocks itself (git-style block matching, as in `--color-moved`), **within and across files**, independent of the viewer.
- The diff shows a moved block as **moved from `file:line`** (click to jump) or **moved with changes**, with the small inner diff.
- Moved, unchanged lines are left out of lens prompts to save tokens.
- Moves that need no review stay out of the way: an exact move is **collapsed** behind its header by default, with an arrow to expand it. A move with changes starts expanded.
- The M0 spike confirmed CodeMirror renders all of this in split view, with both sides aligned. In unified view the viewer draws deleted lines itself, so a collapsed moved-away block is hidden by leaving it out of the original document Docent hands the viewer.

### 5.15 Verification (M4b)

**When**: on demand, through a **Verify** button on the PR or a "use verification" checkbox per lens run. A per-repo setting adds it to auto-runs. Verification is a capability any lens can use (Bugs especially), not a lens.

**Requirements**: the repo is trusted, **allow verification** is on, and the PR is not from a fork.

**Verify worktree**: a second Docent-owned worktree per PR, at the head SHA, reset with `git reset --hard && git clean -fdx` before each run. The review worktree stays clean, so line anchors and diffs stay valid. Tests the agent writes appear as findings with the test code attached, which you can promote to a comment.

**Commands:**

- Docent detects default commands per repo and you edit them in repo settings: for example `go build ./...`, `go test ./...`, `golangci-lint run` from `go.mod`; `cargo build`, `cargo test`, `cargo clippy` from `Cargo.toml`; scripts from `package.json`.
- The agent may run only commands that start with an allowed prefix, so `go test ./pkg/foo -run TestNewCase` works for a test it wrote. This list is the shell allowlist in Docent-managed mode, and the linter runner uses it too.
- Commands may pull dependencies (`go build`, `cargo build`) into your normal toolchain caches (`~/go/pkg/mod`, `~/.cargo`). Docent **never installs toolchains**: a missing tool shows "skipped: `go` not found".

**Limits**: 10 minutes per command, one verify run per repo at a time, cancellable. Forge tokens are still removed from the environment. Command output goes into the Agent activity drawer.

---

## 6. Data model (SQLite, initial)

```
accounts(id, forge, host, login, auth_source, keychain_ref NULL)   -- one login per host; never a token
repos(id, account_id, owner, name, default_branch, local_path NULL, watched BOOL, profile_id NULL)
trust(id, repo_id, scope, ref NULL, pr_id NULL, head_sha NULL, granted_at)   -- scope: repo | branch | pr
pull_requests(id, repo_id, number, title, author, state, is_draft, is_fork, head_sha, base_sha, updated_at, raw_json)
review_state(pr_id, last_reviewed_sha, last_seen_event_at, viewed_files_json)
waiting_on_author(pr_id, moved_at, head_sha)                  -- manual "Waiting on author" (5.2); local only
events(id, pr_id, kind, actor, created_at, seen BOOL)          -- commits, comments, reviews, CI
agent_profiles(id, name, command, args_json, env_json, mcp_allowlist_json,
               permission_mode, allow_writes BOOL, allow_shell BOOL, allow_web_search BOOL, allow_web_fetch BOOL)
                                                                -- permission_mode: docent | inherit
account_profiles(account_id, profile_id)
lenses(id, lens_key, source, repo_id NULL, path, tombstoned BOOL, edited_from_default_hash NULL)
                                                                -- source: default | global | repo_local | repo_committed
lens_settings(id, lens_key, scope, repo_id NULL, pr_id NULL, enabled BOOL)   -- scope: global | repo | pr
steering_rules(id, lens_key NULL, scope, repo_id NULL, body_md, source_comment_url NULL, created_at)
                                                                -- lens_key NULL = General; scope: global | repo
steering_candidates(id, repo_id, pr_id, source_comment_url, lens_key NULL, body_md, merge_with_rule_id NULL,
                    status, created_at)                         -- status: staged | accepted | dismissed
verify_settings(repo_id, allowed BOOL, auto_run BOOL)
verify_commands(id, repo_id, command_prefix, detected BOOL)
lens_runs(id, lens, lens_hash, profile_id, pr_id NULL, project_id NULL, repo_id, head_sha, base_sha, scope, status,
          started_at, finished_at, transcript_path, transcript_expires_at)
findings(id, lens_run_id, repo_id, fingerprint, file, line_start, line_end, anchored BOOL, severity, category,
         title, body_md, suggestion, steering_rule_id NULL, test_code NULL, dismissed BOOL, saved BOOL)
pending_comments(id, pr_id, file, line, side, body_md, from_finding_id NULL)
bookmarks(id, repo_id, file, line, snippet, origin_sha, label)
projects(id, name)
project_repos(project_id, repo_id, ref)
```

---

## 7. Milestones

Each milestone is a set of short-lived branches off `main`.

| # | Milestone | Outcome |
|---|---|---|
| **M0.5** | **Work-laptop proof** (gate) | A plain Tauri window, built from source on the work laptop: token from `gh`, one watched repo's PRs, worktree, plain diff, one Explain through `kiro-cli acp`. Also tests a malicious `.kiro/` folder, Jira tools under the restricted Kiro agent, MCP injection (for information only), and whether a restricted Kiro agent can run one allowlisted shell command. **Done: go** ([write-up](spikes/work-laptop-proof.md)). The restricted-agent questions move to M2. |
| **M0** | **Viewer spike** | Monaco vs CodeMirror 6 on macOS: gutter decorations, hover `?` at the far right, large-file performance, rendering moved blocks (within and across files), plus a one-off WebKitGTK load test. Pick one. **Done: CodeMirror 6** ([write-up](spikes/viewer-choice.md)). |
| **UI** | **UI mockups** | Throwaway mockups of the shell, inbox, PR view, Explore, Projects and Settings, to fix the layout before M1. **Done: activity bar, mail-split inbox, three-pane review** ([write-up](spikes/ui-mockups.md)). |
| **M1** | GitHub inbox + read-only PR view | Auth via `gh`, watched repos, Bind/Clone, inbox with review lifecycle, PR overview, diff viewer, worktree lifecycle, CSP. Tracked in [#10](https://github.com/thenorthnate/docent/issues/10) |
| **M2** | Explain + trust + agent settings | Trust model, profiles and detection, tool policy, Explain, PR summary, Chat, activity drawer |
| **M3** | Review lenses | Bugs, Style, Modernize & lint; findings in gutter and file tree; fingerprints; caching; lens files; lens management (global library, repo lens sets, custom lenses, tombstones, toggles, Run anyway) |
| **M4** | Commenting, reviews, since-last-review | Pending review, promote finding, submit, threads, review state, "updated since your last review" inbox badge, interdiff/range-diff, new-comment highlighting, moved-code detection, steering (staging, scope, UI, citations) |
| **M4b** | Verification + permission modes | Docent-managed and Inherit modes, Trust + allow verification, verify worktree, command detection and allowlist, limits |
| **M5** | Background tracking | Poller, tray, OS notifications, viewed-file sync |
| **M6** | Explore | Any-ref worktrees, file browser, on-demand lenses, saved findings, self-review, Trace, bookmarks |
| **M6b** | Projects | Multi-repo view and cross-repo Trace |
| **Jira** | Ticket writes | Create issue and comment via the agent's MCP server |
| **M8** | Release 0.1 | GitHub issues from findings, device-flow auth, PAT-in-keychain fallback, clearer `gh` failures (tell "not logged in" apart from other errors), login shells beyond bash and zsh (fish), custom app icon, `release-please`, full OS build matrix, paid signing (macOS notarized, Windows signed, Linux AppImage/deb/rpm), auto-update, OS sandbox decision, docs site, contributor guide |
| **0.2** | GitLab | Forge implementation |

**M0.5 gate:** M1 can't start until the Explain answer is useful on real work code and you decide it's worth continuing. If Kiro is blocked, swap the `AgentBackend` implementation or get the waiver first. If Explain isn't useful without ticket text, fix the Jira path before M1.

**MVP cut (personal use at work)**: M0.5–M4b. **First public release**: through M8.

**Early builds** are unsigned with the workaround documented. Tauri's updater has its own signing key, so auto-update doesn't need paid certificates.

---

## 8. Testing strategy

- **Rust**: unit tests per crate. `docent-git` tests run against temp repos built in-test. `docent-forge` uses recorded HTTP fixtures (`wiremock`) and a fake `Forge`.
- **Mock ACP agent**: a tiny test binary that speaks ACP and replays scripted responses (including permission requests and malformed JSON), so the lens pipeline is tested without real agents or cost.
- **Hostile-repo fixtures**: repos with malicious `.kiro/`, `.claude/`, `.mcp.json`, `.docent/lenses/` and linter configs, proving nothing runs before trust, and no build or test runs without **allow verification** or on a fork PR.
- **Moved-code fixtures**: `docent-git` tests for moves within a file, across files, and moved-with-changes.
- **Mock agent** also scripts out-of-allowlist shell requests and write requests outside the verify worktree, which must be denied.
- **Frontend**: Vitest + Vue Test Utils for components and stores. Playwright for a few smoke flows against the Vite dev build with mocked `invoke`, set up once M1 has a flow worth smoking (inbox → open PR → diff). Not set up yet.
- **CI**: GitHub Actions on macOS arm64 (`just check` and `tauri build`), plus a cheap Linux `cargo check`. The Linux job switches to the full `just check` once platform-specific Rust lands (`docent-git`, process spawning). The full OS matrix arrives at M8.

## 9. Open-source hygiene

- MIT license (already present). Add `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, issue/PR templates, `SECURITY.md` (token handling, trust model, agent limits).
- `AGENTS.md` / `CLAUDE.md` describing repo conventions.
- Conventional commits + `release-please` for changelogs and versioning. `release-please` is set up before the first release (M8); until then, squash-merge titles are written as conventional commits so the history is ready for it.
- No telemetry by default.

## 10. Risks and mitigations

| Risk | Mitigation |
|---|---|
| Kiro can't be run over ACP at work | M0.5 gate, `AgentBackend` interface, waiver request |
| Agents ignore the read-only policy | Trust gate, restricted agent configs, stripped tokens, plain wording in the Trust dialog, OS sandbox decision at M8 |
| Kiro loads hostile workspace config under ACP (**confirmed in M0.5**: repo agents, hooks, steering and `AGENTS.md` all take effect) | Never start an agent before trust; always name Docent's agent and fail if it doesn't load; open questions on v3 engine, `KIRO_HOME` and named-agent shadowing |
| Prompt injection writes to Jira | Read-only tool allowlist in lens sessions, separate approved session for writes |
| ACP agents differ in capabilities and output quality | Schema validation with retry, per-agent quirk flags in profiles, mock-agent tests |
| Agent cost and latency on big PRs | Auto-run limits, interdiff re-runs, cache, cancel |
| Viewer problems on Linux WebKitGTK surface late | One-off load test in M0 (passed under CI software rendering), full matrix at M8 |
| GitHub rate limits from polling | Conditional `/notifications` polling, GraphQL only on change, backoff |
| `/notifications` doesn't accept the `gh` token | Retired: it works under SSO, including 304s (M0.5). GraphQL poll stays the fallback |
| Force-pushes break "since last review" | `git range-diff` fallback; old SHAs kept in `refs/docent/*` |
| Verification runs hostile code from a PR | Separate opt-in on top of trust, never on fork PRs, allowlisted commands, verify worktree, stripped tokens, time limits, OS sandbox decision at M8 |
| Inherit mode lets the harness auto-allow things Docent can't see | Off by default; plain warning in the UI; user-level settings only, never the repo's |
| Steering rules pile up and cost tokens | Staged approval, merge suggestions, token cost per lens shown |
| Name overlaps with `TransluceAI/docent`, and npm/PyPI names are taken | Accepted; no package needs those names. Trademark search before M8 |
