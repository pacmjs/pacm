import { exec } from "child_process";
import { readFileSync } from "fs";
import { join } from "path";

export function run(args) {
  const scriptName = args[0];
  if (!scriptName) {
    console.error("No script name provided.");
    process.exit(1);
  }

  const packageJsonPath = join(process.cwd(), "package.json");
  let packageJson;

  try {
    packageJson = JSON.parse(readFileSync(packageJsonPath, "utf-8"));
  } catch (error) {
    console.error("Failed to read package.json:", error);
    process.exit(1);
  }

  const scripts = packageJson.scripts || {};
  const script = scripts[scriptName];

  if (!script) {
    console.error(`No script named "${scriptName}" found in package.json.`);
    process.exit(1);
  }

  exec(script, (error, stdout, stderr) => {
    if (error) {
      console.error(`Error executing script "${scriptName}":`, error);
      process.exit(1);
    }
    console.log(stdout);
    console.error(stderr);
  });
}
