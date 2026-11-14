# Docent — Open Questions and Decision Log

The design was stress-tested in November 2026. `PLAN.md` holds the result. This file records what's still unknown and how each earlier question was resolved.

## Still open

### Carried from M0.5 into M2 (on the work laptop)

M0.5 answered 1, 2, 3, 6 and 7 (see Resolved). These still need a real test. The steps are in the round-2 list in `spike/README.md` on the `spike/work-laptop-proof` branch.

3b. When a repo's instructions steered Kiro into running `touch`, did Kiro send a permission request to the client first, or run it under its own policy? If it didn't ask, ACP permission answers are no control for Kiro.
3c. Does the v3 engine (`--agent-engine v3`) apply workspace trust under ACP? v2 is the default and applied none.
3d. Can a workspace `.kiro/agents/<name>.json` shadow Docent's **named** global agent, as `kiro_default` was shadowed? Does a separate `KIRO_HOME` stop Kiro reading workspace `.kiro/`?
4. Can a restricted Kiro custom agent still reach the Jira MCP read tools, with writes refused? Blocked in M0.5 because the named agent never loaded.
4a. How does a Jira MCP server's OAuth login work under ACP? Kiro sent `_kiro.dev/mcp/oauth_request` to the client. Does Docent need to handle it (and `--auth-method client`), or is a login made once in `kiro-cli chat` reused?
5. Does a session-injected MCP server work under a restricted Kiro profile? For information only.
7a. Can a restricted Kiro custom agent run one allowlisted shell command (for verification), while other commands are refused? Which layer refuses: Kiro's policy or Docent's permission answer?
7b. What is the shape of Kiro's usage in `_meta` and `_kiro.dev/metadata`?

### To test before relying on Claude Code

8. Does the combined lockdown work (`settingSources: []`, `tools`, `strictMcpConfig`, `disableAllHooks`)? Each option was found in source or docs, but the combination is untested, and dropping user settings may drop auth config. Also test `settingSources: ["user"]` (Inherit mode), which keeps user settings and may fix the auth question.

### To decide later

9. **OS sandbox** around the agent process: decide at M8, before public release. Windows is the weak spot.
10. **Trademark search** on Docent: before M8.
12. **Findings on deleted lines in unified view**: CodeMirror can't mark individual deleted lines there (M0). Decide in M3: markers in split view only, or a marker on the nearest new-side line.
11. **Parked ideas**, to revisit after 0.1:
    - E1: review checklist per PR
    - E2: risk heatmap in the file tree
    - E3: re-review on author reply
    - E5: Docent as an MCP server
    - E7: pre-submit agent check of manual comments (one run before sending, suggesting removals or rewrites). Parked: 5.8's "draft with agent" covers it for now
    - E8: free-form "report" lenses with no line anchors

## Resolved

### Original questions

| # | Question | Decision |
|---|---|---|
| Q1 | UI component library | shadcn-vue |
| Q2 | Diff/code viewer | CodeMirror 6, chosen by the M0 spike (see below) |
| Q3 | When review lenses run | Auto on opening a new head SHA, narrowed by the auto-run rules in `PLAN.md` 5.5 |
| Q4 | Agents running commands | ~~Agents never run commands by policy~~ **Superseded** by verification (see below): allowlisted commands in a verify worktree, in repos with Trust + allow verification. Docent never installs toolchains |
| Q5 | Findings private until promoted | Yes, always. No auto-post in v1 |
| Q6 | Explore findings | Save and copy in M6, Jira next, GitHub issues before public release. No fix branches |
| Q7 | Custom lenses | Editable defaults in your config folder; repo overrides for trusted repos, from the default branch only. Extended by lens management (see below) |
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

### Added in the second review (lenses, steering, verification)

| Topic | Decision |
|---|---|
| Lens scope | Global library plus per-repo lens sets with repo-only lenses; a Project uses the union of its repos' sets |
| Lens storage | Local in Docent's config folder; `.docent/lenses/` stays optional for team sharing |
| Same-id lenses | A repo lens replaces the global one with the same `id`; different ids both show |
| Deleting defaults | Tombstoned, with Restore defaults; an edited default is your copy and stops updating |
| Explain and Chat | Editable, not deletable or disableable; PR summary has its own switch |
| Lens toggles | Per repo, per PR (with "apply to repo"), and global pause; hidden findings kept; one-click Run anyway |
| Custom lens schema | Fixed Finding schema, own categories allowed |
| Manual comments | Covered by 5.8; own gutter marker and file-tree counts |
| Pre-submit check | Dropped for now (parked as E7) |
| Steering source | Extracted after you submit, from new comments you wrote yourself; only rules that generalize |
| Steering approval | Staged as suggestions; nothing applies without your approval |
| Steering scope | Per repo by default; promote to global |
| Steering visibility | Rule counts on the agent panel's lens groups, Repo Settings → Lenses & steering, rule citations on findings |
| Steering and cache | Part of `lens_hash`; never re-runs existing results, shows "Re-run" |
| Duplicate steering | Merge with existing rule offered; token cost per lens shown |
| Moved code | Detected in `docent-git` within and across files; left out of lens prompts; M4 |
| Verification | Separate verify worktree, reset each run; agent-written tests become findings |
| Verification trust | Trust + per-repo allow verification; never on fork PRs |
| Verification commands | Detected per repo, editable, prefix allowlist shared with linters; dependencies may be pulled, toolchains never installed |
| Verification runs | On demand by default, optional auto-run per repo; 10 min per command, one run per repo |
| Permission modes | Per profile: Docent-managed (default; web search and fetch off) or Inherit harness settings (user level only) |
| Milestones | Lens management in M3; moved code and steering in M4; verification and permission modes in new M4b; MVP is M0.5–M4b |

