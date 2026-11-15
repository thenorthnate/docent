import type { CiStatus, PullDetails, PullSummary, ReviewState } from "@/stores/inbox";

/** A key naming one PR across hosts, like `github.com/octocat/Hello-World#1347`. */
export function pullKey(pr: Pick<PullSummary, "host" | "owner" | "repo" | "number">): string {
  return `${pr.host}/${pr.owner}/${pr.repo}#${pr.number}`;
}

/** How a row names its CI status. */
export const CI_LABELS: Record<CiStatus, string> = {
  passing: "Checks passing",
  failing: "Checks failing",
  pending: "Checks pending",
  no_checks: "No checks",
};

/** How a row names its review state. */
export const REVIEW_LABELS: Record<ReviewState, string> = {
  approved: "Approved",
  changes_requested: "Changes requested",
  review_required: "Review required",
  reviewed: "Reviewed",
  no_reviews: "No reviews",
};

/** A PR's size as a row shows it, like `+120 −30 · 4 files`. */
export function formatSize(details: PullDetails): string {
  const files = details.changed_files === 1 ? "file" : "files";
  return `+${details.additions} −${details.deletions} · ${details.changed_files} ${files}`;
}
