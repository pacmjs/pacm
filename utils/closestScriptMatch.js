import { readFileSync } from "node:fs";
import { join } from "node:path";
import logger from "../lib/logger.js";
import process from "node:process";

export default async function closestScriptMatch(args) {
  const scriptName = args[0];
  if (!scriptName)
    logger.logError({
      message: "No script name provided.",
      exit: true,
      errorType: " PACM_CLOSEST_MATCH_ERROR ",
    });

  const packageJsonPath = join(process.cwd(), "package.json");
  let packageJson;

  try {
    packageJson = JSON.parse(readFileSync(packageJsonPath, "utf-8"));
  } catch (error) {
    logger.logError({
      message: `Failed to read package.json: ${error}`,
      exit: true,
      errorType: " PACM_CLOSEST_MATCH_ERROR ",
    });
  }

  const scripts = packageJson.scripts || {};
  const closestMatch = Object.keys(scripts).find((key) =>
    key.includes(scriptName),
  );

  return closestMatch;
}
