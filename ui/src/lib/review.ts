import type { CiStatus } from "@/stores/inbox";

/** A PR's description, checks and timeline, as `pull_overview` reports it. Every string came from the forge. */
export type PullOverview = {
  title: string;
  author: string;
  /** Markdown, or `null` when the PR has no description. */
  body: string | null;
  /** The base branch's tip, as the forge has it for this PR. */
  base_sha: string;
  ci: CiStatus;
  checks: Check[];
  /** The latest commits, reviews and comments, oldest first. */
  timeline: TimelineItem[];
  /** There are earlier timeline items than Docent fetched. */
  timeline_truncated: boolean;
};

export type CheckState = "passing" | "failing" | "pending" | "skipped";

export type Check = { name: string; state: CheckState };

export type ReviewVerdict = "approved" | "changes_requested" | "commented" | "dismissed";

/** A timeline event. `at` is the forge's RFC 3339 timestamp. */
export type TimelineItem =
  | { kind: "commit"; sha: string; headline: string; author: string; at: string }
  | { kind: "review"; author: string; verdict: ReviewVerdict; body: string | null; at: string }
  | { kind: "comment"; author: string; body: string; at: string };

/** How the timeline words each review verdict, after the author's login. */
export const VERDICT_LABELS: Record<ReviewVerdict, string> = {
  approved: "approved",
  changes_requested: "requested changes",
  commented: "reviewed",
  dismissed: "had a review dismissed",
};

export const CHECK_LABELS: Record<CheckState, string> = {
  passing: "Passed",
  failing: "Failed",
  pending: "Pending",
  skipped: "Skipped",
};

export type FileStatus = "added" | "modified" | "deleted" | "renamed";

/** A file a PR changes, as `changed_files` reports it. Paths came from the PR, so they're shown as text only. */
export type ChangedFile = {
  path: string;
  /** Where a renamed file was before. */
  old_path: string | null;
  status: FileStatus;
  /** `null` for a binary file. */
  lines: { added: number; deleted: number } | null;
  /** The contents before and after, as object IDs; `null` on a side the file doesn't exist on. */
  old_blob: string | null;
  new_blob: string | null;
};

/** A hunk as git's `@@ -old_start,old_lines +new_start,new_lines @@` header gives it. */
export type Hunk = { old_start: number; old_lines: number; new_start: number; new_lines: number };

/** One file's diff, as `file_diff` reports it. Contents came from the PR, so they're shown as text only. */
export type FileDiff =
  | { kind: "text"; old: string; new: string; hunks: Hunk[] }
  | { kind: "binary" }
  | { kind: "too_large"; bytes: number };

/** How the file tree marks each status. */
export const FILE_STATUSES: Record<FileStatus, { label: string; letter: string; tone: string }> = {
  added: { label: "Added", letter: "A", tone: "text-emerald-600 dark:text-emerald-400" },
  modified: { label: "Modified", letter: "M", tone: "text-amber-600 dark:text-amber-400" },
  deleted: { label: "Deleted", letter: "D", tone: "text-destructive" },
  renamed: { label: "Renamed", letter: "R", tone: "text-sky-600 dark:text-sky-400" },
};
