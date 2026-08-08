import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    globals: true,
    environment: "node",
    include: ["tests/**/*.{test,spec}.ts"],
    // Integration test spawns a real child process and can take longer than vitest's default
    // 5s under cold cargo/OS scheduling — give it real headroom rather than flaking.
    testTimeout: 20_000,
  },
});
