import fetch from "node-fetch";
import { retryOnECONNRESET } from "./retry.js";
import chalk from "chalk";
import { fetchNpmConfig } from "./fetchNpmConfig.js";

export async function fetchPackageMetadata(
  packageName,
  spinner,
  currentPackageIndex,
  totalPackages,
  isForce,
) {
  const { registry } = fetchNpmConfig();
  spinner.text = `${isForce ? chalk.bgYellow("FORCE") : ""} [${currentPackageIndex}/${totalPackages}] Fetching metadata for ${packageName}`;
  return retryOnECONNRESET(async (packageName) => {
    const response = await fetch(`${registry}/${packageName}`);
    if (!response.ok) {
      throw new Error(`Failed to fetch metadata for package ${packageName}`);
    }
    return response.json();
  }, packageName);
}
