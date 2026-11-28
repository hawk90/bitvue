/**
 * Save a text file the user picked in a native save dialog (#605).
 *
 * The renderer sends only the content, a suggested name and a filter. It never sends a path: the
 * main process shows the dialog itself and writes to whatever the user chose there, so a
 * compromised renderer cannot write anywhere on its own (an IPC taking a path from the renderer
 * would be an arbitrary-file-write primitive). Kept free of `electron` imports so the validation
 * and the dialog-then-write order can be unit tested; `ipc/fileExport.ts` wires the real dialog.
 */

/** 64 MiB: far above any CSV/JSON/report this app produces, low enough that one request cannot
 *  fill the disk or stall the main process. */
export const MAX_SAVE_BYTES = 64 * 1024 * 1024;

export interface SaveTextFileRequest {
  /** Suggested file name, no directory (any directory part is dropped). */
  defaultName: string;
  /** Label of the dialog's file-type filter, e.g. "CSV". */
  filterName: string;
  /** Extensions of that filter without the dot, e.g. ["csv"]. */
  extensions: string[];
  content: string;
}

export interface SaveTextFileDeps {
  /** Shows the native save dialog; `null` when the user cancels. */
  chooseSavePath: (options: {
    defaultName: string;
    filters: { name: string; extensions: string[] }[];
  }) => Promise<string | null>;
  writeFile: (filePath: string, content: string) => Promise<void>;
}

const FALLBACK_NAME = "export.txt";
const MAX_NAME_LENGTH = 120;

/** A bare file name: directory parts, control characters and characters Windows forbids are
 *  dropped, so the suggestion can never point outside the dialog's own starting folder. */
export function sanitizeDefaultName(name: string): string {
  const last = name.split(/[\\/]/).pop() ?? "";
  const cleaned = last
    // eslint-disable-next-line no-control-regex
    .replace(/[\u0000-\u001f<>:"|?*]/g, "")
    .trim()
    .slice(0, MAX_NAME_LENGTH);
  return cleaned === "" || cleaned === "." || cleaned === ".."
    ? FALLBACK_NAME
    : cleaned;
}

export function validateSaveRequest(raw: unknown): SaveTextFileRequest {
  if (typeof raw !== "object" || raw === null) {
    throw new Error("saveTextFile: request must be an object");
  }
  const req = raw as Record<string, unknown>;
  if (typeof req.defaultName !== "string") {
    throw new Error("saveTextFile: defaultName must be a string");
  }
  if (typeof req.filterName !== "string" || req.filterName.length > 40) {
    throw new Error(
      "saveTextFile: filterName must be a string of at most 40 characters",
    );
  }
  const extensions = req.extensions;
  if (
    !Array.isArray(extensions) ||
    extensions.length === 0 ||
    extensions.length > 8 ||
    !extensions.every(
      (e) => typeof e === "string" && /^[A-Za-z0-9]{1,8}$/.test(e),
    )
  ) {
    throw new Error(
      'saveTextFile: extensions must be 1-8 plain extensions like "csv"',
    );
  }
  if (typeof req.content !== "string") {
    throw new Error("saveTextFile: content must be a string");
  }
  if (Buffer.byteLength(req.content, "utf8") > MAX_SAVE_BYTES) {
    throw new Error(
      `saveTextFile: content is larger than ${MAX_SAVE_BYTES} bytes`,
    );
  }
  return {
    defaultName: sanitizeDefaultName(req.defaultName),
    filterName: req.filterName,
    extensions: extensions as string[],
    content: req.content,
  };
}

/** Validates, asks the user where to save, writes. Returns the path written, or `null` when the
 *  user cancelled (not an error). */
export async function saveTextFile(
  raw: unknown,
  deps: SaveTextFileDeps,
): Promise<string | null> {
  const req = validateSaveRequest(raw);
  const chosen = await deps.chooseSavePath({
    defaultName: req.defaultName,
    filters: [{ name: req.filterName, extensions: req.extensions }],
  });
  if (chosen === null || chosen === "") return null;
  await deps.writeFile(chosen, req.content);
  return chosen;
}
