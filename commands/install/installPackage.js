/* eslint-disable no-unused-vars */
import { existsSync, mkdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import semver from "semver";
import { fetchPackageMetadata } from "../../utils/fetchPackageMetadata.js";
import { downloadAndExtractTarball } from "../../utils/downloadAndExtractTarball.js";
import { homedir } from "node:os";
import process from "node:process";
import chalk from "chalk";
import logger from "../../lib/logger.js";

const globalCacheDir = join(homedir(), ".pacm-cache");

if (!existsSync(globalCacheDir)) {
  mkdirSync(globalCacheDir);
}

export async function installPackage(
  spinner,
  packageName,
  version,
  installDir = process.cwd(),
  postInstallScripts = [],
  lockFileData = { dependencies: {}, devDependencies: {} },
  isDevDependency = false,
  currentPackageIndex = 0,
  totalPackages = 0,
  isForce = false,
) {
  if (typeof packageName !== "string" || typeof version !== "string") {
    logger.logError({
      message: "Invalid packageName or version. Both must be strings.",
      exit: true,
      errorType: "PACM_ERROR",
    });
  }

  if (!packageName || !version) {
    logger.logError({
      message: "[ERRNO2] Invalid packageName or version. Both must be defined.",
      exit: true,
      errorType: "PACM_ERROR",
    });
  }

  let metadata;
  let versionToInstall = version;

  if (version && version.startsWith("npm:")) {
    const [npmPackage, npmVersion] = version.slice(4).split("@");
    metadata = await fetchPackageMetadata(
      npmPackage,
      spinner,
      currentPackageIndex,
      totalPackages,
    );
    versionToInstall = npmVersion || metadata["dist-tags"].latest;
  } else if (version && version.startsWith("github:")) {
    logger.logError({
      message: "GitHub packages are not supported yet.",
      exit: true,
      errorType: "PACM_GITHUB_EXTENSION_ERROR",
    });
  } else {
    metadata = await fetchPackageMetadata(
      packageName,
      spinner,
      currentPackageIndex,
      totalPackages,
      isForce,
    );
    versionToInstall = version || metadata["dist-tags"].latest;
  }

  spinner.text = `${isForce ? chalk.bgYellow("FORCE") : ""} [${currentPackageIndex}/${totalPackages}] Validating version for ${packageName}`;
  const availableVersions = Object.keys(metadata.versions);
  let maxSatisfyingVersion;

  if (versionToInstall !== "latest") {
    maxSatisfyingVersion = semver.maxSatisfying(
      availableVersions,
      versionToInstall,
    );

    if (!maxSatisfyingVersion) {
      logger.logError({
        message: `No version found for ${packageName}@${versionToInstall}`,
        exit: true,
        errorType: "PACM_VERSION_ERROR",
      });
    }
  } else {
    maxSatisfyingVersion = metadata["dist-tags"].latest;
  }

  const packageVersion = metadata.versions[maxSatisfyingVersion];
  const tarballUrl = packageVersion.dist.tarball;
  const packageDir = join(installDir, "node_modules", packageName);
  const cachePath = join(
    globalCacheDir,
    packageName.startsWith("@") ? packageName.replace("/", "_") : packageName,
    `${maxSatisfyingVersion}.tgz`,
  );

  if (!existsSync(packageDir)) {
    mkdirSync(packageDir, { recursive: true });
    await downloadAndExtractTarball(
      tarballUrl,
      packageDir,
      cachePath,
      spinner,
      currentPackageIndex,
      totalPackages,
      isForce,
    );
  }

  const dependencies =
    metadata.versions[maxSatisfyingVersion].dependencies || {};

  for (const [depName, depVersion] of Object.entries(dependencies)) {
    await installPackage(
      spinner,
      depName,
      depVersion,
      installDir,
      postInstallScripts,
      lockFileData,
      isDevDependency,
      currentPackageIndex,
      totalPackages,
      isForce,
    );
    if (currentPackageIndex < totalPackages) {
      currentPackageIndex++;
    }
  }

  postInstallScripts.push(packageDir);

  if (isDevDependency) {
    lockFileData.devDependencies[packageName] = {
      version: maxSatisfyingVersion,
      resolved: tarballUrl,
      integrity: packageVersion.dist.integrity,
      dependencies:
        Object.keys(dependencies).length > 0 ? dependencies : undefined,
    };
  } else {
    lockFileData.dependencies[packageName] = {
      version: maxSatisfyingVersion,
      resolved: tarballUrl,
      integrity: packageVersion.dist.integrity,
      dependencies:
        Object.keys(dependencies).length > 0 ? dependencies : undefined,
    };
  }

  return {
    packageName,
    version: maxSatisfyingVersion,
    resolved: tarballUrl,
    integrity: packageVersion.dist.integrity,
    dependencies,
  };
}
