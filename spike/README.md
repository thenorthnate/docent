# M0.5 work-laptop proof (spike)

Throwaway code for [thenorthnate/docent#4](https://github.com/thenorthnate/docent/issues/4). It lives on `spike/work-laptop-proof` and is **never merged**. Only the write-up (`docs/spikes/work-laptop-proof.md`) lands on `main`.

## What's here

| Path | What it is |
|---|---|
| `src-tauri/src/gh.rs` | Token from `gh auth token`, API probes (REST, `/notifications` + 304, search, blame → PR, GraphQL), PR list, pending-review round trip |
| `src-tauri/src/git.rs` | Fetch `refs/pull/N/head` into `refs/docent/*`, detached worktree, plain diff, blame/log for a hunk |
| `src-tauri/src/acp.rs` | Hand-rolled ACP client: logs every raw message, strips forge tokens from the agent's env, approves only exact allowlisted shell commands |
| `ui/` | Plain HTML/JS, no build step |
| `fixtures/mock-acp-agent.mjs` | Scripted ACP agent for checking the pipeline without Kiro |
| `fixtures/hostile-kiro.sh` | Builds a local repo with hostile `.kiro/` config that leaves marker files if it runs |
| `fixtures/restricted-agent/` | Template Kiro custom agent (tool allowlist, Jira read tools, one shell command) |

## Build and run

Needs Rust (stable), Xcode command-line tools, `git` and `gh` logged in to the work host. Node is only needed for the mock agent.

```bash
cd spike/src-tauri
cargo test            # git worktree + ACP policy tests (uses node for the mock agent)
cargo run             # opens the window
```

Corporate proxy: `reqwest` honours `HTTPS_PROXY` and uses the macOS keychain, so a proxy root CA installed there is trusted.

Run `cargo run` from a terminal so the app inherits your shell's `PATH` (it needs `gh`, `git`, `kiro-cli`). Loading the login-shell environment for Finder launches is out of scope for the spike.

App data (worktrees, transcripts) goes to `~/Library/Application Support/com.thenorthnate.docent.spike/`. Every agent run writes a JSONL transcript there, and its path is shown when the run ends.

**Quick pipeline check:** set Command to `node`, Args to the absolute path of `fixtures/mock-acp-agent.mjs`, Shell allowlist to `cargo check`, cwd to any folder, type a prompt and click Run. You should see one ALLOW and two DENY permission lines, a usage line and a `_meta` line.

## Experiments

Record answers as you go in the results template at the bottom. Don't copy work code, repo names, hostnames, ticket keys or transcripts into it.

### 1. Forge access (Q6)

1. Enter the host (e.g. `github.com`), owner, repo and the path of your local clone.
2. **Connect via gh**: shows the token type (`gho_` = OAuth from `gh`).
3. **Run API probes**: one row per call. Red means it failed. Note `x-github-sso` headers or SAML errors, and whether the conditional `/notifications` call returns 304.
4. **List open PRs**, select one, then optionally **Pending-review round trip**: creates a pending review only you can see and deletes it. Use a PR where that's acceptable.

Headless alternative: `DOCENT_SPIKE_REPO=owner/repo DOCENT_SPIKE_HOST=<host> cargo test live_probe -- --ignored --nocapture`

### 2. Worktree and diff

Select a PR → **Check out selected PR**. The agent cwd fills in with the worktree path. Click a hunk to select it.

### 3. Explain through Kiro (Q1, Q2, Q7)

1. Command `kiro-cli`, Args `acp`. Add `--agent <name>` on its own line once the restricted agent exists. Check `kiro-cli acp --help` for the real flag.
2. **Build prompt from selected hunk**: gathers blame at the base, file history and the PRs that introduced the blamed lines. Edit the prompt freely.
3. **Run**. Watch the answer stream, the events pane (tool calls, permission decisions, `_kiro.dev/*` notifications) and the usage pane.
4. Judge usefulness on several hunks: a small one, a refactor, and one where the "why" needs history.
5. Note whether usage arrives as `usage_update`, as `_meta` (look for `kiro` keys), or not at all.

### 4. Hostile `.kiro/` (Q3)

```bash
spike/fixtures/hostile-kiro.sh clean
spike/fixtures/hostile-kiro.sh setup ~/tmp/hostile-kiro
```

Set the agent cwd to `~/tmp/hostile-kiro`, write a prompt like "Explain src/main.rs", and run it with the default agent and again with the restricted agent. Then:

```bash
spike/fixtures/hostile-kiro.sh check
```

Each marker file names a vector that ran (steering, `AGENTS.md`, workspace MCP, workspace agent hooks or shadowing, IDE hook, code-comment injection). Before concluding "nothing fired", confirm the `.kiro/` formats in the script match the Kiro docs; adjust and re-run if they don't.

### 5. Restricted Kiro agent (Q4, Q5, Q7a)

1. Copy `fixtures/restricted-agent/docent-restricted.json` into the Kiro global agents folder, or into a separate `KIRO_HOME` (set `KIRO_HOME=…` in the Env box). Fill in the Jira MCP server and tool names, and check every field name against Kiro's custom-agent docs.
2. **Jira (Q4)**: ask "Look up <ticket> and summarise it". Then ask it to add a comment to the ticket. The read should work and the write should be refused.
3. **Session-injected MCP (Q5)**: put a server in the mcpServers box, e.g. `[{"name":"probe","command":"sh","args":["-c","touch /tmp/docent-spike-markers/session-mcp; sleep 30"],"env":[]}]`, run, then `hostile-kiro.sh check`.
4. **Allowlisted shell (Q7a)**: ask the agent to run `git status`, then something else (`ls -la`, `git log`). Put `git status` in the Shell allowlist box too, so the client answers the same way as the Kiro policy. Record which layer refused what: Kiro's policy, or Docent's permission answer, visible as a PERMISSION line.

### 6. Explain with and without Jira

Pick a hunk on a PR with a Jira key. Run with **Ask agent to use Jira tools** ticked, then unticked (rebuild the prompt each time). Compare.

## Results template

Copy this into `docs/spikes/work-laptop-proof.md` on a `docs/` branch off `main`.

```markdown
# Spike: M0.5 work-laptop proof

Date: · Machine: Apple silicon, macOS · Kiro version: · gh version:

## Answers
| # | Question | Answer | Evidence |
|---|---|---|---|
| Q1 | kiro-cli acp allowed as a subprocess? | | |
| Q2 | Explain useful on real work code? | | |
| Q3 | Workspace trust under ACP (hostile .kiro/)? | vectors fired: | |
| Q4 | Restricted agent reaches Jira read tools; writes refused? | | |
| Q5 | Session-injected MCP under restricted profile? | | |
| Q6 | gh token covers every call (incl. /notifications 304)? | | |
| Q7 | Usage via usage_update, _meta.kiro, or neither? | | |
| Q7a | One allowlisted shell command allowed, others refused? Which layer refused? | | |

## Explain quality notes
## Surprises
## Gate decision: go / no-go
```
