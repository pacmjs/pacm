import { existsSync, rmSync } from "node:fs";
import { join } from "node:path";
import { homedir } from "node:os";

export function clean() {
  const cacheDir = join(homedir(), ".pacm-cache");

  if (existsSync(cacheDir)) {
    rmSync(cacheDir, { recursive: true, force: true });
    console.log("Cache cleared successfully.");
  } else {
    console.log("No cache found to clear.");
  }
}
