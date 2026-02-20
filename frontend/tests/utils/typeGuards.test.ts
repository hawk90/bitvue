/**
 * Type Guard Utility Tests
 * Tests runtime type checking utilities
 */

import { describe, it, expect } from "vitest";
import {
  isObject,
  isString,
  isNumber,
  isBoolean,
  isArray,
  isFunction,
  isError,
  isErrorWithCode,
  isTauriError,
  isTauriSuccess,
  isValidFrameIndex,
  isValidFilePath,
  hasProperty,
  getErrorMessage,
  tryParseJSON,
  assertNotNil,
  type ErrorWithCode,
  type TauriError,
  type TauriSuccess,
} from "@/utils/typeGuards";

describe("isObject", () => {
  it("should return true for plain objects", () => {
    expect(isObject({})).toBe(true);
    expect(isObject({ a: 1 })).toBe(true);
  });

  it("should return false for null", () => {
    expect(isObject(null)).toBe(false);
  });

  it("should return false for arrays", () => {
    expect(isObject([])).toBe(false);
    expect(isObject([1, 2, 3])).toBe(false);
  });

  it("should return false for primitives", () => {
    expect(isObject("string")).toBe(false);
    expect(isObject(123)).toBe(false);
    expect(isObject(true)).toBe(false);
  });

  it("should return false for undefined", () => {
    expect(isObject(undefined)).toBe(false);
  });
});

describe("isString", () => {
  it("should return true for strings", () => {
    expect(isString("")).toBe(true);
    expect(isString("hello")).toBe(true);
    expect(isString("123")).toBe(true);
  });

  it("should return false for non-strings", () => {
    expect(isString(123)).toBe(false);
    expect(isString(null)).toBe(false);
    expect(isString(undefined)).toBe(false);
    expect(isString({})).toBe(false);
  });
});

describe("isNumber", () => {
  it("should return true for numbers", () => {
    expect(isNumber(0)).toBe(true);
    expect(isNumber(123)).toBe(true);
    expect(isNumber(-456)).toBe(true);
    expect(isNumber(1.5)).toBe(true);
  });

  it("should return false for NaN", () => {
    expect(isNumber(NaN)).toBe(false);
  });

  it("should return false for non-numbers", () => {
    expect(isNumber("123")).toBe(false);
    expect(isNumber(null)).toBe(false);
    expect(isNumber(undefined)).toBe(false);
    expect(isNumber({})).toBe(false);
  });
});

describe("isBoolean", () => {
  it("should return true for booleans", () => {
    expect(isBoolean(true)).toBe(true);
    expect(isBoolean(false)).toBe(true);
  });

  it("should return false for non-booleans", () => {
    expect(isBoolean(1)).toBe(false);
    expect(isBoolean(0)).toBe(false);
    expect(isBoolean("true")).toBe(false);
    expect(isBoolean(null)).toBe(false);
  });
});

describe("isArray", () => {
  it("should return true for arrays", () => {
    expect(isArray([])).toBe(true);
    expect(isArray([1, 2, 3])).toBe(true);
    expect(isArray(["a", "b"])).toBe(true);
  });

  it("should return false for non-arrays", () => {
    expect(isArray({})).toBe(false);
    expect(isArray(null)).toBe(false);
    expect(isArray(undefined)).toBe(false);
    expect(isArray("string")).toBe(false);
  });

  it("should work with generic types", () => {
    const arr: unknown = [1, 2, 3];
    if (isArray<number>(arr)) {
      expect(arr[0]).toBe(1);
    }
  });
});

describe("isFunction", () => {
  it("should return true for functions", () => {
    expect(isFunction(() => {})).toBe(true);
    expect(isFunction(function () {})).toBe(true);
    expect(isFunction(async () => {})).toBe(true);
  });

  it("should return false for non-functions", () => {
    expect(isFunction({})).toBe(false);
    expect(isFunction(null)).toBe(false);
    expect(isFunction(undefined)).toBe(false);
    expect(isFunction("string")).toBe(false);
  });
});

describe("isError", () => {
  it("should return true for Error instances", () => {
    expect(isError(new Error("test"))).toBe(true);
    expect(isError(new TypeError("test"))).toBe(true);
    expect(isError(new ReferenceError("test"))).toBe(true);
  });

  it("should return true for error-like objects", () => {
    expect(isError({ name: "Error", message: "test" })).toBe(true);
  });

  it("should return false for non-errors", () => {
    expect(isError({})).toBe(false);
    expect(isError({ name: "Error" })).toBe(false);
    expect(isError({ message: "test" })).toBe(false);
    expect(isError(null)).toBe(false);
    expect(isError(undefined)).toBe(false);
  });
});

