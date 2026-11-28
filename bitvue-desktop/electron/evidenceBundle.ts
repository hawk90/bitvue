/**
 * Evidence bundle export through a folder the user picked in a native dialog (#640).
 *
 * The renderer sends the manifest fields and an optional screenshot -- never an output directory.
 * The main process shows the folder dialog itself and gives the sidecar only the folder chosen
 * there, so a compromised renderer cannot make the bundle land in a directory of its choosing.
 * Kept free of `electron` imports so the validation and the dialog-then-export order can be unit
 * tested; `ipc/evidenceExport.ts` wires the real dialog and sidecar.
 */

/** 32 MiB of base64 text: a full-window PNG is a few MiB; this stops one request from stalling
 *  the main process or filling the disk. */
export const MAX_SCREENSHOT_CHARS = 32 * 1024 * 1024;
const MAX_FIELD_CHARS = 64;
const PNG_DATA_URL_PREFIX = "data:image/png;base64,";

export interface EvidenceBundleRequest {
  workspace?: string;
  mode?: string;
  orderType?: "display" | "decode";
  screenshotDataUrl?: string;
}

export interface EvidenceBundleDeps {
  /** Shows the native folder dialog; `null` when the user cancels. */
  chooseDirectory: () => Promise<string | null>;
  /** The sidecar's `export_evidence_bundle`, called with the folder the dialog returned. */
  exportBundle: (params: {
    output_dir: string;
    workspace?: string;
    mode?: string;
    order_type?: string;
    screenshot_data_url?: string;
  }) => Promise<unknown>;
}

function optionalShortString(
  req: Record<string, unknown>,
  key: string,
): string | undefined {
  const value = req[key];
  if (value === undefined) return undefined;
  if (typeof value !== "string" || value.length > MAX_FIELD_CHARS) {
    throw new Error(
      `exportEvidenceBundle: ${key} must be a string of at most ${MAX_FIELD_CHARS} characters`,
    );
  }
  return value;
}

export function validateEvidenceRequest(raw: unknown): EvidenceBundleRequest {
  if (typeof raw !== "object" || raw === null) {
    throw new Error("exportEvidenceBundle: request must be an object");
  }
  const req = raw as Record<string, unknown>;
  const orderType = req.orderType;
  if (
    orderType !== undefined &&
    orderType !== "display" &&
    orderType !== "decode"
  ) {
    throw new Error(
      'exportEvidenceBundle: orderType must be "display" or "decode"',
    );
  }
  const shot = req.screenshotDataUrl;
  if (shot !== undefined) {
    if (
      typeof shot !== "string" ||
      !shot.startsWith(PNG_DATA_URL_PREFIX) ||
      shot.length > MAX_SCREENSHOT_CHARS
    ) {
      throw new Error(
        "exportEvidenceBundle: screenshotDataUrl must be a PNG data URL within the size limit",
      );
    }
  }
  return {
    workspace: optionalShortString(req, "workspace"),
    mode: optionalShortString(req, "mode"),
    orderType,
    screenshotDataUrl: shot,
  };
}

/** Validates, asks the user for a folder, exports there. Returns the sidecar's result, or `null`
 *  when the user cancelled the dialog (nothing was written). */
export async function exportEvidenceBundleViaDialog(
  raw: unknown,
  deps: EvidenceBundleDeps,
): Promise<unknown | null> {
  const req = validateEvidenceRequest(raw);
  const dir = await deps.chooseDirectory();
  if (dir === null || dir === "") return null;
  return deps.exportBundle({
    output_dir: dir,
    workspace: req.workspace,
    mode: req.mode,
    order_type: req.orderType,
    screenshot_data_url: req.screenshotDataUrl,
  });
}
