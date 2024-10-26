/* eslint-disable no-unused-vars */
import { existsSync, mkdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import semver from "semver";
import { fetchPackageMetadata } from "./fetchPackageMetadata.js";
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
  isRootPackage = false,
) {
  if (typeof packageName !== "string" || typeof version !== "string") {
    logger.logError({
      message: "Invalid package name or version. Both must be strings.",
      exit: true,
      errorType: " PACM_INVALID_PACKAGE_NAME_OR_VERSION ",
    });
  }

  if (!packageName || !version) {
    logger.logError({
      message: "Package name and version are required.",
      exit: true,
      errorType: " PACM_PACKAGE_NAME_AND_VERSION_REQUIRED ",
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
      errorType: " PACM_GITHUB_PACKAGES_NOT_SUPPORTED ",
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
      throw new Error(
        `Version ${versionToInstall} of package ${packageName} not found`,
      );
    }
  } else {
    maxSatisfyingVersion = metadata["dist-tags"].latest;
  }

  const packageVersion = metadata.versions[maxSatisfyingVersion];
  const tarballUrl = packageVersion.dist.tarball;
  const packageDir = isRootPackage
    ? join(installDir, "node_modules", packageName)
    : join(installDir, "node_modules", packageName);
  const cachePath = join(
    globalCacheDir,
    packageName.startsWith("@") ? packageName.replace("/", "_") : packageName,
    `${maxSatisfyingVersion}.tgz`,
  );

  if (existsSync(packageDir)) {
    const packageJsonPath = join(packageDir, "package.json");
    if (existsSync(packageJsonPath)) {
      const packageJson = readFileSync(packageJsonPath, "utf-8");
      try {
        const parsedPackageJson = JSON.parse(packageJson);
        const installedVersion = parsedPackageJson.version;

        if (installedVersion !== maxSatisfyingVersion) {
          await downloadAndExtractTarball(
            tarballUrl,
            packageDir,
            cachePath,
            spinner,
            isForce,
            currentPackageIndex,
            totalPackages,
          );
        }
      } catch (error) {
        logger.logError({
          message: "Error parsing package.json",
          exit: true,
          errorType: " PACM_ERROR_PARSING_PACKAGE_JSON ",
        });
      }
    } else {
      await downloadAndExtractTarball(
        tarballUrl,
        packageDir,
        cachePath,
        spinner,
        isForce,
        currentPackageIndex,
        totalPackages,
      );
    }
  } else {
    mkdirSync(packageDir, { recursive: true });
    await downloadAndExtractTarball(
      tarballUrl,
      packageDir,
      cachePath,
      spinner,
      isForce,
      currentPackageIndex,
      totalPackages,
    );
  }

  const dependencies =
    metadata.versions[maxSatisfyingVersion].dependencies || {};

  const dependencyPromises = Object.entries(dependencies).map(
    async ([depName, depVersion]) => {
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
        false,
      );
      if (currentPackageIndex < totalPackages) {
        currentPackageIndex++;
      }
    },
  );

  await Promise.all(dependencyPromises);

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