import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { execSync } from "node:child_process";

export function unlink(args) {
  const packageName = args[0];
  const globalNodeModules = execSync("npm root -g").toString().trim();
  const linkPath = join(globalNodeModules, packageName);

  if (!existsSync(linkPath)) {
    console.error(`Package ${packageName} is not linked globally.`);
    process.exit(1);
  }

  try {
    execSync(`npm unlink ${packageName}`);
    console.log(`Successfully unlinked ${packageName} from global node_modules.`);
  } catch (error) {
    console.error(`Failed to unlink ${packageName}: ${error.message}`);
    process.exit(1);
  }

  const currentNodeModules = join(process.cwd(), "node_modules");
  const currentLinkPath = join(currentNodeModules, packageName);

  if (!existsSync(currentNodeModules)) {
    console.error(`node_modules directory does not exist in the current directory.`);
    process.exit(1);
  }

  if (!existsSync(currentLinkPath)) {
    console.error(`Package ${packageName} is not linked locally.`);
    process.exit(1);
  }

  try {
    execSync(`npm unlink ${packageName}`);
    console.log(`Successfully unlinked ${packageName} from local node_modules.`);
  } catch (error) {
    console.error(`Failed to unlink ${packageName} from local node_modules: ${error.message}`);
    process.exit(1);
  }
}
