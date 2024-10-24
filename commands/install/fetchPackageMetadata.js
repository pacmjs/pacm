import fetch from "node-fetch";
import logger from "../../lib/logger.js";

export async function fetchPackageMetadata(packageName, spinner, currentPackageIndex, totalPackages, isForce = false) {
  const url = `https://registry.npmjs.org/${packageName}`;
  spinner.text = `${isForce ? chalk.bgYellow("FORCE") : ""} [${currentPackageIndex}/${totalPackages}] Fetching metadata for ${packageName}`;

  try {
    const response = await fetch(url);
    if (!response.ok) {
      throw new Error(`Failed to fetch metadata for ${packageName}`);
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
