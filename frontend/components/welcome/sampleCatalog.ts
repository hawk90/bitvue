/**
 * Welcome screen's "Samples" quick-open catalog, grouped by codec.
 *
 * Every codec x container combination the file-open dialog currently accepts (see
 * `useAppFileOperations.ts`'s extension filter list), not just one entry per codec -- covers the
 * full range of input formats each codec can realistically appear in per the container-parsing
 * code in `crates/bitvue-formats`.
 *
 * `available` is a deliberate editorial flag, not derived from file existence: as of 2026-08-21,
 * AV1/IVF is the ONLY combination with a real end-to-end path (bitvue-indexer only indexes
 * IVF/AV1 streams -- MP4/MKV/WebM/TS demuxing exists in bitvue-formats but isn't wired into the
 * live bitvue-sidecar, and VP9/VVC have no container-extraction code there at all). Several of
 * the "unavailable" rows below DO have real sample files already sitting in `samples/`
 * (foreman_hevc.mp4, foreman_h264.mp4, foreman_vp9.webm, foreman_vp9.mkv) -- they're still marked
 * unavailable because opening them today only produces the sidecar's honest "Indexing is only
 * implemented for IVF/AV1 streams so far" diagnostic, not real frame analysis. Flip `available`
 * to `true` here once a combination's backend actually lands, no other wiring needed --
 * WelcomeContainer resolves real paths for whatever's marked available.
 *
 * Filenames for combinations that don't have a sample file yet are the planned names (matching
 * this repo's `foreman_<codec>.<ext>` convention) -- the actual video bytes are expected to be
 * added separately, not part of this catalog.
 */

export interface WelcomeSampleEntry {
  container: string;
  filename: string;
  available: boolean;
}

export interface WelcomeSampleGroup {
  codec: string;
  /** All groups start collapsed -- Samples now sits next to Practice (see
   *  WelcomePracticeList), which is the actual "start here" entry point, so Samples doesn't
   *  need to force one codec open by default anymore. */
  defaultExpanded: boolean;
  entries: WelcomeSampleEntry[];
}

export const WELCOME_SAMPLE_GROUPS: WelcomeSampleGroup[] = [
  {
    codec: "AV1",
    defaultExpanded: false,
    entries: [
      { container: "IVF", filename: "foreman_av1.ivf", available: true },
      { container: "MP4", filename: "foreman_av1.mp4", available: false },
      { container: "MKV", filename: "foreman_av1.mkv", available: false },
      { container: "WebM", filename: "foreman_av1.webm", available: false },
      { container: "TS", filename: "foreman_av1.ts", available: false },
    ],
  },
  {
    codec: "HEVC",
    defaultExpanded: false,
    entries: [
      {
        container: "Raw (.265)",
        filename: "foreman_hevc.265",
        available: false,
      },
      { container: "MP4", filename: "foreman_hevc.mp4", available: false },
      { container: "MKV", filename: "foreman_hevc.mkv", available: false },
      { container: "TS", filename: "foreman_hevc.ts", available: false },
    ],
  },
  {
    codec: "H.264",
    defaultExpanded: false,
    entries: [
      {
        container: "Raw (.264)",
        filename: "foreman_h264.264",
        available: false,
      },
      { container: "MP4", filename: "foreman_h264.mp4", available: false },
      { container: "MKV", filename: "foreman_h264.mkv", available: false },
      { container: "TS", filename: "foreman_h264.ts", available: false },
    ],
  },
  {
    codec: "VP9",
    defaultExpanded: false,
    entries: [
      { container: "IVF", filename: "foreman_vp9.ivf", available: false },
      { container: "WebM", filename: "foreman_vp9.webm", available: false },
      { container: "MKV", filename: "foreman_vp9.mkv", available: false },
    ],
  },
  {
    codec: "VVC",
    defaultExpanded: false,
    entries: [
      {
        container: "Raw (.vvc)",
        filename: "foreman_vvc.vvc",
        available: false,
      },
      { container: "MP4", filename: "foreman_vvc.mp4", available: false },
    ],
  },
];

/** Stable module-level reference (not recomputed per render) -- see useSamplePaths' doc on why
 *  that matters for its effect's dependency array. */
export const AVAILABLE_SAMPLE_FILENAMES = WELCOME_SAMPLE_GROUPS.flatMap((g) =>
  g.entries.filter((e) => e.available).map((e) => e.filename),
);

/** The sample WelcomePracticeList's rows open -- AV1/IVF is the only combination with a real
 *  end-to-end analysis path today (see file doc above), so it's the only honest target for a
 *  "try this workflow" entry point. */
export const PRACTICE_SAMPLE_FILENAME = "foreman_av1.ivf";
