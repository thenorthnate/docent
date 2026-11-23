import type { CiStatus } from "@/stores/inbox";

/** A PR's description, checks and timeline, as `pull_overview` reports it. Every string came from the forge. */
export type PullOverview = {
  title: string;
  author: string;
  /** Markdown, or `null` when the PR has no description. */
  body: string | null;
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