describe("isErrorWithCode", () => {
  it("should return true for Error with code property", () => {
    const error: ErrorWithCode = new Error("test") as ErrorWithCode;
    error.code = "ENOENT";
    expect(isErrorWithCode(error)).toBe(true);
  });

  it("should return true for error-like object with code", () => {
    const error = { name: "Error", message: "test", code: "ERR_TEST" };
    expect(isErrorWithCode(error)).toBe(true);
  });

  it("should return false for errors without code", () => {
    const error = new Error("test");
    expect(isErrorWithCode(error)).toBe(false);
  });

  it("should return false for non-errors", () => {
    expect(isErrorWithCode({ code: "TEST" })).toBe(false);
  });
});

describe("isTauriError", () => {
  it("should return true for Tauri error objects", () => {
    const error: TauriError = { success: false, error: "Test error" };
    expect(isTauriError(error)).toBe(true);
  });

  it("should return false for Tauri success", () => {
    const response = { success: true, data: "test" };
    expect(isTauriError(response)).toBe(false);
  });

  it("should return false for non-objects", () => {
    expect(isTauriError(null)).toBe(false);
    expect(isTauriError(undefined)).toBe(false);
  });
});

describe("isTauriSuccess", () => {
  it("should return true for Tauri success objects", () => {
    const response: TauriSuccess<string> = { success: true, data: "test" };
    expect(isTauriSuccess(response)).toBe(true);
  });

  it("should return true for success without data", () => {
    const response = { success: true };
    expect(isTauriSuccess(response)).toBe(true);
  });

  it("should return false for Tauri error", () => {
    const error = { success: false, error: "Test error" };
    expect(isTauriSuccess(error)).toBe(false);
  });

  it("should return false for non-objects", () => {
    expect(isTauriSuccess(null)).toBe(false);
    expect(isTauriSuccess(undefined)).toBe(false);
  });
});

describe("isValidFrameIndex", () => {
  it("should return true for valid frame indices", () => {
    expect(isValidFrameIndex(0, 100)).toBe(true);
    expect(isValidFrameIndex(50, 100)).toBe(true);
    expect(isValidFrameIndex(99, 100)).toBe(true);
  });

  it("should return false for negative indices", () => {
    expect(isValidFrameIndex(-1, 100)).toBe(false);
  });

  it("should return false for indices >= maxFrames", () => {
    expect(isValidFrameIndex(100, 100)).toBe(false);
    expect(isValidFrameIndex(101, 100)).toBe(false);
  });

  it("should return false for non-numbers", () => {
    expect(isValidFrameIndex("50", 100)).toBe(false);
    expect(isValidFrameIndex(null, 100)).toBe(false);
    expect(isValidFrameIndex(undefined, 100)).toBe(false);
  });

  it("should return false for non-integers", () => {
    expect(isValidFrameIndex(1.5, 100)).toBe(false);
    expect(isValidFrameIndex(50.9, 100)).toBe(false);
  });

  it("should return false for NaN", () => {
    expect(isValidFrameIndex(NaN, 100)).toBe(false);
  });
});

describe("isValidFilePath", () => {
  it("should return true for valid paths", () => {
    expect(isValidFilePath("/path/to/file.mp4")).toBe(true);
    expect(isValidFilePath("C:\\Users\\test\\file.mp4")).toBe(true);
    expect(isValidFilePath("./relative/path.txt")).toBe(true);
    expect(isValidFilePath("../parent/file.txt")).toBe(true);
  });

  it("should return false for empty string", () => {
    expect(isValidFilePath("")).toBe(false);
  });

  it("should return false for non-strings", () => {
    expect(isValidFilePath(null)).toBe(false);
    expect(isValidFilePath(undefined)).toBe(false);
    expect(isValidFilePath(123)).toBe(false);
  });

  it("should return false for paths with invalid characters", () => {
    expect(isValidFilePath("path<with>invalid:chars")).toBe(false);
    expect(isValidFilePath("path|with|invalid|pipe")).toBe(false);
    expect(isValidFilePath("path?with?question")).toBe(false);
    expect(isValidFilePath("path*with*asterisk")).toBe(false);
  });

  it("should return false for excessively long paths", () => {
    const longPath = "a".repeat(5000);
    expect(isValidFilePath(longPath)).toBe(false);
  });
});

