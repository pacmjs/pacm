import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import ora from "ora";
import { fetchPackageMetadata } from "../utils/fetchPackageMetadata.js";
import { installPackage } from "./install/installPackage.js";
import { createLockFile } from "../utils/createLockFile.js";
import chalk from "chalk";

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

    const isDevDependency = flags.includes("--save-dev") || flags.includes("-D");
    const isForce = flags.includes("--force") || flags.includes("-f");

    const packageInfoList = [];
    for (const pkg of packages) {
      let packageName, version;

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
        packages.length
      );

      if (version === "latest") {
        version = packageInfo["dist-tags"].latest;
      }

      packageInfoList.push({ ...packageInfo, version });
    }

    const totalPackages = packageInfoList.length;
    let currentPackageIndex = 0;

    const startTime = Date.now();

    for (const pkgInfo of packageInfoList) {
      const { name: packageName, version } = pkgInfo;

      if (!isForce) {
        const nodeModulesDir = join(installDir, "node_modules", packageName);
        if (existsSync(nodeModulesDir)) {
          const packageJsonPath = join(nodeModulesDir, "package.json");
          const packageJson = JSON.parse(readFileSync(packageJsonPath, "utf-8"));
          const installedVersion = packageJson.version;

          if (installedVersion === version) {
            alreadyUpdatedPackages.push(packageName);
            spinner.text = `[${currentPackageIndex}/${totalPackages}] Package already up-to-date: ${packageName}, version: ${version}, skipping.`;
            continue;
          }
        };
      };

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
        isForce
      );
      spinner.text = `${isForce ? chalk.bgYellow("FORCE") : ""} [${currentPackageIndex}/${totalPackages}] Updated package: ${updatedPackage.packageName}, version: ${updatedPackage.version}`;

      if (isDevDependency) {
        packageJson.devDependencies[updatedPackage.packageName] = updatedPackage.version;
        lockFileData.devDependencies[updatedPackage.packageName] = {
          version: updatedPackage.version,
          resolved: updatedPackage.resolved,
          integrity: updatedPackage.integrity,
          dependencies: updatedPackage.dependencies,
        };
      } else {
        packageJson.dependencies[updatedPackage.packageName] = updatedPackage.version;
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
    const durationText = duration < 1000 ? `${duration} ms` : `${(duration / 1000).toFixed(2)} seconds`;

    spinner.succeed(`Packages updated successfully in ${durationText}.`);
    if (alreadyUpdatedPackages.length > 0) console.log(`\n\n${chalk.bgYellow("Packages already up-to-date")} ${alreadyUpdatedPackages.join(", ")}`);
  } catch (error) {
    spinner.fail(`Update failed: ${error.message}`);
    console.error(error);
  }
}