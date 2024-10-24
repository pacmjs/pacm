import { exec } from "child_process";
import checkScriptExists from "../utils/checkScriptExists.js";
import closestScriptMatch from "../utils/closestScriptMatch.js";
import { join } from "node:path";
import { readFileSync } from "node:fs";
import logger from "../lib/logger.js";

export async function run(args) {
  const exists = await checkScriptExists(args);

  if (exists !== true) {
    const closestMatch = await closestScriptMatch(args);

    logger.logError({
      message: exists,
      exit: false,
      errorType: " PACM_RUNTIME_ERROR ",
    });
    if (closestMatch) {
      console.log(
        `\n\nDid you mean "${closestMatch}"? Run "pacm run ${closestMatch}" to execute.`,
      );
    }
    process.exit(1);
  }

  const packageJsonPath = join(process.cwd(), "package.json");
  let packageJson;

  try {
    packageJson = JSON.parse(readFileSync(packageJsonPath, "utf-8"));
  } catch (error) {
    logger.logError({
      message: `Failed to read package.json: ${error}`,
      exit: true,
      errorType: " PACM_RUNTIME_ERROR ",
    });
  }

  const scripts = packageJson.scripts || {};
  const script = scripts[args[0]];

  exec(script, (error, stdout, stderr) => {
    if (error)
      logger.logError({
        message: error,
        exit: true,
        errorType: " PACM_RUNTIME_ERROR ",
      });
    console.log(stdout);
    console.error(stderr);
  });
}
