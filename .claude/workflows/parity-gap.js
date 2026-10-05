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
    { title: 'Compact', detail: 'merge findings into docs/specs/features.yaml items (competitors: map, new items)' },
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
        `into docs/specs/features.yaml, the only feature/parity status source of truth (schema + update rules: ` +
        `docs/specs/README.md). One item per concrete feature; do not put status marks in Markdown and do not ` +
        `write narrative summaries.\n\n` +
        `Steps:\n` +
        `1. Read docs/specs/README.md, then grep docs/specs/features.yaml (titles, acceptance, legacy_ids).\n` +
        `2. For each new feature, check if an equivalent item already exists (match by intent, not exact string). ` +
        `If it exists, set/adjust its competitors: entry for this product (yes/partial/no) and add the doc to source. ` +
        `If new, append an item in the right area block with the next free id; set status by grepping the codebase ` +
        `(done only if reachable from the desktop app, CLI or MCP), evidence = real paths, notes for anything unverified.\n` +
        `3. Out-of-scope features (broadcast QC, ABR ladder, bundled tools) get status: dropped with the reason in notes ` +
        `(see COMPETITOR_FEATURE_MATRIX.md "Out of scope"). Add the research URL to that doc's provenance section.\n` +
        `4. Fold any unverified_notes into item notes, explicitly flagged "unverified/gated", not as fact. Validate with ` +
        `the python snippet in docs/specs/README.md and bump updated:.\n` +
        `5. Report a short changelog of items added/changed (ids) — do not restate full item content back.\n\n` +
        `Raw findings JSON:\n${JSON.stringify({ features: allFeatures, unverified_notes: allNotes }, null, 2)}`,
      { phase: 'Compact', label: 'compact-findings' }
    )
  : 'No features found — nothing to compact.'
log(compactSummary)

phase('Cross-reference')
const xrefSummary = await agent(
  `Bitvue project. docs/specs/features.yaml and docs/COMPETITOR_FEATURE_MATRIX.md may have just been edited by a ` +
    `prior step. Validate features.yaml with the snippet in docs/specs/README.md (unique ids, evidence paths exist), ` +
    `then grep docs/*.md for section/ID references (e.g. "§4.9", "OVL-012") that might now be stale, and fix any pointing at content that moved or no ` +
    `longer exists. Confirm each parity doc's "See also" header still points at docs/specs/features.yaml. Report a ` +
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
