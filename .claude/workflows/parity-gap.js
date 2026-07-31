export const meta = {
  name: 'parity-gap',
  description: 'Scan a competitor product for new Bitvue parity gaps and merge findings into the parity docs',
  whenToUse:
    'Run when a tracked competitor (VQ Analyzer, VQ Probe, VEGA, StreamEye, Codecian, or a new one) ships an ' +
    'update, or periodically to re-check for drift. args: { product: <key or free-text name>, urls?: [...], notes?: string }. ' +
    'product keys with built-in URLs: vq_analyzer, vq_probe, vega, streameye, codecian. Pass urls explicitly for ' +
    'anything else (e.g. a new competitor, or a specific changelog/release-notes page to re-check).',
  phases: [
    { title: 'Research', detail: 'web research per source URL, schema-constrained feature extraction' },
    { title: 'Compact', detail: 'merge findings into docs/COMPETITOR_FEATURE_MATRIX.md + PARITY_CHECKLIST.md as dense tables' },
    { title: 'Cross-reference', detail: 'fix stale section/ID references across the 4 parity docs' },
  ],
}

// Mirrors docs/COMPETITOR_FEATURE_MATRIX.md's sourcing (2026-07-31 research pass).
// Extend this as new competitors get tracked — keeps `args: { product: 'vega' }` runnable without re-typing URLs.
const KNOWN_PRODUCTS = {
  vq_analyzer: {
    name: 'ViCueSoft VQ Analyzer',
    urls: [
      'https://cdn.vicuesoft.com/vqAnalyzer/docs/VQAnalyzerUserGuide.html',
      'https://cdn.vicuesoft.com/vqAnalyzer/docs/VQAnalyzerReleaseNotes.html',
      'https://vicuesoft.com/vq-analyzer/',
    ],
  },
  vq_probe: {
    name: 'ViCueSoft VQ Probe',
    urls: ['https://vicuesoft.com/vq-probe/'],
  },
  vega: {
    name: 'Interra Systems VEGA Media Analyzer',
    urls: [
      'https://www.interrasystems.com/vega-media-analyzer.php',
      'https://www.interrasystems.com/pdf/datasheet/vega-datasheet.pdf',
      'https://www.interrasystems.com/Vega-cli.php',
    ],
  },
  streameye: {
    name: 'Elecard StreamEye',
    urls: [
      'https://www.elecard.com/products/video-analysis/streameye',
      'https://www.elecard.com/products/video-analysis/video-quality-estimator',
      'https://www.elecard.com/products/video-analysis/streameye-studio',
    ],
  },
  codecian: {
    name: 'Codecian CodecVisa',
    urls: ['https://www.codecian.com/features.html', 'https://www.codecian.com/codecvisa.html'],
  },
}

const FEATURES_SCHEMA = {
  type: 'object',
  properties: {
    product: { type: 'string' },
    features: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          name: { type: 'string', description: 'concrete feature/overlay-mode/CLI-flag/metric name, verbatim from source' },
          category: { type: 'string', description: 'e.g. overlay-mode, cli-flag, metric, compare-feature, misc-tool' },
          codec: { type: 'string', description: 'codec it applies to, or "global" / "n/a"' },
          description: { type: 'string' },
          source_url: { type: 'string' },
        },
        required: ['name', 'description', 'source_url'],
      },
    },
    unverified_notes: {
      type: 'array',
      items: { type: 'string' },
      description: 'gated/unconfirmed claims worth flagging but not treating as verified fact',
    },
  },
  required: ['product', 'features'],
}

const productKey = args?.product
const known = KNOWN_PRODUCTS[productKey]
const productName = known?.name ?? productKey ?? 'unknown product'
const urls = args?.urls ?? known?.urls ?? []

if (urls.length === 0) {
  log(`No URLs for "${productKey}" — pass args.urls explicitly for products not in KNOWN_PRODUCTS.`)
}

phase('Research')
const researched = urls.length
  ? await parallel(
      urls.map(url => () =>
        agent(
          `Research ${productName} at ${url} for a video-bitstream-analyzer competitive feature matrix ` +
            `(Bitvue project — open-source competitor to this product). Extract CONCRETE features only: ` +
            `overlay/visualization mode names, CLI flags, quality metrics, compare/dual-stream features, ` +
            `distinctive tools. Skip vague marketing language — if something is gated/unconfirmed, put it in ` +
            `unverified_notes instead of features. ${args?.notes ?? ''}`,
          { schema: FEATURES_SCHEMA, phase: 'Research', label: `research:${url}` }
        )
      )
    )
  : []

const allFeatures = researched.filter(Boolean).flatMap(r => r.features.map(f => ({ ...f, product: productName })))
const allNotes = researched.filter(Boolean).flatMap(r => r.unverified_notes ?? [])
log(`Found ${allFeatures.length} concrete features for ${productName} across ${urls.length} source(s).`)

phase('Compact')
const compactSummary = allFeatures.length
  ? await agent(
      `Bitvue project (video bitstream analyzer, docs live in docs/). New competitor research just ran for ` +
        `product "${productName}", producing ${allFeatures.length} concrete features (raw JSON below). Merge this ` +
        `into the existing parity docs as DENSE TABLES — one row per concrete feature, minimal prose. This is a ` +
        `hard house-style rule established by prior user correction (see CLAUDE.md "Working conventions" if present) ` +
        `— do not write narrative summaries.\n\n` +
        `Steps:\n` +
        `1. Read docs/COMPETITOR_FEATURE_MATRIX.md, docs/PARITY_CHECKLIST.md, docs/VQA_PARITY_SPEC_V3.md.\n` +
        `2. For each new feature, check if an equivalent row already exists (match by intent, not exact string). ` +
        `If new, add a row to the right section of COMPETITOR_FEATURE_MATRIX.md with a Bitvue status mark ` +
        `(✅/⚠️/❌/—) — grep the codebase to confirm ✅ rather than guessing; use ⚠️ "unverified" if you can't check.\n` +
        `3. If a new feature is a real, currently-untracked parity gap (not already a Layer 1-6 item), add it to ` +
        `PARITY_CHECKLIST.md Layer 6 with a fresh CMP-N ID, or propose a new Layer if it doesn't fit CMP's scope.\n` +
        `4. Fold any unverified_notes into the doc as explicitly-flagged "unverified/gated" items, not as fact.\n` +
        `5. Report a short changelog of rows added/changed — do not restate full row content back.\n\n` +
        `Raw findings JSON:\n${JSON.stringify({ features: allFeatures, unverified_notes: allNotes }, null, 2)}`,
      { phase: 'Compact', label: 'compact-findings' }
    )
  : 'No features found — nothing to compact.'
log(compactSummary)

phase('Cross-reference')
const xrefSummary = await agent(
  `Bitvue project. docs/VQA_PARITY_SPEC_V3.md, docs/PARITY_CHECKLIST.md, docs/COMPETITOR_FEATURE_MATRIX.md, ` +
    `docs/UX_PARITY_MATRIX.md may have just been edited by a prior step. Grep all four for section/ID references ` +
    `(e.g. "§4.9", "Layer 6", "CMP-0N") that might now be stale, and fix any pointing at content that moved or no ` +
    `longer exists. Confirm each doc's "See also" header still lists all four filenames correctly. Report a ` +
    `one-line confirmation, or a list of fixes made if any were needed.`,
  { phase: 'Cross-reference', label: 'xref-fix' }
)
log(xrefSummary)

return {
  product: productName,
  sourcesChecked: urls,
  featuresFound: allFeatures.length,
  compactSummary,
  xrefSummary,
}
