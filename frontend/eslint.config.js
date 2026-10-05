import js from "@eslint/js";
import globals from "globals";
import tseslint from "typescript-eslint";
import react from "eslint-plugin-react";
import reactHooks from "eslint-plugin-react-hooks";

export default tseslint.config(
  {
    ignores: [
      "dist/",
      "node_modules/",
      "**/*.config.js",
      "**/*.config.cjs",
      "**/*.config.ts",
      ".eslintrc.cjs",
    ],
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  react.configs.flat.recommended,
  {
    files: ["**/*.{ts,tsx,js,jsx}"],
    languageOptions: {
      ecmaVersion: "latest",
      sourceType: "module",
      globals: { ...globals.browser, ...globals.es2022 },
      parserOptions: { ecmaFeatures: { jsx: true } },
    },
    plugins: { "react-hooks": reactHooks },
    settings: { react: { version: "detect" } },
    rules: {
      // TypeScript
      "@typescript-eslint/no-explicit-any": "warn",
      "@typescript-eslint/no-unused-vars": [
        "warn",
        { argsIgnorePattern: "^_", varsIgnorePattern: "^_" },
      ],
      "@typescript-eslint/no-require-imports": "warn",

      // React
      "react/react-in-jsx-scope": "off", // Not needed with React 17+
      "react/prop-types": "off", // TypeScript handles this
      "react/display-name": "warn",
      "react/no-unescaped-entities": "warn",

      // React Hooks
      "react-hooks/rules-of-hooks": "warn", // Has false positives with HOC patterns
      "react-hooks/exhaustive-deps": "warn",

      // General
      "no-prototype-builtins": "warn",
      "no-case-declarations": "warn",
      "no-control-regex": "warn",
    },
  },
);
