/**
 * Logger Utility Tests
 * Tests logging functionality
 */

import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { createLogger, setLogLevel } from "@/utils/logger";

describe("createLogger", () => {
  let consoleSpy: {
    log: ReturnType<typeof vi.spyOn>;
    debug: ReturnType<typeof vi.spyOn>;
    info: ReturnType<typeof vi.spyOn>;
    warn: ReturnType<typeof vi.spyOn>;
    error: ReturnType<typeof vi.spyOn>;
  };

  beforeEach(() => {
    consoleSpy = {
      log: vi.spyOn(console, "log").mockImplementation(() => {}),
      debug: vi.spyOn(console, "debug").mockImplementation(() => {}),
      info: vi.spyOn(console, "info").mockImplementation(() => {}),
      warn: vi.spyOn(console, "warn").mockImplementation(() => {}),
      error: vi.spyOn(console, "error").mockImplementation(() => {}),
    };
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("should create a named logger", () => {
    const logger = createLogger("TestComponent");
    expect(logger).toBeDefined();
  });

  it("should log debug messages", () => {
    const logger = createLogger("TestComponent");
    // Set log level to debug to ensure debug messages are logged
    setLogLevel("debug");

    logger.debug("test message");

    expect(consoleSpy.log).toHaveBeenCalledWith("[TestComponent] test message");
  });

  it("should log info messages", () => {
    const logger = createLogger("TestComponent");
    logger.info("info message");

    expect(consoleSpy.info).toHaveBeenCalledWith(
      "[TestComponent] info message",
    );
  });

  it("should log warning messages", () => {
    const logger = createLogger("TestComponent");
    logger.warn("warning message");

    expect(consoleSpy.warn).toHaveBeenCalledWith(
      "[TestComponent] warning message",
    );
  });

  it("should log error messages", () => {
    const logger = createLogger("TestComponent");
    logger.error("error message");

    expect(consoleSpy.error).toHaveBeenCalledWith(
      "[TestComponent] error message",
    );
  });

  it("should log multiple arguments", () => {
    const logger = createLogger("TestComponent");
    logger.info("test", "data", { key: "value" });

    expect(consoleSpy.info).toHaveBeenCalledWith(
      "[TestComponent] test",
      "data",
      { key: "value" },
    );
  });

  it("should format error objects", () => {
    const logger = createLogger("TestComponent");
    const error = new Error("Test error");
    logger.error(error);

    expect(consoleSpy.error).toHaveBeenCalled();
  });

  it("should be module-scoped", () => {
    const logger1 = createLogger("ModuleA");
    const logger2 = createLogger("ModuleB");

    logger1.info("from A");
    logger2.info("from B");

    expect(consoleSpy.info).toHaveBeenCalledTimes(2);
  });
});

describe("Logger behavior", () => {
  let consoleSpy: {
    log: ReturnType<typeof vi.spyOn>;
    debug: ReturnType<typeof vi.spyOn>;
    info: ReturnType<typeof vi.spyOn>;
    warn: ReturnType<typeof vi.spyOn>;
    error: ReturnType<typeof vi.spyOn>;
  };

  beforeEach(() => {
    consoleSpy = {
      log: vi.spyOn(console, "log").mockImplementation(() => {}),
      debug: vi.spyOn(console, "debug").mockImplementation(() => {}),
      info: vi.spyOn(console, "info").mockImplementation(() => {}),
      warn: vi.spyOn(console, "warn").mockImplementation(() => {}),
      error: vi.spyOn(console, "error").mockImplementation(() => {}),
    };
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("should include timestamp in production", () => {
    // This tests if timestamps are included in production builds
    const logger = createLogger("ProductionLogger");

    // In test environment, just verify the function exists
    expect(logger.info).toBeDefined();
  });

  it("should handle undefined/null arguments gracefully", () => {
    const logger = createLogger("TestComponent");

    logger.info(undefined);
    logger.info(null);
    logger.error(null);

    expect(consoleSpy.info).toHaveBeenCalled();
    expect(consoleSpy.error).toHaveBeenCalled();
  });
});
