<!-- Title: a conventional commit, e.g. `feat(inbox): show review-requested PRs`. It becomes the squash commit on main. -->

## What and why

<!-- What this PR changes and why. One concern only. -->

Closes #

## How to review

<!-- Required. Which files to read first, where the real decisions are, what to focus on, and what is mechanical and can be skimmed. -->

## How it was tested

<!-- New or changed tests, and the final lines of `just check`. For a bug fix, name the regression test that failed before the fix. -->

## Size

<!-- Approximate changed lines, not counting lockfiles, generated code, fixtures and generator scaffolding. Budget is about 400. If over, explain why this can't be split. -->

## Review notes

<!-- Findings from the agent code review, and how each was handled: fixed, or why not. -->

## Follow-ups

<!-- Things noticed but deliberately left out of scope. -->

## Checklist

- [ ] One concern, within the size budget (or explained above)
- [ ] Tests cover the behaviour change; `just check` passes
- [ ] Agent code review done; findings fixed or answered above
- [ ] **Security-sensitive:** touches a path listed in `AGENTS.md`; includes denial tests and nothing unrelated
- [ ] **Plan changes:** `docs/PLAN.md` and `docs/OPEN-QUESTIONS.md` updated to match
- [ ] **New dependencies:** each one justified above
