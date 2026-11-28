/**
 * Save a text file through a native save dialog (CSV / JSON / report export, #605).
 *
 * The renderer sends only the content, a suggested name and a filter -- never a path. The main
 * process shows the dialog and writes to the path the user picks, so nothing in the renderer can
 * choose where a file is written. See `bitvue-desktop/electron/saveTextFile.ts`.
 */

import { requireBridge } from "./core";

export interface SaveTextFileRequest {
  /** Suggested file name, no directory. */
  defaultName: string;
  /** Label of the dialog's file-type filter, e.g. "CSV". */
  filterName: string;
  /** Extensions of that filter without the dot, e.g. `["csv"]`. */
  extensions: string[];
  content: string;
}

/** The path written, or `null` when the user cancelled the dialog (nothing was written).
 *  Throws when the request is invalid (e.g. over 64 MiB) or the write fails. */
export async function saveTextFile(
  request: SaveTextFileRequest,
): Promise<string | null> {
  return requireBridge().saveTextFile(request);
}
