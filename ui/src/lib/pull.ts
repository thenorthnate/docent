import type { PullSummary } from "@/stores/inbox";

/** A key naming one PR across hosts, like `github.com/octocat/Hello-World#1347`. */
export function pullKey(pr: Pick<PullSummary, "host" | "owner" | "repo" | "number">): string {
  return `${pr.host}/${pr.owner}/${pr.repo}#${pr.number}`;
}
