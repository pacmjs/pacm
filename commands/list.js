import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import process from "node:process";

export function list() {
  const installDir = process.cwd();
  const packageJsonPath = join(installDir, "package.json");

  if (!existsSync(packageJsonPath)) {
    console.error("No package.json found in the current directory.");
    process.exit(1);
  }

  const packageJson = JSON.parse(readFileSync(packageJsonPath, "utf-8"));
  const dependencies = packageJson.dependencies || {};
  const devDependencies = packageJson.devDependencies || {};

  console.log("Installed packages:");
  for (const [name, version] of Object.entries(dependencies)) {
    console.log(`- ${name}@${version}`);
  }

  console.log("\nDev dependencies:");
  for (const [name, version] of Object.entries(devDependencies)) {
    console.log(`- ${name}@${version}`);
  }
}
