// Docent M0.5 spike UI. Plain JS, no build step.
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const $ = (id) => document.getElementById(id);
const esc = (s) => String(s ?? "").replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);

const state = { prs: [], pr: null, checkout: null, files: [], hunk: null };

// Remember form fields between launches (per-machine convenience only).
const FIELDS = ["host", "owner", "repo", "clonePath", "agentCmd", "agentArgs", "agentEnv", "agentMode", "agentAllow", "agentMcp"];
for (const f of FIELDS) {
  try {
    const v = localStorage.getItem("spike." + f);
    if (v !== null) $(f).value = v;
  } catch {}
  $(f).addEventListener("change", () => {
    try { localStorage.setItem("spike." + f, $(f).value); } catch {}
  });
}

function status(el, msg, err = false) {
  $(el).textContent = msg;
  $(el).classList.toggle("err", err);
}
async function guarded(el, fn) {
  try {
    await fn();
  } catch (e) {
    status(el, String(e), true);
  }
}
const lines = (id) => $(id).value.split("\n").map((s) => s.trim()).filter(Boolean);

// ---------- 1. Forge ----------
$("connect").onclick = () =>
  guarded("forgeStatus", async () => {
    status("forgeStatus", "Running gh auth token…");
    status("forgeStatus", await invoke("connect", { host: $("host").value.trim() }));
  });

$("probe").onclick = () =>
  guarded("forgeStatus", async () => {
    status("forgeStatus", "Probing…");
    const rows = await invoke("probe_cmd", { owner: $("owner").value.trim(), repo: $("repo").value.trim() });
    $("probeTable").innerHTML =
      "<tr><th>Probe</th><th>Status</th><th>Headers</th><th>Note</th></tr>" +
      rows
        .map(
          (r) => `<tr><td>${esc(r.name)}<br><span class="muted">${esc(r.method)} ${esc(r.url)}</span></td>
          <td class="${r.ok ? "good" : "bad"}">${esc(r.status ?? "—")}</td>
          <td>${r.headers.map(([k, v]) => `${esc(k)}: ${esc(v)}`).join("<br>")}</td>
          <td>${esc(r.note)}</td></tr>`
        )
        .join("");
    status("forgeStatus", `${rows.filter((r) => r.ok).length}/${rows.length} probes OK`);
  });

$("listPrs").onclick = () =>
  guarded("forgeStatus", async () => {
    state.prs = await invoke("list_prs", { owner: $("owner").value.trim(), repo: $("repo").value.trim() });
    $("prList").innerHTML = state.prs
      .map((p, i) => `<li data-i="${i}">#${p.number} ${esc(p.title)} <span class="muted">by ${esc(p.author)}${p.draft ? " · draft" : ""} · ${esc(p.headRef)} → ${esc(p.baseRef)}</span></li>`)
      .join("");
    status("forgeStatus", `${state.prs.length} open PRs`);
  });

$("prList").onclick = (e) => {
  const li = e.target.closest("li");
  if (!li) return;
  state.pr = state.prs[+li.dataset.i];
  document.querySelectorAll("#prList li").forEach((x) => x.classList.toggle("sel", x === li));
  $("checkout").disabled = false;
  $("reviewTest").disabled = false;
};

$("reviewTest").onclick = () =>
  guarded("forgeStatus", async () => {
    if (!confirm(`Create a PENDING review on #${state.pr.number} (visible only to you) and delete it immediately?`)) return;
    status("forgeStatus", await invoke("pending_review_roundtrip", { owner: $("owner").value.trim(), repo: $("repo").value.trim(), number: state.pr.number }));
  });

