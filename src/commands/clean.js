import { existsSync, rmSync } from "node:fs";
import { join } from "node:path";

export function clean() {
  const cacheDir = join(process.cwd(), "node_modules", ".cache");

  if (existsSync(cacheDir)) {
    rmSync(cacheDir, { recursive: true, force: true });
    console.log("Cache cleared successfully.");
  } else {
    console.log("No cache found to clear.");
  }
}
