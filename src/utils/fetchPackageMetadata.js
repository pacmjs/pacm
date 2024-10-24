import fetch from "node-fetch";
import { retryOnECONNRESET } from "./retry.js";
import chalk from "chalk";

export async function fetchPackageMetadata(
  packageName,
  spinner,
  currentPackageIndex,
  totalPackages,
  isForce,
) {
  spinner.text = `${isForce ? chalk.bgYellow("FORCE") : ""} [${currentPackageIndex}/${totalPackages}] Fetching metadata for ${packageName}`;
  return retryOnECONNRESET(async (packageName) => {
    const response = await fetch(`https://registry.npmjs.org/${packageName}`);
    if (!response.ok) {
      throw new Error(`Failed to fetch metadata for package ${packageName}`);
    }
    return response.json();
  }, packageName);
}
