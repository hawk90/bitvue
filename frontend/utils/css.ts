/**
 * CSS Utilities
 *
 * Helper functions for CSS-related operations
 */

/**
 * Get CSS variable value from document root
 * Falls back to the name itself if not found or in SSR context
 */
export function getCssVar(name: string): string {
  if (typeof document === "undefined") return name;
  return (
    getComputedStyle(document.documentElement).getPropertyValue(name).trim() ||
    name
  );
}

/**
 * Set CSS variable value on document root
 */
export function setCssVar(name: string, value: string): void {
  if (typeof document === "undefined") return;
  document.documentElement.style.setProperty(name, value);
}

/**
 * Get multiple CSS variables at once
 */
export function getCssVars(names: string[]): Record<string, string> {
  const result: Record<string, string> = {};
  if (typeof document === "undefined") {
    names.forEach((name) => {
      result[name] = name;
    });
    return result;
  }

  const styles = getComputedStyle(document.documentElement);
  names.forEach((name) => {
    result[name] = styles.getPropertyValue(name).trim() || name;
  });
  return result;
}

/**
 * Parse a CSS size value (e.g., "12px", "1rem") to pixels
 * Returns NaN if unable to parse
 */
export function parseCssPixels(value: string): number {
  if (typeof document === "undefined") return NaN;

  const temp = document.createElement("div");
  temp.style.position = "absolute";
  temp.style.visibility = "hidden";
  temp.style.height = value;
  document.body.appendChild(temp);

  const result = temp.offsetHeight;
  document.body.removeChild(temp);

  return result;
}

/**
 * Check if a CSS variable is defined
 */
export function hasCssVar(name: string): boolean {
  if (typeof document === "undefined") return false;
  const value = getComputedStyle(document.documentElement).getPropertyValue(
    name,
  );
  return value !== null && value.trim() !== "";
}
