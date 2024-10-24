import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { execSync } from "node:child_process";
import process from "node:process";

export function link(args) {
  const packageName = args[0];
  const packagePath = join(process.cwd(), packageName);

  if (!existsSync(packagePath)) {
    console.error(
      `Package ${packageName} does not exist in the current directory.`,
    );
    process.exit(1);
  }

  const packageJsonPath = join(packagePath, "package.json");
  if (!existsSync(packageJsonPath)) {
    console.error(`package.json not found in ${packageName}`);
    process.exit(1);
  }

  const packageJson = JSON.parse(readFileSync(packageJsonPath, "utf-8"));

  try {
    execSync(`npm link ${packagePath}`);
    console.log(
      `Successfully linked ${packageJson.name} to global node_modules.`,
    );
  } catch (error) {
    console.error(`Failed to link ${packageJson.name}: ${error.message}`);
    process.exit(1);
  }

  const currentNodeModules = join(process.cwd(), "node_modules");

  if (!existsSync(currentNodeModules)) {
    console.error(
      `node_modules directory does not exist in the current directory.`,
    );
    process.exit(1);
  }

  try {
    execSync(`npm link ${packageJson.name}`);
    console.log(
      `Successfully linked ${packageJson.name} to local node_modules.`,
    );
  } catch (error) {
    console.error(
      `Failed to link ${packageJson.name} to local node_modules: ${error.message}`,
    );
    process.exit(1);
  }
}
