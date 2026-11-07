# GAIT — Landscape Research

_Snapshot: November 2026. Captured from the planning conversation so later sessions start with the same context._

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

## Sources

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
