import { describe, it, expect, vi } from "vitest";
import {
  MAX_SAVE_BYTES,
  sanitizeDefaultName,
  saveTextFile,
  validateSaveRequest,
} from "../electron/saveTextFile.js";

const request = {
  defaultName: "frames.csv",
  filterName: "CSV",
  extensions: ["csv"],
  content: "a,b\n1,2",
};

function deps(chosen: string | null) {
  return {
    chooseSavePath: vi.fn(async () => chosen),
    writeFile: vi.fn(async () => {}),
  };
}

describe("saveTextFile (#605)", () => {
  it("writes the content to the path the dialog returned, and only that path", async () => {
    const d = deps("/Users/me/out.csv");
    const written = await saveTextFile(request, d);
    expect(written).toBe("/Users/me/out.csv");
    expect(d.writeFile).toHaveBeenCalledTimes(1);
    expect(d.writeFile).toHaveBeenCalledWith(
      "/Users/me/out.csv",
      request.content,
    );
  });

  it("shows the dialog before anything is written, with the filter it was given", async () => {
    const order: string[] = [];
    await saveTextFile(request, {
      chooseSavePath: async (options) => {
        order.push("dialog");
        expect(options).toEqual({
          defaultName: "frames.csv",
          filters: [{ name: "CSV", extensions: ["csv"] }],
        });
        return "/tmp/x.csv";
      },
      writeFile: async () => {
        order.push("write");
      },
    });
    expect(order).toEqual(["dialog", "write"]);
  });

  it("writes nothing and returns null when the user cancels", async () => {
    const d = deps(null);
    expect(await saveTextFile(request, d)).toBeNull();
    expect(d.writeFile).not.toHaveBeenCalled();
    const empty = deps("");
    expect(await saveTextFile(request, empty)).toBeNull();
    expect(empty.writeFile).not.toHaveBeenCalled();
  });

  it("ignores a path smuggled into the request: only the dialog decides where to write", async () => {
    const d = deps("/chosen/by/user.csv");
    await saveTextFile(
      {
        ...request,
        path: "/etc/passwd",
        outputPath: "/etc/passwd",
        filePath: "/etc/passwd",
      },
      d,
    );
    expect(d.writeFile).toHaveBeenCalledWith(
      "/chosen/by/user.csv",
      request.content,
    );
  });

  it("rejects malformed requests without opening a dialog", async () => {
    const bad: unknown[] = [
      null,
      "text",
      { ...request, defaultName: 5 },
      { ...request, filterName: 5 },
      { ...request, filterName: "x".repeat(41) },
      { ...request, extensions: [] },
      { ...request, extensions: ["c/sv"] },
      { ...request, extensions: ["*"] },
      { ...request, extensions: ["a", "b", "c", "d", "e", "f", "g", "h", "i"] },
      { ...request, content: 5 },
      { ...request, content: "x".repeat(MAX_SAVE_BYTES + 1) },
    ];
    for (const b of bad) {
      const d = deps("/tmp/x");
      await expect(saveTextFile(b, d)).rejects.toThrow(/saveTextFile/);
      expect(d.chooseSavePath).not.toHaveBeenCalled();
      expect(d.writeFile).not.toHaveBeenCalled();
    }
  });

  it("accepts content right at the size limit", () => {
    expect(() =>
      validateSaveRequest({ ...request, content: "x".repeat(MAX_SAVE_BYTES) }),
    ).not.toThrow();
  });

  it("counts bytes, not characters, for the size limit", () => {
    // 3 bytes per character in UTF-8
    const content = "한".repeat(Math.floor(MAX_SAVE_BYTES / 3) + 1);
    expect(() => validateSaveRequest({ ...request, content })).toThrow(
      /larger/,
    );
  });
});

describe("sanitizeDefaultName", () => {
  it("drops directory parts so the suggestion cannot leave the dialog's folder", () => {
    expect(sanitizeDefaultName("../../etc/passwd")).toBe("passwd");
    expect(sanitizeDefaultName("C:\\Windows\\win.ini")).toBe("win.ini");
    expect(sanitizeDefaultName("/abs/path/report.txt")).toBe("report.txt");
  });

  it("drops control and Windows-forbidden characters", () => {
    expect(sanitizeDefaultName('a<b>c:d"e|f?g*h\u0000i\n.csv')).toBe(
      "abcdefghi.csv",
    );
  });

  it("falls back to a fixed name when nothing usable is left", () => {
    for (const n of ["", "   ", ".", "..", "/", "<>", "dir/"]) {
      expect(sanitizeDefaultName(n)).toBe("export.txt");
    }
  });

  it("limits the length", () => {
    expect(sanitizeDefaultName("a".repeat(500)).length).toBe(120);
  });
});
