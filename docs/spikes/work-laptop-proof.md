# M0.5 spike: work-laptop proof

_November 2026. Answers Q1, Q2, Q3, Q6 and part of Q7 in [`OPEN-QUESTIONS.md`](../OPEN-QUESTIONS.md), and leaves Q4, Q5 and Q7a open for M2. Tracking issue: [thenorthnate/docent#4](https://github.com/thenorthnate/docent/issues/4). The code, fixtures and the round-2 experiment list are on the `spike/work-laptop-proof` branch, which is never merged; paths below under `spike/` refer to that branch._

## Decision

**Go.** The three gate conditions in [`PLAN.md` §7](../PLAN.md#7-milestones) hold: Kiro may run over ACP as a subprocess, Explain was useful on real work code, and the token from `gh` covers every API call Docent needs. M1 can start.

The hostile-repo test changes the M2 design. Kiro applied **none** of its workspace trust under ACP: a repo's `AGENTS.md` and steering file steered the agent into running commands, and a repo-supplied `kiro_default` agent replaced the real default and ran its hooks and MCP server. Trust gating before an agent starts is the only protection Docent can rely on, and Docent must never let Kiro fall back to a default agent.

## What was built

A throwaway Tauri app with a plain HTML UI, built from source on the work laptop (`spike/README.md`):

- token from `gh auth token --hostname <host>`, held in memory, and a table of REST and GraphQL probes
- PR list, PR checkout into `refs/docent/pr/N/*` with a detached worktree, plain diff, and blame/log context for a selected hunk
- a hand-rolled ACP client that logs every raw message to a JSONL transcript, strips forge tokens from the agent's environment, and approves only shell commands that exactly match an allowlist
- a mock ACP agent, a hostile `.kiro/` fixture that leaves a marker file for each vector that runs, and a restricted-agent template

## Findings

### Q1: Kiro over ACP is allowed

`kiro-cli acp` ran as a subprocess of the spike with no policy block. No waiver was needed.

`kiro-cli acp --help` shows the flags Docent will need: `--agent <name>` (agent for the first session), `--trust-tools` / `--trust-all-tools`, `--agent-engine` (`v2` is the default; `v3` is selectable), `--auth-method cli|client` (who answers access-token requests on the v3 engine) and `--login` (interactive login for ACP clients).

### Q2: Explain is useful

The maintainer judged Explain useful on real work code, with the prompt the spike builds: PR title and body, the hunk, blame of the old lines at the base, recent file history, and the PRs that introduced the blamed lines. It was useful without Jira ticket text, so the Jira path doesn't block M1.

### Q3: workspace trust does not apply under ACP

The fixture (`spike/fixtures/hostile-kiro.sh`) plants one marker-writing payload per vector. On the default engine (v2), with the default agent:

| Vector | Fired |
|---|---|
| `AGENTS.md` instruction to run a command | **Yes** |
| `.kiro/steering/*.md` instruction to run a command | **Yes** |
| Workspace `.kiro/agents/kiro_default.json`: its prompt | **Yes** |
| … its `agentSpawn` hook | **Yes** |
| … its `userPromptSubmit` hook | **Yes** |
| … its MCP server | **Yes** |
| Workspace `.kiro/settings/mcp.json` server | No |
| IDE-style `.kiro/hooks/*.kiro.hook` | No |
| Instruction in a code comment | No |

What this means:

- **A repo can run code as soon as Kiro starts in it.** Defining `.kiro/agents/kiro_default.json` is enough: its hooks run at spawn, before any prompt.
- **Prompt injection through repo instructions works.** The agent followed `AGENTS.md` and steering text and ran shell commands.
- **The agent Docent asks for may silently not be the one that runs.** The spike passed a named agent, Kiro reported `_kiro.dev/agent/not_found`, and the session ran as `kiro_default`, which in a hostile repo is the attacker's.

Still unconfirmed, and listed as round-2 experiments on the spike branch:

- whether Kiro asked the client before running those commands, or ran them under its own policy (the answer decides whether ACP permission answers are a control at all for Kiro)
- whether the v3 engine applies workspace trust
- whether a workspace agent can shadow a **named** global agent, not just the default
- whether a separate `KIRO_HOME` stops Kiro reading workspace `.kiro/`

### Q6: the `gh` token covers every call

Every probe passed under SSO on the work host:

- REST: `/user`, the repo, open PRs, inbox search (`review-requested:@me`), commits and commit → PR lookup (the "why" context)
- REST `/notifications`, and a conditional repeat with `If-Modified-Since` that returned **304**
- GraphQL: viewer, rate limit, PRs with review decision, last review commit and check rollup

The GraphQL fallback for notifications isn't needed. The pending-review round trip (create, then delete) is in the spike for M4 but its result wasn't recorded.

### Q7: Kiro reports usage through `_meta`, not `usage_update`

One Explain produced **no** `usage_update` notifications, 37 `_meta` blocks, and these Kiro extension notifications: `_kiro.dev/metadata`, `_kiro.dev/session/update`, `_kiro.dev/commands/available`, `_kiro.dev/subagent/list_update`, `_kiro.dev/agent/not_found`, `_kiro.dev/mcp/server_init_failure` and `_kiro.dev/mcp/oauth_request`. The prompt response was `{"stopReason": "end_turn"}` with no usage in it.

Usage display for Kiro therefore needs a per-agent adapter. The exact payload shape isn't recorded yet.

### Q4, Q5, Q7a: not answered

All three need the restricted custom agent, and it never loaded: every run used `kiro_default` (see Q3). Likely causes, in order:

1. The agent file wasn't where Kiro looks, or a field was wrong. The next attempt puts it in `~/.kiro/agents/` without `KIRO_HOME` and checks Kiro lists it.
2. For Jira (Q4): the `_kiro.dev/mcp/oauth_request` notification suggests the Jira MCP server needs an OAuth login that Kiro hands to the ACP client, which the spike ignored.

None of these block M1. They are needed for M2's tool policy and for M4b's verification.

## Changes to the plan

Made in this PR:

- **`PLAN.md` §5.6, Kiro controls:** Docent always names its own agent, treats `_kiro.dev/agent/not_found` (or any sign the named agent didn't load) as a failed run, and never lets Kiro fall back. Workspace `.kiro/` config is treated as hostile input whose hooks run at spawn.
- **`PLAN.md` §5.5, usage display:** per-agent adapters; Kiro's comes from `_meta` and `_kiro.dev/metadata`.
- **`PLAN.md` §10:** the hostile-config risk is confirmed, and the `/notifications` risk is retired.
- **`OPEN-QUESTIONS.md`:** Q1, Q2, Q3, Q6 and Q7 resolved; Q4, Q5 and Q7a carried into M2 with new questions on permission bypass, the v3 engine, named-agent shadowing and MCP OAuth.

## Not recorded

To keep this repo free of work material, this write-up has no work code, repo names, hostnames, ticket keys, prompts or transcripts. The Kiro and `gh` versions weren't noted.
