import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import ora from "ora";
import { fetchPackageMetadata } from "../utils/fetchPackageMetadata.js";
import { installPackage } from "./install/installPackage.js";
import { createLockFile } from "../utils/createLockFile.js";
import chalk from "chalk";
import process from "node:process";
import { fetchAllDependencies } from "./update/fetchAllDependencies.js";
import logger from "../lib/logger.js";

export async function update(args) {
  const packages = [];
  const flags = [];
  const alreadyUpdatedPackages = [];

  args.forEach((arg) => {
    if (arg.startsWith("-")) {
      flags.push(arg);
    } else {
      packages.push(arg);
    }
  });

  const installDir = process.cwd();
  const packageJsonPath = join(installDir, "package.json");
  const lockFilePath = join(installDir, "pacm.lockp");
  let packageJson = {};
  let lockFileData = { dependencies: {}, devDependencies: {} };
  let notInstalledPackages = [];

  if (existsSync(packageJsonPath)) {
    packageJson = JSON.parse(readFileSync(packageJsonPath, "utf-8"));
  } else {
    packageJson = { dependencies: {}, devDependencies: {} };
  }

  if (!packageJson.dependencies) {
    packageJson.dependencies = {};
  }

  if (!packageJson.devDependencies) {
    packageJson.devDependencies = {};
  }

  if (existsSync(lockFilePath)) {
    lockFileData = JSON.parse(readFileSync(lockFilePath, "utf-8"));
  }

  const spinner = ora("[0/0] Fetching package information").start();
  const postInstallScripts = [];

  try {
    if (packages.length === 0) {
      if (existsSync(lockFilePath)) {
        packages.push(...Object.keys(lockFileData.dependencies));
        packages.push(...Object.keys(lockFileData.devDependencies));
      } else if (existsSync(packageJsonPath)) {
        packages.push(...Object.keys(packageJson.dependencies));
        packages.push(...Object.keys(packageJson.devDependencies));
      } else {
        throw new Error("No packages to update.");
      }
    }

    const isDevDependency = flags.includes("--dev") || flags.includes("-D");
    const isForce = flags.includes("--force") || flags.includes("-f");

    const packageInfoList = [];
    for (const pkg of packages) {
      let packageName, version;

      if (!packageJson.dependencies[pkg] && !packageJson.devDependencies[pkg]) {
        notInstalledPackages.push(pkg);
        continue;
      }

      if (pkg.startsWith("@")) {
        const atIndex = pkg.indexOf("@", 1);
        if (atIndex === -1) {
          packageName = pkg;
          version = "latest";
        } else {
          packageName = pkg.substring(0, atIndex);
          version = pkg.substring(atIndex + 1) || "latest";
        }
      } else {
        [packageName, version] = pkg.split("@");
        version = version || "latest";
      }

      const packageInfo = await fetchPackageMetadata(
        packageName,
        spinner,
        packageInfoList.length + 1,
        packages.length,
      );

      if (version === "latest") {
        version = packageInfo["dist-tags"].latest;
      }

      packageInfoList.push({ ...packageInfo, version });

      if (packageInfo.dependencies) {
        for (const depName in packageInfo.dependencies) {
          await fetchAllDependencies(depName, spinner, packageInfoList, packages, installDir);
        }
      }
    }

    const totalPackages = packageInfoList.length;
    let currentPackageIndex = 0;

    const startTime = Date.now();

    for (const pkgInfo of packageInfoList) {
      const { name: packageName, version } = pkgInfo;

      if (notInstalledPackages.includes(packageName)) {
        spinner.text = `[${currentPackageIndex}/${totalPackages}] Package not installed: ${packageName}, skipping.`;
        continue;
      }

      if (!isForce) {
        const nodeModulesDir = join(installDir, "node_modules", packageName);
        if (existsSync(nodeModulesDir)) {
          const packageJsonPath = join(nodeModulesDir, "package.json");
          const packageJson = JSON.parse(
            readFileSync(packageJsonPath, "utf-8"),
          );
          const installedVersion = packageJson.version;

          if (installedVersion === version) {
            alreadyUpdatedPackages.push(packageName);
            spinner.text = `[${currentPackageIndex}/${totalPackages}] Package already up-to-date: ${packageName}, version: ${version}, skipping.`;
            continue;
          }
        }
      }

      currentPackageIndex++;
      spinner.text = `${isForce ? chalk.bgYellow("FORCE") : ""} [${currentPackageIndex}/${totalPackages}] Updating package: ${packageName}, version: ${version}`;
      const updatedPackage = await installPackage(
        spinner,
        packageName,
        version,
        installDir,
        postInstallScripts,
        lockFileData,
        isDevDependency,
        currentPackageIndex,
        totalPackages,
        isForce,
      );
      spinner.text = `${isForce ? chalk.bgYellow("FORCE") : ""} [${currentPackageIndex}/${totalPackages}] Updated package: ${updatedPackage.packageName}, version: ${updatedPackage.version}`;

      if (isDevDependency) {
        packageJson.devDependencies[updatedPackage.packageName] =
          updatedPackage.version;
        lockFileData.devDependencies[updatedPackage.packageName] = {
          version: updatedPackage.version,
          resolved: updatedPackage.resolved,
          integrity: updatedPackage.integrity,
          dependencies: updatedPackage.dependencies,
        };
      } else {
        packageJson.dependencies[updatedPackage.packageName] =
          updatedPackage.version;
        lockFileData.dependencies[updatedPackage.packageName] = {
          version: updatedPackage.version,
          resolved: updatedPackage.resolved,
          integrity: updatedPackage.integrity,
          dependencies: updatedPackage.dependencies,
        };
      }
    }

    spinner.text = "Writing package.json";
    writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, 2));

    createLockFile(lockFileData, lockFilePath);

    const endTime = Date.now();
    const duration = endTime - startTime;
    const durationText =
      duration < 1000
        ? `${duration} ms`
        : `${(duration / 1000).toFixed(2)} seconds`;

    spinner.succeed(`Packages updated successfully in ${durationText}.`);
    if (alreadyUpdatedPackages.length > 0)
      logger.logWarning({
        message: `Packages already up-to-date: ${alreadyUpdatedPackages.join(", ")}`,
        warningType: " PACM_UPDATE_ALREADY_UP_TO_DATE ",
      });
    if (notInstalledPackages.length > 0)
      logger.logWarning({
        message: `Packages not installed: ${notInstalledPackages.join(", ")}`,
        warningType: " PACM_UPDATE_NOT_INSTALLED ",
      });
  } catch (error) {
    spinner.fail(`Update failed: ${error.message}`);
    console.error(error);
  }
}