// ---------- 2. Worktree & diff ----------
$("checkout").onclick = () =>
  guarded("checkoutStatus", async () => {
    status("checkoutStatus", "Fetching and creating worktree…");
    const c = await invoke("checkout_pr_cmd", {
      repoPath: $("clonePath").value.trim(),
      number: state.pr.number,
      baseBranch: state.pr.baseRef,
      headSha: state.pr.headSha,
    });
    state.checkout = c;
    $("agentCwd").value = c.worktree;
    state.files = parseDiff(c.diff);
    renderDiff();
    status(
      "checkoutStatus",
      `Worktree: ${c.worktree}\nHead ${c.headSha.slice(0, 10)}${c.headMatchesApi ? "" : " (differs from API head: PR updated?)"} · ${state.files.length} files${c.truncated ? " · DIFF TRUNCATED" : ""}`
    );
  });

function parseDiff(text) {
  const files = [];
  let file = null, hunk = null;
  for (const line of text.split("\n")) {
    if (line.startsWith("diff --git ")) {
      file = { oldPath: "", newPath: "", hunks: [] };
      files.push(file);
      hunk = null;
    } else if (!file) {
      continue;
    } else if (!hunk && line.startsWith("--- ")) {
      file.oldPath = line.slice(4).replace(/^a\//, "");
    } else if (!hunk && line.startsWith("+++ ")) {
      file.newPath = line.slice(4).replace(/^b\//, "");
    } else if (line.startsWith("@@")) {
      const m = /^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@/.exec(line);
      hunk = { header: line, oldStart: +m[1], oldCount: m[2] === undefined ? 1 : +m[2], newStart: +m[3], newCount: m[4] === undefined ? 1 : +m[4], lines: [] };
      file.hunks.push(hunk);
    } else if (hunk) {
      hunk.lines.push(line);
    }
  }
  for (const f of files) {
    if (!f.newPath || f.newPath === "/dev/null") f.newPath = f.oldPath;
  }
  return files;
}

function renderDiff() {
  $("diff").innerHTML = state.files
    .map(
      (f, fi) => `<div class="file"><div class="name">${esc(f.newPath)}</div>${f.hunks
        .map(
          (h, hi) => `<div class="hunk" data-f="${fi}" data-h="${hi}"><pre><span class="hh">${esc(h.header)}</span>\n${h.lines
            .map((l) => `<span class="${l[0] === "+" ? "l-add" : l[0] === "-" ? "l-del" : ""}">${esc(l)}</span>`)
            .join("\n")}</pre></div>`
        )
        .join("")}</div>`
    )
    .join("");
}

$("diff").onclick = (e) => {
  const el = e.target.closest(".hunk");
  if (!el) return;
  const f = state.files[+el.dataset.f];
  state.hunk = { file: f, hunk: f.hunks[+el.dataset.h] };
  document.querySelectorAll(".hunk").forEach((x) => x.classList.toggle("sel", x === el));
  $("buildPrompt").disabled = false;
};

// ---------- 3. Agent ----------
const JIRA_RE = /\b[A-Z][A-Z0-9]+-\d+\b/g;

$("buildPrompt").onclick = () =>
  guarded("agentStatus", async () => {
    const { file, hunk } = state.hunk;
    const pr = state.pr;
    status("agentStatus", "Gathering blame and history…");
    const ctx = await invoke("hunk_context", {
      worktree: state.checkout.worktree,
      number: pr.number,
      file: file.newPath,
      oldFile: file.oldPath,
      oldStart: hunk.oldStart,
      oldCount: hunk.oldCount,
    });
    const introducing = [];
    for (const sha of ctx.commits.slice(0, 3)) {
      try {
        const prs = await invoke("prs_for_commit", { owner: $("owner").value.trim(), repo: $("repo").value.trim(), sha });
        for (const p of prs) introducing.push(`#${p.number} ${p.title}\n${(p.body ?? "").slice(0, 1500)}`);
      } catch (e) {
        introducing.push(`(lookup for ${sha.slice(0, 10)} failed: ${e})`);
      }
    }
    const jiraKeys = [...new Set(`${pr.title} ${pr.headRef} ${pr.body ?? ""} ${ctx.log}`.match(JIRA_RE) ?? [])];
    const jira = !$("useJira").checked
      ? "Do not use any Jira or issue-tracker tools for this answer."
      : jiraKeys.length
        ? `Jira keys found: ${jiraKeys.join(", ")}. If you have Jira tools, look them up and use the ticket text.`
        : "No Jira keys found.";
    $("prompt").value = `You are helping a code reviewer understand a pull request. Explain the selected hunk:
1. What it does.
2. Why it is there. Use the history and linked PRs below, and read any other files in the repository you need.
3. Related changes elsewhere in this PR or repository, as path:line references.
4. Callers and impact.
Do not modify any files. Answer in Markdown.

The PR head is checked out in the current directory. The base is ${state.checkout.baseRef}.

## Pull request
#${pr.number} ${pr.title}
${(pr.body ?? "").slice(0, 4000)}

## Selected hunk (${file.newPath})
\`\`\`diff
${hunk.header}
${hunk.lines.join("\n")}
\`\`\`

## Blame of the old lines at the base
${ctx.blame || "(new lines only)"}

## Recent commits touching this file
${ctx.log}

## PRs that introduced the blamed lines
${introducing.join("\n\n") || "(none found)"}

## Jira
${jira}
`;
    status("agentStatus", `Prompt built (${$("prompt").value.length} chars)`);
  });

$("runAgent").onclick = () =>
  guarded("agentStatus", async () => {
    let mcpServers;
    try {
      mcpServers = JSON.parse($("agentMcp").value || "[]");
    } catch (e) {
      throw new Error("mcpServers is not valid JSON: " + e.message);
    }
    const env = lines("agentEnv").map((l) => {
      const i = l.indexOf("=");
      return [l.slice(0, i), l.slice(i + 1)];
    });
    $("answer").textContent = "";
    $("usage").textContent = "";
    $("events").textContent = "";
    status("agentStatus", "Running…");
    const t0 = performance.now();
    const res = await invoke("run_agent", {
      config: {
        command: $("agentCmd").value.trim(),
        args: lines("agentArgs"),
        env,
        cwd: $("agentCwd").value.trim(),
        prompt: $("prompt").value,
        modeId: $("agentMode").value.trim() || null,
        mcpServers,
        shellAllowlist: lines("agentAllow"),
        timeoutSecs: 600,
      },
    });
    const secs = ((performance.now() - t0) / 1000).toFixed(1);
    status("agentStatus", `Done in ${secs}s · stopReason=${res.stopReason} · transcript ${res.transcript}`);
    $("usage").textContent += `\n--- final ---\nusage_update count: ${res.usage.length}\n_meta seen: ${res.metas.length}\nother notifications: ${res.otherNotifications.join(", ") || "none"}\npromptResult: ${JSON.stringify(res.promptResult)}`;
  });

$("cancelAgent").onclick = () => invoke("cancel_agent");

const ev = (s) => {
  $("events").textContent += s + "\n";
  $("events").scrollTop = $("events").scrollHeight;
};
listen("acp", ({ payload: { kind, data } }) => {
  switch (kind) {
    case "chunk":
      $("answer").textContent += data;
      break;
    case "usage":
    case "meta":
      $("usage").textContent += `${kind}: ${JSON.stringify(data)}\n`;
      break;
    case "tool":
      ev(`tool ${data.status ?? ""} ${data.kind ?? ""}: ${data.title ?? data.id}${data.rawInput ? " " + JSON.stringify(data.rawInput) : ""}`);
      break;
    case "permission":
      ev(`PERMISSION ${data.decision.toUpperCase()}: ${data.title ?? ""} [${data.command}] → ${data.optionId ?? "cancelled"}`);
      break;
    case "init":
      ev(`initialize: ${JSON.stringify(data)}`);
      break;
    case "session":
      ev(`session/new: ${JSON.stringify(data)}`);
      break;
    default:
      ev(`${kind}: ${typeof data === "string" ? data : JSON.stringify(data)}`);
  }
});
