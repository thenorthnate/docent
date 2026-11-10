# UI mockups (PROTOTYPE, never merged)

Throwaway Vue + Tailwind mockups for Docent's main screens, built before M1 so the layout is chosen on purpose. Lives on `spike/ui-mockups`, like the M0 and M0.5 spikes.

**Question:** what layout should the PR review view and the inbox use, given everything the MVP (M1–M4b) must fit?

```bash
npm --prefix spike/mockups install
npm --prefix spike/mockups run dev   # http://localhost:5199
```

## Round 2: app shell

Feedback on round 1: inbox B (mail split) and PR view A (findings in one place, steering button) won. Missing: a way back to the inbox, Explore, and multi-repo Projects.

`?screen=app` (the default) is a navigable app built from those choices. Variant **A** puts a VS Code-style activity bar on the far left (Inbox, Review, Explore, Projects, Bookmarks, Steering, Settings); each icon opens a drawer with options for that area, and clicking the active icon collapses it. Variant **B** uses browser-style tabs across the top instead, for comparison. Pages: `&page=inbox|review|explore|project`.

The round 1 variants are still under "Old PR" and "Old inbox".

## Round 1

The pink bar at the bottom switches screen and variant (← → keys work too), toggles a trusted/untrusted PR, and toggles dark/light. All state is in the URL: `?screen=app|pr|inbox&variant=A|B|C&trust=trusted|untrusted&theme=dark|light`.

| Screen | A | B | C |
|---|---|---|---|
| PR view | Three panes (PLAN §5.4 as drawn) | Inline findings + Explain popover | Triage queue |
| Inbox | Dense table | Mail split with preview | Board |

The diff is a static stand-in for CodeMirror 6 (unified view only) and only draws what the M0 spike showed CodeMirror can do. Data is fake (`src/data/mock.ts`).
