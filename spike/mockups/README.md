# UI mockups (PROTOTYPE, never merged)

Throwaway Vue + Tailwind mockups for Docent's main screens, built before M1 so the layout is chosen on purpose. Lives on `spike/ui-mockups`, like the M0 and M0.5 spikes.

**Question:** what layout should the PR review view and the inbox use, given everything the MVP (M1–M4b) must fit?

```bash
npm --prefix spike/mockups install
npm --prefix spike/mockups run dev   # http://localhost:5199
```

The pink bar at the bottom switches screen and variant (← → keys work too), toggles a trusted/untrusted PR, and toggles dark/light. All state is in the URL: `?screen=pr|inbox&variant=A|B|C&trust=trusted|untrusted&theme=dark|light`.

| Screen | A | B | C |
|---|---|---|---|
| PR view | Three panes (PLAN §5.4 as drawn) | Inline findings + Explain popover | Triage queue |
| Inbox | Dense table | Mail split with preview | Board |

The diff is a static stand-in for CodeMirror 6 (unified view only) and only draws what the M0 spike showed CodeMirror can do. Data is fake (`src/data/mock.ts`).
