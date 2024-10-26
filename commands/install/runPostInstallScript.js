import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { exec } from "node:child_process";
import logger from "../../lib/logger.js";

export async function runPostInstallScript(packageDir, spinner) {
  const packageJsonPath = join(packageDir, "package.json");
  if (existsSync(packageJsonPath)) {
    const packageJson = JSON.parse(readFileSync(packageJsonPath, "utf-8"));
    if (packageJson.scripts && packageJson.scripts.postinstall) {
      spinner.text = `Running postinstall script for ${packageJson.name}`;
      await new Promise((resolve, reject) => {
        exec(
          "npm run postinstall",
          { cwd: packageDir },
          (error) => {
            if (error) {
              logger.logError({
                message: `Failed to run postinstall script for ${packageJson.name}`,
                exit: false,
                errorType: " PACM_POSTINSTALL_SCRIPT_FAILED ",
              });
              reject(error);
            } else {
              logger.logSuccess({
                message: `Successfully ran postinstall script for ${packageJson.name}`,
                successType: " PACM_POSTINSTALL_SCRIPT_SUCCESS ",
              });
              resolve();
            }
          },
        );
      });
    }
  }
}