describe("hasProperty", () => {
  it("should return true for objects with property", () => {
    expect(hasProperty({ a: 1 }, "a")).toBe(true);
    expect(hasProperty({ name: "test" }, "name")).toBe(true);
  });

  it("should return true for inherited properties", () => {
    const obj = { a: 1 };
    expect(hasProperty(obj, "toString")).toBe(true);
  });

  it("should return false for objects without property", () => {
    expect(hasProperty({ a: 1 }, "b")).toBe(false);
    expect(hasProperty({}, "anything")).toBe(false);
  });

  it("should return false for non-objects", () => {
    expect(hasProperty(null, "a")).toBe(false);
    expect(hasProperty(undefined, "a")).toBe(false);
    expect(hasProperty("string", "length")).toBe(false);
  });
});

describe("getErrorMessage", () => {
  it("should return message from Error", () => {
    const error = new Error("Test error");
    expect(getErrorMessage(error)).toBe("Test error");
  });

  it("should return string as-is", () => {
    expect(getErrorMessage("Error message")).toBe("Error message");
  });

  it("should return error from Tauri error", () => {
    const tauriError = { success: false, error: "Tauri error" };
    expect(getErrorMessage(tauriError)).toBe("Tauri error");
  });

  it("should return default message for unknown errors", () => {
    expect(getErrorMessage(null)).toBe("Unknown error occurred");
    expect(getErrorMessage(undefined)).toBe("Unknown error occurred");
    expect(getErrorMessage(123)).toBe("Unknown error occurred");
    expect(getErrorMessage({})).toBe("Unknown error occurred");
  });
});

describe("tryParseJSON", () => {
  it("should parse valid JSON", () => {
    expect(tryParseJSON('{"a":1}')).toEqual({ a: 1 });
    expect(tryParseJSON("[1,2,3]")).toEqual([1, 2, 3]);
    expect(tryParseJSON('"string"')).toBe("string");
    expect(tryParseJSON("123")).toBe(123);
    expect(tryParseJSON("true")).toBe(true);
    expect(tryParseJSON("null")).toBe(null);
  });

  it("should return null for invalid JSON", () => {
    expect(tryParseJSON("not json")).toBe(null);
    expect(tryParseJSON("{invalid}")).toBe(null);
    expect(tryParseJSON("")).toBe(null);
  });

  it("should work with generic types", () => {
    interface TestType {
      a: number;
      b: string;
    }
    const result = tryParseJSON<TestType>('{"a":1,"b":"test"}');
    if (result) {
      expect(result.a).toBe(1);
      expect(result.b).toBe("test");
    }
  });
});

describe("assertNotNil", () => {
  it("should not throw for non-null values", () => {
    expect(() => assertNotNil(0)).not.toThrow();
    expect(() => assertNotNil(false)).not.toThrow();
    expect(() => assertNotNil("")).not.toThrow();
    expect(() => assertNotNil({})).not.toThrow();
  });

  it("should throw for null", () => {
    expect(() => assertNotNil(null)).toThrow(
      "Expected value to not be null/undefined",
    );
  });

  it("should throw for undefined", () => {
    expect(() => assertNotNil(undefined)).toThrow(
      "Expected value to not be null/undefined",
    );
  });

  it("should throw custom message", () => {
    expect(() => assertNotNil(null, "Custom error")).toThrow("Custom error");
  });

  it("should narrow type correctly", () => {
    const value: string | null = "test";
    assertNotNil(value);
    // After assertion, value should be string
    expect(value.toUpperCase()).toBe("TEST");
  });
});

describe("typeGuards edge cases", () => {
  it("should handle NaN in isNumber", () => {
    expect(isNumber(NaN)).toBe(false);
  });

  it("should handle Infinity in isNumber", () => {
    expect(isNumber(Infinity)).toBe(true);
    expect(isNumber(-Infinity)).toBe(true);
  });

  it("should handle empty objects", () => {
    expect(isObject({})).toBe(true);
    expect(isObject(Object.create(null))).toBe(true);
  });

  it("should handle empty arrays", () => {
    expect(isArray([])).toBe(true);
  });

  it("should handle array-like objects", () => {
    const arrayLike = { "0": "a", "1": "b", length: 2 };
    expect(isObject(arrayLike)).toBe(true);
    expect(isArray(arrayLike)).toBe(false);
  });
});
