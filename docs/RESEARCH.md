# Docent — Landscape Research

_Snapshot: November 2026. Captured from the planning conversation so later sessions start with the same context. The project was called GAIT at the time._

## Question

Does a tool already exist that combines all of the following?

1. Cross-platform desktop app (macOS, Linux, Windows)
2. Native GitHub **and** GitLab connection: PR/MR inbox (assigned, review requested, etc.)
3. "Changed since I last reviewed" tracking (new commits, new comments) with notifications
4. Click any line in a diff → side panel explains *why* the line exists and what related changes go with it
5. Full-repository context from a **local clone**, not just the diff
6. Several default agents running automatically: explainer, bug/issue reviewer, style-vs-repo consistency, modernization/simplification (+ linting)
7. Write review comments in-app and post them back to GitHub/GitLab
8. "Explore mode": the same click-a-line experience on any branch (e.g. `main`) with no PR involved
9. **Agent-agnostic**: use whatever agent harness you already have (Kiro at work, Claude Code at home, Codex, Copilot…)

## Answer

No. Pieces exist; the combination does not.

| Feature | Closest existing coverage | Gap |
|---|---|---|
| Desktop app | GitHub Copilot app, JetBrains IDEs, GitKraken Desktop, Pull Panda | Copilot app is GitHub-only and Copilot-harness only; JetBrains is a full IDE; GitKraken/Pull Panda have little/no AI review |
| GitHub + GitLab | JetBrains, GitKraken | Neither has review agents |
| Inbox + change notifications | Copilot app, Graphite (web), GitKraken Launchpad | None tracks "since *you* last reviewed" precisely |
| Click line → explanation | Devin Review (web/CLI), Graphite chat (web) | Manual chat, web only, GitHub only |
| Local clone context | Copilot app (worktrees), JetBrains, Devin CLI | — |
| Specialized default agents | Qodo 2.0 (multi-agent), Greptile, CodeRabbit | Server-side PR bots, not an interactive reviewer UI |
| Post comments back | Copilot app, JetBrains, Graphite, Devin | — |
| Explore mode (no PR) | Generic IDE chat only | Nothing purpose-built |
| Bring-your-own agent | JetBrains (ACP), Zed (ACP), GitKraken Kepler (ACP) | None of these is a review tool |

## Agent interoperability (why ACP)

The **Agent Client Protocol (ACP)** is an open JSON-RPC-over-stdio protocol (originated by Zed) that lets a client app drive coding agents. Relevant support:

- **Kiro CLI**: `kiro-cli acp` (supports `session/new`, `session/load`, `session/prompt`, `session/cancel`, `session/set_mode`; MCP servers can be passed per session; custom `_kiro.dev/*` extensions).
- **Claude Code**: via the `claude-agent-acp` adapter (`@agentclientprotocol/claude-agent-acp`, formerly `@zed-industries/claude-code-acp`).
- **Codex**: via the `codex-acp` adapter.
- **GitHub Copilot CLI**: `copilot --acp` (v1.0+).
- **Gemini CLI**, Cursor CLI, Auggie, OpenCode and others also speak ACP.
- Rust SDK: crate `agent-client-protocol` (v5.x at time of writing) with stdio/process transports.

GitHub Copilot's own multi-agent support (Claude/Codex inside Copilot) is tied to a Copilot subscription and the Copilot harness; it does not accept Kiro.

## Fact check of the plan (November 2026)

From docs and source reading. No live ACP sessions were run. Kiro findings are medium confidence, because no public source was available.

### ACP and read-only enforcement

- The ACP spec says agents "MAY" request permission, so `session/request_permission` is optional. Not advertising `fs` or terminal capabilities only stops agents calling those client methods, not their built-in tools.
- **claude-agent-acp**: loads user, project and local settings by default. User allow rules resolve before the client is asked. SDK sessions show no trust dialog, so repo hooks in `.claude/settings.json` can run. There's no true read-only mode; `plan` still runs read-only shell commands. A client can pass `settingSources: []`, `tools`, `disallowedTools`, `strictMcpConfig` and `settings: {disableAllHooks: true}` through `_meta.claudeCode.options`, and must call `session/set_mode` to set the mode. The combination is untested.
- **kiro-cli acp**: workspace reads and read-only git/system commands are allowed silently, and other actions prompt. Configured policy outranks client responses. Workspace trust blocks repo config in interactive V3 sessions, but the docs don't say whether it applies under ACP. Read-only options are presets, the Plan agent, or a custom agent with a `tools` list. `KIRO_HOME` can isolate global config.
- **MCP**: stdio MCP is mandatory for agents, and clients may pass servers per session. claude-agent-acp honours this. Kiro V3 accepts `mcpServers`, but the docs don't explicitly confirm stdio.
- **Usage**: ACP `usage_update` reports context tokens and optional cumulative cost. claude-agent-acp sends it. Kiro documents `_meta.kiro` metering instead.

