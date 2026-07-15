import { execFileSync } from "node:child_process";
import { readdirSync } from "node:fs";
import path from "node:path";
import test from "node:test";

const extensionRoot = path.resolve("extension");

function javascriptFiles(directory) {
  return readdirSync(directory, { withFileTypes: true })
    .flatMap((entry) => {
      const entryPath = path.join(directory, entry.name);
      if (entry.isDirectory()) return javascriptFiles(entryPath);
      return entry.isFile() && entry.name.endsWith(".js") ? [entryPath] : [];
    })
    .sort();
}

test("every extension JavaScript file parses as a whole", () => {
  for (const file of javascriptFiles(extensionRoot)) {
    execFileSync(process.execPath, ["--check", file], { stdio: "pipe" });
  }
});
