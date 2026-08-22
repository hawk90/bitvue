/**
 * Welcome screen's "Practice" list -- a handful of basic, codec-agnostic entry points into what
 * the GUI can actually do today. Every row opens the same AV1/IVF sample (see
 * PRACTICE_SAMPLE_FILENAME in sampleCatalog.ts) since that's the only stream with a real
 * end-to-end path; the categories exist to point at *where* to look once it's open (Syntax/Hex/
 * Timeline, the Mode-menu overlays, the diagnostics panels), not to jump straight to a specific
 * view -- there's no view-selection plumbing from the Welcome screen, and building one just for
 * this would be scope beyond what's needed here.
 *
 * Deliberately excludes anything Compare/BD-rate/evidence-diff related -- those are either
 * GUI-incomplete (CompareWorkspace, Phase 7.5) or CLI-only today, so surfacing them as a "practice"
 * entry point would overpromise.
 */

export interface WelcomePracticeItem {
  /** Codicon glyph suffix, e.g. "list-tree" for the "codicon-list-tree" class. */
  icon: string;
  title: string;
  description: string;
}

export const WELCOME_PRACTICE_ITEMS: WelcomePracticeItem[] = [
  {
    icon: "list-tree",
    title: "Explore Structure",
    description: "Walk the bitstream with the Syntax, Hex, and Timeline views",
  },
  {
    icon: "graph-line",
    title: "Visualize Encoding",
    description:
      "See encoder decisions via QP, motion vector, and partition overlays",
  },
  {
    icon: "search",
    title: "Diagnose Artifacts",
    description:
      "Trace quality issues with the Deblocking, Residual, and Diagnostics panels",
  },
];