### Sandboxing

- macOS `sandbox-exec` is deprecated but still works, and is what Claude Code and Kiro rely on. Linux has bubblewrap and Landlock. Windows is the weak spot.
- `anthropics/sandbox-runtime` is a beta off-the-shelf wrapper, and it allows reads everywhere unless paths are denied.
- The built-in sandboxes in Claude Code and Kiro don't stop repo hooks, so the whole agent process would need wrapping.

### GitHub

- GraphQL is POST, and GitHub doesn't support conditional POST requests, so expect no 304s (inferred, not tested).
- REST `/notifications` supports `Last-Modified` and `X-Poll-Interval`, and a 304 with a correct `Authorization` header doesn't count against the primary rate limit. Notifications may not cover CI changes. The docs say this endpoint needs a classic PAT rather than GitHub App tokens, and don't mention OAuth tokens.
- Each GHES instance needs its own OAuth App. SAML SSO orgs need an active session when authorizing, and may need owner approval.
- `gh auth token --hostname` is viable, including refresh. It requires `gh`, and the token dies on `gh` logout.

### Linters on untrusted code

- `eslint`: executes repo code (config is imported), and plugins need `node_modules`.
- `cargo clippy`: executes `build.rs` and proc macros. `.cargo/config.toml` and `rust-toolchain.toml` add further risk.
- `ruff`: the safest; `--isolated` ignores config and it needs no dependencies.
- `golangci-lint`: mixed; a repo config may load a Go plugin. Use `--no-config`, and `GOTOOLCHAIN=local`.

### Naming

- Tauri warns about identifiers ending in `.app`.
- `gait` was taken on crates.io, npm and PyPI, and `Clyra-AI/gait` is an AI agent policy tool.
- `docent` is free on crates.io and Homebrew casks, taken on npm and PyPI. `TransluceAI/docent` is an AI tool with the same name. No trademark search has been done.

### Fact-check sources

- ACP — https://agentclientprotocol.com/protocol/tool-calls, https://agentclientprotocol.com/protocol/session-setup, https://agentclientprotocol.com/announcements/session-usage-stabilized
- claude-agent-acp — https://github.com/agentclientprotocol/claude-agent-acp/blob/main/src/acp-agent.ts
- Claude Code — https://code.claude.com/docs/en/agent-sdk/permissions, https://code.claude.com/docs/en/sandboxing
- Kiro — https://kiro.dev/docs/cli/v3/acp-migration/, https://kiro.dev/docs/permissions/, https://kiro.dev/docs/sandbox/
- sandbox-runtime — https://github.com/anthropics/sandbox-runtime
- GitHub — https://docs.github.com/en/rest/activity/notifications, https://docs.github.com/en/rest/using-the-rest-api/best-practices-for-using-the-rest-api, https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/differences-between-github-apps-and-oauth-apps, https://cli.github.com/manual/gh_auth_token
- Linters — https://eslint.org/docs/latest/use/configure/configuration-files, https://doc.rust-lang.org/cargo/reference/config.html, https://docs.astral.sh/ruff/configuration/, https://golangci-lint.run/docs/plugins/go-plugins/
- Tauri — https://github.com/tauri-apps/tauri/blob/dev/crates/tauri-cli/src/build.rs

## Landscape sources

- GitHub Copilot app docs — https://docs.github.com/en/copilot/concepts/copilot-surfaces/github-copilot-app
- VS Code agent harnesses — https://code.visualstudio.com/docs/agents/run/agent-harnesses
- Devin Review — https://docs.devin.ai/work-with-devin/devin-review
- Graphite PR page / chat — https://graphite.com/docs/pr-page-overview.md
- Qodo 2.0 — https://www.qodo.ai/blog/introducing-qodo-2-0/
- Greptile — https://greptile.com/docs/code-review/key-features
- CodeRabbit IDE/CLI — https://docs.coderabbit.ai/overview/ide-cli-review.md
- GitKraken Kepler agent integrations — https://support.gitkraken.com/kepler/agent-integrations
- JetBrains AI Assistant agents (ACP) — https://www.jetbrains.com/help/ai-assistant/2026.2/activate-agents.html
- Kiro CLI ACP — https://kiro.dev/docs/cli/acp/
- Rust ACP SDK — https://docs.rs/agent-client-protocol
- Pull Panda — https://github.com/artmann/pull-panda-desktop
