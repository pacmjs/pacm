import { existsSync, rmSync } from "node:fs";
import { join } from "node:path";
import { homedir } from "node:os";
import logger from "../lib/logger.js";
import process from "node:process";

export function clean() {
  const cacheDir = join(homedir(), ".pacm-cache");

  if (existsSync(cacheDir)) {
    rmSync(cacheDir, { recursive: true, force: true });
    logger.logSuccess({
      message: "Cache cleared successfully.",
      successType: " PACM_CACHE_CLIENT ",
    });
    process.exit(0);
  } else {
    logger.logError({
      message: "Cache is already empty.",
      errorType: " PACM_CACHE_CLIENT ",
    });
    process.exit(1);
  }
}
