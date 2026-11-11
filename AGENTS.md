# AGENTS.md

Docent is a desktop app for AI-guided code review and exploration: Tauri 2, a Rust core split into `docent-*` crates, and a Vue 3 + TypeScript UI. It drives the user's own coding agent over ACP.

- **Design:** [`docs/PLAN.md`](docs/PLAN.md). Read the sections a slice touches before planning it.
- **Decisions and unknowns:** [`docs/OPEN-QUESTIONS.md`](docs/OPEN-QUESTIONS.md).
- **Process rules** (slices, PR size, branches, commits, review, dependencies): [`CONTRIBUTING.md`](CONTRIBUTING.md). This file covers how an agent works within them.

Status: scaffold only. `just setup` installs dependencies, `just check` runs every gate CI runs, and `just dev` opens the app. The app shell is in `src-tauri/` and the Vue app in `ui/` (the pnpm package `docent-ui`). The `docent-*` crates are added under `crates/` by the first slice that needs each one.

## Working a slice

A **slice** is one GitHub issue, delivered as one draft PR within the [size budget](CONTRIBUTING.md#pr-size). Work it in this order:

1. **Read** the slice issue, its milestone tracking issue, and the `PLAN.md` sections they cite.
2. **Plan.** Post the plan as a comment on the slice issue: files to touch, tests to write, open questions. If the plan departs from the issue, raises open questions, or will not fit the budget, stop and ask the maintainer. Propose a split for anything over budget. Otherwise continue.
3. **Build test-first** on a `<type>/<short-desc>` branch. Write the test, watch it go **red**, make it green, then refactor. Commit at each logical step (`test: …`, `feat: …`, `refactor: …`) so the PR reads commit by commit. Open the draft PR, using the [PR template](.github/pull_request_template.md), once the first commit is pushed.
4. **Check.** Run `just check` until it passes. Paste its final summary lines into the PR's "How it was tested" section.
5. **Review.** Run `/code-review` (or your harness's equivalent) on the branch. Fix each finding, or answer it under "Review notes" in the PR.
6. **Hand over.** Complete every template section and leave the PR as a draft. Marking it ready and merging are the maintainer's calls.

Stay inside the slice. Record anything else you notice under "Follow-ups" in the PR instead of fixing it. Push only to the slice branch. Once the maintainer has started reviewing, add new commits on top so they can see what changed since their review.

## Tests

- Every behaviour change ships with tests in the same PR.
- A bug fix starts with a regression test that goes **red** on the bug.
- Test-first is required in the pure-logic code: `docent-git`, `docent-store`, agent output parsing, and trust and permission policy.
- Every **security-sensitive path** needs a test that proves the *denial*, alongside the allowed case.
- Prefer the test doubles in [`PLAN.md` §8](docs/PLAN.md#8-testing-strategy) to real network, real agents or real forges: temp git repos, `wiremock` fixtures, the fake `Forge`, the mock ACP agent and hostile-repo fixtures.

## Security-sensitive paths

Docent treats PR content as **hostile** until the user trusts the repo, branch or PR ([`PLAN.md` §5.6](docs/PLAN.md#56-trust-and-agent-safety-m2)). Code in these areas guards that boundary:

- trust gating: deciding whether agents, linters or repo config may run
- agent permission and tool policy, including MCP allowlists and answers to ACP permission requests
- forge token handling: acquiring, storing, and stripping tokens from child process environments
- spawning agent and git processes
- verification: running commands and managing the verify worktree

A PR touching any of these ticks the template's security box, includes a denial test, and contains nothing else. Add to this list when new code of this kind appears.

## Conventions

These are already decided. Add a convention here in the PR that first settles it.

- Rust library crates define typed errors with `thiserror`. Use `unwrap`/`expect` only in tests and in startup code where failure should abort, and give `expect` a message saying why.
- All `git` invocations go through `docent-git`. All agent processes go through `docent-agents`. One module owns each kind of child process, so environment scrubbing and logging live in one place.
- Treat agent output as untrusted input: validate it against its schema and check every reported line against the real file.
- UI tests use Vitest and Vue Test Utils on happy-dom, next to the code as `*.test.ts`. Mock Tauri commands with `mockIPC` from `@tauri-apps/api/mocks`, never a hand-rolled `invoke` stub.
- Prefer the standard library and existing dependencies. A new dependency needs the justification described in [CONTRIBUTING.md](CONTRIBUTING.md#dependencies).

## When the design changes

If building a slice shows that `PLAN.md` is wrong, update `PLAN.md` in the same PR, add the decision to `OPEN-QUESTIONS.md`, and tick the template's "Plan changes" box.
