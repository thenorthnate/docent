# Docent — Open Questions and Decision Log

The design was stress-tested in November 2026. `PLAN.md` holds the result. This file records what's still unknown and how each earlier question was resolved.

## Still open

### To answer in M0.5 (on the work laptop)

These need a real test, not a decision.

1. Is `kiro-cli acp` allowed as a subprocess of another app, or is a waiver needed?
2. Does Explain give useful answers on real work code?
3. Does Kiro's workspace trust apply under ACP? Test with a malicious `.kiro/` folder. The docs don't say.
4. Can a restricted Kiro custom agent still reach the Jira MCP read tools?
5. Does a session-injected MCP server work under a restricted Kiro profile? For information only.
6. Does `gh auth token` work with SSO for every API call Docent needs, including REST `/notifications`?
7. Does Kiro report usage through `usage_update`, or only `_meta.kiro` metering?

### To test before relying on Claude Code

8. Does the combined lockdown work (`settingSources: []`, `tools`, `strictMcpConfig`, `disableAllHooks`)? Each option was found in source or docs, but the combination is untested, and dropping user settings may drop auth config.

### To decide later

9. **OS sandbox** around the agent process: decide at M8, before public release. Windows is the weak spot.
10. **Trademark search** on Docent: before M8.
11. **Parked ideas**, to revisit after 0.1:
    - E1: review checklist per PR
    - E2: risk heatmap in the file tree
    - E3: re-review on author reply
    - E5: Docent as an MCP server

## Resolved

### Original questions

| # | Question | Decision |
|---|---|---|
| Q1 | UI component library | shadcn-vue |
| Q2 | Diff/code viewer | Monaco vs CodeMirror 6, chosen by spike |
| Q3 | When review lenses run | Auto on opening a new head SHA, narrowed by the auto-run rules in `PLAN.md` 5.5 |
| Q4 | Agents running commands | Agents never run commands by policy; Docent runs linters in trusted repos only and never installs dependencies |
| Q5 | Findings private until promoted | Yes, always. No auto-post in v1 |
| Q6 | Explore findings | Save and copy in M6, Jira next, GitHub issues before public release. No fix branches |
| Q7 | Custom lenses | Editable defaults in your config folder; repo overrides for trusted repos, from the default branch only |
| Q8 | Agents to test | Kiro in M0.5; Claude Code at home. Assume Kiro over ACP is allowed, or a waiver is obtainable |
| Q9 | GitHub auth | Token from `gh`, PAT fallback, device flow at M8 |
| Q10 | Polling and notifications | Conditional REST `/notifications`, GraphQL on change, tray-resident. Added CI failed and PR approved/changes requested |
| Q11 | Packaging and signing | Unsigned early builds, paid signing at M8 |
| Q12 | Name and identifiers | Docent, `com.thenorthnate.docent` |
| E4 | Self-review | In M6 |
| E6 | Token/cost display | Where the agent reports it |

### Added during review

| Topic | Decision |
|---|---|
| First target | Work setup: Apple silicon Mac, GitHub Enterprise Cloud with SSO, Kiro, Jira |
| Early proof | M0.5 on the work laptop gates M1 |
| Agent coupling | `AgentBackend` interface, ACP as the only implementation for now |
| Threat model | PR content is hostile until trusted |
| Trust | Two states; `?` offers repo or branch trust; fork PRs per PR, reset on push; revocable |
| Read-only claim | Replaced with "best-effort read-only" |
| MCP tools | Denied unless allowlisted per profile; lens sessions get Jira read tools only |
| Jira | Keys from commits, titles and branches; lookups and writes through the agent's MCP server; writes in an approved, single-purpose session |
| "Reviewed" | Submitting a review or clicking Mark reviewed, never opening |
| MVP scope | Since-last-review moved into the MVP; poller and notifications stay in M5 |
| Repos | Watched repos only; Bind or full Clone; diffs always from local git |
| Worktrees | `refs/docent/*`, detached HEAD, removed on close, disk cap |
| Explain | Hover `?` only; per hunk or symbol; one at a time per PR |
| Finding continuity | Fingerprints, interdiff re-runs, wider cache key, line validation |
| Auto-run limits | Open, non-draft, review requested or pinned; 1,500 lines or 50 files |
| Why context | Docent looks up blamed PRs and puts them in the prompt; JSON output; no MCP server in the MVP |
| Trace | Agent-generated, validated steps, M6 |
| Bookmarks | Re-located by snippet, M6 |
| Projects | Multi-repo Explore in M6b; side-by-side worktrees; cross-repo steps marked inferred |
| Milestone order | M5, M6, M6b, Jira, M8, then GitLab in 0.2 |
| Early platforms | macOS arm64 plus a Linux `cargo check`; full matrix at M8 |
| Agent profiles | Per forge account with repo overrides; no global fallback |
| Transcripts | 30 days, configurable; findings kept; Clear all button |
