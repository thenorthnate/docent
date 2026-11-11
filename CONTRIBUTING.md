# Contributing to Docent

Docent is built in small, reviewable steps, with quality and correctness ahead of speed. Every change should be something one person can review properly in one sitting. These rules apply to human and agent contributors alike. Agents also follow [`AGENTS.md`](AGENTS.md).

The design is in [`docs/PLAN.md`](docs/PLAN.md), and decisions and open unknowns are in [`docs/OPEN-QUESTIONS.md`](docs/OPEN-QUESTIONS.md). Read the relevant sections before changing anything they cover.

## From milestone to PR

1. **Milestones** come from [`PLAN.md` §7](docs/PLAN.md#7-milestones). When one starts, it gets a tracking issue.
2. **Slices.** The milestone is broken into slices, each a PR-sized piece of work, and each slice gets its own issue. The tracking issue lists them as a checklist. An agent may draft the breakdown, but the maintainer approves it before any issues are filed. Slices are planned when their milestone starts, not ahead of time.
3. **Vertical slices.** Each slice delivers something you can see or test end to end, even if it's thin, such as one inbox row or one diff rendered. Avoid horizontal layers like "the whole schema" followed by "the whole API".
4. **One slice, one PR.**

### Spikes

M0.5 (work-laptop proof) and M0 (viewer spike) are experiments that answer questions. Their code lives on `spike/*` branches and is **never merged**. What lands on `main` is a short write-up in `docs/spikes/<name>.md` with the findings, plus updates to `OPEN-QUESTIONS.md`. Real code starts again from a clean scaffold.

## Branches and commits

- Branch off `main` as `<type>/<short-desc>`, for example `feat/inbox-list` or `fix/interdiff-force-push`.
- **Types** follow [Conventional Commits](https://www.conventionalcommits.org/): `feat`, `fix`, `refactor`, `test`, `docs`, `build`, `ci`, `chore`.
- PRs are **squash-merged**, and the PR title becomes the commit on `main`. Write the title as a conventional commit (`feat(inbox): show review-requested PRs`), because `release-please` builds the changelog from it.
- Commits inside a PR exist to help review. Make them small and logical, such as a failing test, then the implementation, then a refactor. Once review has started, push follow-ups as new commits instead of force-pushing, so the reviewer can see what changed.
- Stacked PRs are allowed only when a slice can't be split any other way. Each PR in a stack must still meet the size budget.
- `main` is protected: CI must pass and the maintainer must approve.

## PR size

- **One concern per PR.** A feature, a refactor and a bug fix are three PRs. Record drive-by improvements as follow-ups instead of including them.
- **Budget:** about **400 changed lines**. Lockfiles, generated code, test fixtures and scaffolding from official generators don't count.
- Over budget: split the PR, or explain in its description why it can't be split. The template asks for this.

## PR descriptions

Every PR uses [the template](.github/pull_request_template.md). The **How to review** section is required. It tells the reviewer which files to read first, where the real decisions are, and which parts are mechanical. A good description is also the context Docent's own lenses will read when Docent reviews its own PRs.

## Tests

Correctness is shown by tests, not asserted:

- Every behaviour change ships with tests in the same PR.
- Bug fixes start with a failing regression test.
- Test-first (red, green, refactor) is required in the pure-logic crates (`docent-git`, `docent-store`, agent output parsing, trust and permission policy) and encouraged everywhere else.
- Security-sensitive code needs tests that prove the **denial**, not just the allowed case (see [Security-sensitive changes](#security-sensitive-changes)).
- There's no coverage percentage to hit. Reviewers judge whether the tests are enough.

The test approach for each layer (temp git repos, HTTP fixtures, the fake `Forge`, the mock ACP agent, hostile-repo fixtures) is in [`PLAN.md` §8](docs/PLAN.md#8-testing-strategy).

## Quality gates

One local command runs the same checks as CI:

```bash
just check
```

It runs `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, and the UI's lint (ESLint, no warnings allowed), typecheck (`vue-tsc`) and tests (Vitest). It must pass before a PR is marked ready for review. CI runs it on macOS arm64, along with a release build and a Linux `cargo check`.

You need Rust (stable), Node 24 or later, pnpm (the version is pinned in `package.json`) and `just` (`brew install just`). Run `just setup` once to install dependencies, and `just dev` to run the app.

## Review

- **Agent review first.** Before a PR is marked ready, it goes through an AI code review (`/code-review` in Claude Code, or the equivalent in another harness). Each finding is fixed or answered under "Review notes" in the PR.
- **Then the maintainer** reviews and merges.
- **Dogfooding.** From M3 onward, Docent reviews its own PRs. The repo's lenses will live in `.docent/lenses/`, and this section will be updated in the PR that turns that on.

## Dependencies

The planned building blocks are listed in [`PLAN.md` §3.3](docs/PLAN.md#33-off-the-shelf-building-blocks). Any other crate or npm package needs a short note in the PR saying what it's for, what the alternatives were (including the standard library or an existing dependency), and whether it's maintained and widely used.

## Security-sensitive changes

Docent runs coding agents near hostile PR content. The trust model is in [`PLAN.md` §5.6](docs/PLAN.md#56-trust-and-agent-safety-m2), and the list of security-sensitive paths is in [`AGENTS.md`](AGENTS.md#security-sensitive-paths). A PR touching one of them:

- says so in the template,
- includes tests proving the denial cases, and
- contains no unrelated changes.

## Changing the design

Building will show where the plan was wrong. When it does, the PR that departs from the plan also updates `PLAN.md` and records the decision in `OPEN-QUESTIONS.md`. The code and the design docs should never disagree on `main`.
