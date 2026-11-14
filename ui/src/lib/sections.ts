import type { Section } from "@/stores/inbox";

/** Each section's name, as the drawer and the list heading show it. */
export const SECTION_LABELS: Record<Section, string> = {
  review_requested: "Review requested",
  assigned: "Assigned",
  authored: "Authored by me",
  mentioned: "Mentioned",
};

/** What the list says when a section has no PRs and every host answered. */
export const SECTION_EMPTY: Record<Section, string> = {
  review_requested: "No review requests in your watched repos.",
  assigned: "No PRs assigned to you in your watched repos.",
  authored: "You have no open PRs in your watched repos.",
  mentioned: "No PRs mention you in your watched repos.",
};
