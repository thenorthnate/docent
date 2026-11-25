import { describe, expect, it } from "vitest";
import { fileTreeRows } from "./fileTree";
import type { ChangedFile } from "./review";

function rows(...paths: string[]) {
  const files = paths.map((path): ChangedFile => ({
    path,
    old_path: null,
    status: "modified",
    lines: null,
    old_blob: null,
    new_blob: null,
  }));
  return fileTreeRows(files).map((row) => `${"  ".repeat(row.depth)}${row.name}${row.file ? "" : "/"}`);
}

describe("fileTreeRows", () => {
  it("nests files under their folders, folders first, each in order", () => {
    expect(rows("b.txt", "src/z.ts", "a.txt", "src/lib/y.ts", "docs/x.md")).toEqual([
      "docs/",
      "  x.md",
      "src/",
      "  lib/",
      "    y.ts",
      "  z.ts",
      "a.txt",
      "b.txt",
    ]);
  });

  it("joins folders that hold only one other folder", () => {
    expect(rows("a/b/c/one.ts", "a/b/d/two.ts")).toEqual(["a/b/", "  c/", "    one.ts", "  d/", "    two.ts"]);
  });

  it("treats names from the PR as plain text", () => {
    expect(rows("__proto__/constructor")).toEqual(["__proto__/", "  constructor"]);
  });

  it("gives look-alike files their own keys", () => {
    const file = (new_blob: string): ChangedFile => ({
      path: "src/bad\u{fffd}.txt",
      old_path: null,
      status: "added",
      lines: null,
      old_blob: null,
      new_blob,
    });

    const keys = fileTreeRows([file("1".repeat(40)), file("2".repeat(40))]).map((row) => row.key);

    expect(new Set(keys).size).toBe(3);
  });
});
