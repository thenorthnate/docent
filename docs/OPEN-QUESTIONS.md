# GAIT — Open Questions

Each item has my recommendation. Nothing here is built until you answer. Answers will be folded back into `PLAN.md` and the question marked resolved.

## Stack and UI

**Q1. UI component library?** ✅ Resolved: shadcn-vue
- **shadcn-vue (Recommended)**: Tailwind-native, components copied into the repo so they're fully editable, built on Reka UI (accessible primitives).
- PrimeVue: huge ready-made set (data tables, trees). Tailwind support via "unstyled" mode, but more opinionated.
- Nuxt UI: polished, but Nuxt-flavored. Works in plain Vue, with less community usage outside Nuxt.

**Q2. Diff/code viewer?** ✅ Resolved: prototype Monaco and CodeMirror 6 in M0, decide then
- **Monaco (Recommended)**: the VS Code editor. Best diff UX, gutter decorations, hover, many languages. Heavier (~several MB) and needs a check on Linux WebKitGTK.
- CodeMirror 6 + `@codemirror/merge`: lighter, very customizable, slightly more assembly required.
- `@git-diff-view/vue`: GitHub-style diff component. Quick to start, but less suited to Explore mode (full files).

## Agents

**Q3. When should review lenses (Bugs/Style/Modernize) run?** ✅ Resolved: auto-run when a PR is opened at a new head SHA, on-demand above a size threshold
- **Automatically when a PR's new head SHA is first opened in GAIT (Recommended)**, with a size threshold above which they become on-demand.
- Automatically on every new head SHA in the background, even before you open the PR (fastest, most cost).
- Only on demand (cheapest).

**Q4. Should agents be allowed to run commands (linters, type checkers, tests)?** ✅ Resolved: agents read-only; GAIT runs linters itself
- **GAIT detects and runs the repo's linters itself, then feeds the output to the Modernize lens; agents themselves stay read-only (Recommended).** Predictable and safe.
- Allow agents to run commands from an allowlist you edit per repo.
- Fully read-only: no linters at all.

**Q5. Should findings stay private until you promote them to a comment?**
- **Yes (Recommended)**: nothing reaches GitHub/GitLab without you clicking.
- Optional "auto-post findings above severity X" per repo.

**Q6. In Explore mode, what can you do with findings?**
- Private notes (stored locally)
- Copy as markdown
- Create a GitHub/GitLab issue pre-filled
- Hand off to an agent to make a fix branch (would break the "never writes code" non-goal; suggest later)

**Q7. Customizable lenses?**
- **Ship the 4 default lenses as editable markdown files, and allow per-repo overrides/additions in `.gait/lenses/` (Recommended).**
- Fixed built-in lenses only for v1.

**Q8. Which agents should we test against in M0?**
I assume **Kiro CLI** (work) and **Claude Code via `claude-agent-acp`** (home). Add Codex / Copilot / Gemini? Also: does your work allow running `kiro-cli acp` as a subprocess of another app (any policy concerns)?

## Forge and sync

**Q9. GitHub auth**
Device flow requires registering a **GAIT OAuth App** (free, under your GitHub account or a `gait` org). Fine to register it under your account for now? Do you use GitHub Enterprise Server or SSO-enforced orgs at work (affects token approval)?

**Q10. Polling interval and notifications**
Default 60s with ETags. OS notifications for: new review requests, new commits on PRs you've reviewed, new comments mentioning you or replying to you. Anything to add or remove?

## Project

**Q11. Packaging and signing**
macOS notarization needs an Apple Developer account ($99/yr); Windows signing needs a certificate (or Azure Trusted Signing). OK to ship unsigned early builds (documented workaround) and add signing at M8?

**Q12. Name and identifiers**
Bundle id `dev.gait.app`? Or something under your name (e.g. `com.thenorthnate.gait`)? Is there a domain?

## Extra ideas (opt-in)

- **E1. Review checklist per PR**: an auto-generated "things to verify" list (tests added? migrations? docs?) from the PR and the lens findings.
- **E2. Risk heatmap in file tree**: order files by risk (findings + churn + ownership) so you review the important ones first.
- **E3. Re-review on author reply**: when the author pushes a fix for a comment you left, auto-run the relevant lens on just that thread's lines and show "looks addressed / not addressed".
- **E4. Self-review mode**: run all lenses on your own branch before opening a PR.
- **E5. MCP server**: expose GAIT's context (PR, findings, review state) as an MCP server so your agent in other tools can use it.
- **E6. Token/cost display**: show per-lens usage where the agent reports it.
