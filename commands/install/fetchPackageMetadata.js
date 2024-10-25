import fetch from "node-fetch";
import logger from "../../lib/logger.js";
import chalk from "chalk";

export async function fetchPackageMetadata(packageName, spinner, currentPackageIndex, totalPackages, isForce = false) {
  const url = `https://registry.npmjs.org/${packageName}`;
  spinner.text = `${isForce ? chalk.bgYellow("FORCE") : ""} [${currentPackageIndex}/${totalPackages}] Fetching metadata for ${packageName}`;

  try {
    const response = await fetch(url);
    if (!response.ok) {
      spinner.stop();
      logger.logError({
        message: response.statusText,
        exit: true,
        errorType: " PACM_FETCH_METADATA_ERROR ",
      });
    }
    const metadata = await response.json();
    return metadata;
  } catch (error) {
    spinner.fail(`Failed to fetch metadata for ${packageName}: ${error.message}`);
    logger.logError({
      message: error.message,
      exit: true,
      errorType: " PACM_FETCH_METADATA_ERROR ",
    });
  }
}
