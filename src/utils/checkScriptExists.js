import { join } from "node:path";
import { readFileSync } from "node:fs";

export default function checkScriptExists(args) {
  const scriptName = args[0];
  if (!scriptName) {
    return "No script name provided.";
  }

  const packageJsonPath = join(process.cwd(), "package.json");
  let packageJson;

  try {
    packageJson = JSON.parse(readFileSync(packageJsonPath, "utf-8"));
  } catch (error) {
    return `Failed to read package.json: ${error}`;
  }

  const scripts = packageJson.scripts || {};
  const script = scripts[scriptName];

  if (!script) {
    const cloestMatch = Object.keys(scripts).find((key) =>
      key.includes(scriptName),
    );

    return `No script named "${scriptName}" found in package.json.`;
  }

  return true;
}
