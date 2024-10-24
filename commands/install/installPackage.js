/* eslint-disable no-unused-vars */
import { existsSync, mkdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import semver from "semver";
import { fetchPackageMetadata } from "../../utils/fetchPackageMetadata.js";
import { downloadAndExtractTarball } from "../../utils/downloadAndExtractTarball.js";
import { homedir } from "node:os";
import process from "node:process";
import chalk from "chalk";

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
    throw new Error(
      "[ERRNO1] Invalid packageName or version. Both must be strings.",
    );
  }

  if (!packageName || !version) {
    throw new Error(
      "[ERRNO2] Invalid packageName or version. Both must be defined.",
    );
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
    throw new Error("GitHub packages are not supported yet");
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
    : join(installDir, "node_modules", ".pacm", packageName);
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

  const dependencies = metadata.versions[maxSatisfyingVersion].dependencies || {};

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
      false,
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