### Added by the M0 viewer spike

See [`spikes/viewer-choice.md`](spikes/viewer-choice.md).

| Topic | Decision |
|---|---|
| Viewer | CodeMirror 6 (`@codemirror/merge`). Monaco can't take an external diff, and disagreed with git's hunks in 42 of 133 real files |
| Diff source in the viewer | Always git's hunks, passed through `diffConfig.override` |
| Moves across files (was 8a) | Yes: headers rendered as block widgets, with a click to jump to the other end |
| Collapsing moves | Exact moves collapsed by default with an expand arrow; moves with changes start expanded |

### Added by the M0.5 work-laptop proof

See [`spikes/work-laptop-proof.md`](spikes/work-laptop-proof.md).

| # | Question | Answer |
|---|---|---|
| 1 | `kiro-cli acp` as a subprocess | Allowed; no waiver needed |
| 2 | Explain useful on work code | Yes, even without Jira ticket text. Gate passed: **go** for M1 |
| 3 | Kiro workspace trust under ACP | **Not applied** on the default (v2) engine. `AGENTS.md` and steering injections ran commands; a workspace `kiro_default` agent replaced the default and its prompt, hooks and MCP server ran. Docent always names its agent and fails if it doesn't load (`PLAN.md` 5.6). Follow-ups 3b–3d |
| 6 | `gh` token under SSO | Every call Docent needs works, including `/notifications` with a 304 on a conditional repeat |
| 7 | Kiro usage reporting | No `usage_update`; usage comes in `_meta` and `_kiro.dev/metadata`. Per-agent usage adapter (`PLAN.md` 5.5). Shape is 7b |

### Added by the UI mockups spike

See [`spikes/ui-mockups.md`](spikes/ui-mockups.md).

| Topic | Decision |
|---|---|
| App shell | Activity bar with drawers (VS Code style), over browser-style tabs. Steering and Bookmarks on the rail |
| Inbox layout | Mail split: sections in the drawer, PR list, preview pane |
| PR view layout | Three panes: file tree in the drawer, diff, agent panel. Findings only in the agent panel |
| Findings in the agent panel | Grouped by lens, each with an on/off switch; expanded by default, collapsible |
| Comment kind | New: you pick General or a lens for each comment; steering files the candidate under it instead of guessing |
| Explore from a PR | Opens the PR's head branch with changed files and lines marked, not the default branch |
| Untrusted PR panel | Your own comments stay visible; the trust card replaces the lens groups |

### Added while planning M1

From the M1 breakdown ([#10](https://github.com/thenorthnate/docent/issues/10)) and the follow-ups of earlier PRs.

| Topic | Decision |
|---|---|
| PAT fallback | Moved from M1 to M8, alongside device flow. The first target has `gh` |
| "Updated since your last review" badge | Moved from M1 to M4, where review state arrives |
| Clone credentials | Clone uses your existing git credentials; Docent never hands a forge token to git |
| Hunk fidelity | The M1 viewer stops CodeMirror's `makePresentable` from sliding or merging git's hunks |
| Content Security Policy | A real CSP replaces `csp: null` with the first M1 slice that renders forge or agent content |
| Linux CI | Switches from `cargo check` to the full `just check` once platform-specific Rust lands |
| Playwright smoke flows | Set up once M1 has an inbox → PR → diff flow |
| `release-please` and the app icon | At M8, before the first release |

### Added while connecting GitHub hosts (#12)

| Topic | Decision |
|---|---|
| When the `gh` token is read | When you connect a host, and again at startup for each remembered host (#13). Only the host name and login are stored, never the token. The startup reconnect runs in the background; commands that need accounts wait for it. It only reads the login shell's `PATH` when there is a host to reconnect. A host that fails stays listed with its error rather than being forgotten |
| Login shell environment | Only `PATH` is read from the login shell, and only its absolute entries. It is passed to child processes explicitly; Docent's own environment is not changed |

### Added while designing the review lifecycle

GitHub's `review-requested:@me` drops a PR once you review it, and team requests vanish when a teammate picks them up, so PRs disappeared from the inbox while still needing you. See the review lifecycle in `PLAN.md` 5.2.

| Topic | Decision |
|---|---|
| Reviewed PRs | Tracked with a second search, `reviewed-by:@me -author:@me`. They wait in **Waiting on author** and come back to Review requested when the author acts, re-request or not |
| "Author acted" | New commits since your latest review's commit, or an author comment or reply after it. Other reviewers' activity doesn't count |
| Your approval | Counts as a review: the PR waits, and comes back if the author acts afterward |
| Other people's approvals | Don't move a PR. They show as review state on the row |
| Team PRs nobody asked you for | A quiet **Could review** group: every open, non-draft PR in a watched repo you didn't author and haven't reviewed. Reviewing one moves it to Waiting on author |
| Precedence | Review requested, then Waiting on author, then Could review. Each PR in exactly one; other sections may repeat it |
| Merged or closed | Leaves every group |
| Manual "Waiting on author" | Local only, never written to GitHub. Comes back on author commits, author comments, or a fresh review request after the move. **Back to inbox** undoes it |
| "Updated since your review" marker | In M1, without counts. The counted badge stays in M4 |
| Parked idea E3 (re-review on author reply) | Partly taken up: a reply brings the PR back to the inbox. Re-running lenses on a reply stays parked |
