/**
 * Theme Context
 *
 * Manages theme (light/dark) for the application
 */

import {
  createContext,
  useContext,
  useState,
  useCallback,
  useEffect,
  ReactNode,
  useMemo,
} from "react";

export type Theme = "dark" | "light";

interface ThemeContextType {
  theme: Theme;
  setTheme: (theme: Theme) => void;
  toggleTheme: () => void;
}

const ThemeContext = createContext<ThemeContextType | null>(null);

interface ThemeProviderProps {
  children: ReactNode;
  defaultTheme?: Theme;
}

export function ThemeProvider({
  children,
  defaultTheme = "dark",
}: ThemeProviderProps) {
  console.log("[ThemeProvider] Initializing with defaultTheme:", defaultTheme);

  const [theme, setThemeState] = useState<Theme>(() => {
    // Set initial theme attribute immediately during state initialization
    console.log("[ThemeProvider] Setting initial theme to:", defaultTheme);
    document.documentElement.setAttribute("data-theme", defaultTheme);
    console.log(
      "[ThemeProvider] data-theme attribute set to:",
      document.documentElement.getAttribute("data-theme"),
    );
    return defaultTheme;
  });

  const setTheme = useCallback((newTheme: Theme) => {
    console.log("[ThemeProvider] setTheme called:", newTheme);
    setThemeState(newTheme);
    document.documentElement.setAttribute("data-theme", newTheme);
    console.log(
      "[ThemeProvider] data-theme attribute now:",
      document.documentElement.getAttribute("data-theme"),
    );
  }, []);

  const toggleTheme = useCallback(() => {
    setThemeState((prev) => {
      const newTheme = prev === "dark" ? "light" : "dark";
      document.documentElement.setAttribute("data-theme", newTheme);
      return newTheme;
    });
  }, []);

  // Memoize context value to prevent unnecessary re-renders in consumers
  const contextValue = useMemo<ThemeContextType>(
    () => ({ theme, setTheme, toggleTheme }),
    [theme, setTheme, toggleTheme],
  );

  return (
    <ThemeContext.Provider value={contextValue}>
      {children}
    </ThemeContext.Provider>
  );
}

export function useTheme(): ThemeContextType {
  const context = useContext(ThemeContext);
  if (!context) {
    throw new Error("useTheme must be used within ThemeProvider");
  }
  return context;
}
