import fetch from "node-fetch";
import { retryOnECONNRESET } from "./retry.js";
import chalk from "chalk";
import logger from "../lib/logger.js";

export async function fetchPackageMetadata(
  packageNames,
  spinner,
  currentPackageIndex,
  totalPackages,
  isForce,
) {
  if (!Array.isArray(packageNames)) {
    logger.logError({
      message: "packageNames should be an array",
      exit: true,
      errorType: " PACM_ERROR ",
    });
  }

  spinner.text = `${isForce ? chalk.bgYellow("FORCE") : ""} [${currentPackageIndex}/${totalPackages}] Fetching metadata for packages: ${packageNames.join(", ")}`;
  const fetchMetadata = async (packageName) => {
    const response = await fetch(`https://registry.npmjs.org/${packageName}`);
    if (!response.ok) {
      throw new Error(`Failed to fetch metadata for package ${packageName}`);
    }
    return response.json();
  };

  const metadataPromises = packageNames.map((packageName) =>
    retryOnECONNRESET(fetchMetadata, packageName),
  );

  return Promise.all(metadataPromises);
}
