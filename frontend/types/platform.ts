/**
 * Platform utilities
 */

export function extractFileName(path: string): string {
  if (!path) return "";
  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] || "";
}

export function extractFileExtension(path: string): string {
  if (!path) return "";
  const parts = path.split(".");
  return parts.length > 1 ? parts[parts.length - 1]?.toLowerCase() || "" : "";
}

export function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(2)} KB`;
  if (bytes < 1024 * 1024 * 1024)
    return `${(bytes / 1024 / 1024).toFixed(2)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}
