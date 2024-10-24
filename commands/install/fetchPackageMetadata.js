import fetch from "node-fetch";
import logger from "../../lib/logger.js";
import chalk from "chalk";

export async function fetchPackageMetadata(packageNames, spinner, currentPackageIndex, totalPackages, isForce = false) {
  spinner.text = `${isForce ? chalk.bgYellow("FORCE") : ""} [${currentPackageIndex}/${totalPackages}] Fetching metadata for packages: ${packageNames.join(", ")}`;

  try {
    const metadataPromises = packageNames.map(async (packageName) => {
      const url = `https://registry.npmjs.org/${packageName}`;
      const response = await fetch(url);
      if (!response.ok) {
        throw new Error(`Failed to fetch metadata for ${packageName}`);
      }
      return response.json();
    });

    const metadataList = await Promise.all(metadataPromises);
    return metadataList;
  } catch (error) {
    spinner.fail(`Failed to fetch metadata for packages: ${error.message}`);
    logger.logError({
      message: error.message,
      exit: true,
      errorType: " PACM_FETCH_METADATA_ERROR ",
    });
  }
}
