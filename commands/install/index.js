import { existsSync, writeFileSync, readFileSync } from "node:fs";
import { join } from "node:path";
import ora from "ora";
import { installPackage } from "./installPackage.js";
import { runPostInstallScript } from "../../utils/runPostInstallScript.js";
import { createLockFile } from "../../utils/createLockFile.js";
import { fetchPackageMetadata } from "../../utils/fetchPackageMetadata.js";
import process from "node:process";
import chalk from "chalk";
import logger from "../../lib/logger.js";
import { fetchAllDependencies } from "./fetchAllDependencies.js";

export async function install(args) {
  const packages = [];
  const flags = [];
  const alreadyInstalledPackages = [];

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
    if (
      readFileSync(lockFilePath, "utf-8") === "" ||
      readFileSync(lockFilePath, "utf-8") === "{}" ||
      readFileSync(lockFilePath, "utf-8") === "{\n}" ||
      readFileSync(lockFilePath, "utf-8") === "{\n}\n"
    ) {
      lockFileData = { dependencies: {}, devDependencies: {} };
    } else {
      lockFileData = JSON.parse(readFileSync(lockFilePath, "utf-8"));
    }
  }

  const spinner = ora("[0/0] Fetching package information").start();
  const postInstallScripts = [];

  try {
    if (packages.length === 0) {
      if (existsSync(lockFilePath)) {
        const allDependencies = {
          ...lockFileData.dependencies,
          ...lockFileData.devDependencies,
        };
        const nonDependencyPackages = Object.keys(allDependencies).filter(
          (pkg) => {
            return !Object.values(allDependencies).some(
              (dep) => dep.dependencies && dep.dependencies[pkg],
            );
          },
        );
        packages.push(...nonDependencyPackages);
      } else if (existsSync(packageJsonPath)) {
        packages.push(...Object.keys(packageJson.dependencies));
        packages.push(...Object.keys(packageJson.devDependencies));
      } else {
        throw new Error("No packages to install");
      }
    }

    const isDevDependency = flags.includes("--dev") || flags.includes("-D");
    const isForce = flags.includes("--force") || flags.includes("-f");

    const packageInfoList = await Promise.all(
      packages.map(async (pkg) => {
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
          packages.length,
        );

        if (version === "latest") {
          version = packageInfo["dist-tags"].latest;
        }

        return { ...packageInfo, version };
      }),
    );

    await Promise.all(
      packageInfoList.map(async (packageInfo) => {
        if (packageInfo.dependencies) {
          await Promise.all(
            Object.keys(packageInfo.dependencies).map((depName) =>
              fetchAllDependencies(
                depName,
                spinner,
                packageInfoList,
                packages,
                installDir,
              ),
            ),
          );
        }
      }),
    );

    const calculateTotalDependencies = (
      pkgInfo,
      version,
      visited = new Set(),
    ) => {
      if (visited.has(pkgInfo.name)) return 0;
      visited.add(pkgInfo.name);

      const dependencies = pkgInfo.versions[version].dependencies || {};
      let totalDependencies = Object.keys(dependencies).length;

      for (const depName in dependencies) {
        const depVersion = dependencies[depName];
        const depInfo = packageInfoList.find((info) => info.name === depName);
        if (depInfo) {
          totalDependencies += calculateTotalDependencies(
            depInfo,
            depVersion,
            visited,
          );
        }
      }

      return totalDependencies;
    };

    const totalPackages =
      packageInfoList.reduce(
        (sum, pkgInfo) =>
          sum + calculateTotalDependencies(pkgInfo, pkgInfo.version),
        0,
      ) + packages.length;
    let currentPackageIndex = 0;

    const startTime = Date.now();

    const installPromises = packageInfoList.map(async (pkgInfo) => {
      const { name: packageName, version } = pkgInfo;

      if (!isForce) {
        const nodeModulesDir = join(installDir, "node_modules", packageName);
        if (existsSync(nodeModulesDir)) {
          const packageJsonPath = join(nodeModulesDir, "package.json");
          const packageJson = JSON.parse(
            readFileSync(packageJsonPath, "utf-8"),
          );
          const installedVersion = packageJson.version;

          if (installedVersion === version) {
            alreadyInstalledPackages.push(packageName);
            spinner.text = `[${currentPackageIndex}/${totalPackages}] Package already installed: ${packageName}, version: ${version}, skipping.`;
            return;
          }
        }
      }

      currentPackageIndex++;
      spinner.text = `${isForce ? chalk.bgYellow("FORCE") : ""} [${currentPackageIndex}/${totalPackages}] Installing package: ${packageName}, version: ${version}`;
      const installedPackage = await installPackage(
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
      spinner.text = `${isForce ? chalk.bgYellow("FORCE") : ""} [${currentPackageIndex}/${totalPackages}] Installed package: ${installedPackage.packageName}, version: ${installedPackage.version}`;

      if (isDevDependency) {
        packageJson.devDependencies[installedPackage.packageName] =
          installedPackage.version;
        lockFileData.devDependencies[installedPackage.packageName] = {
          version: installedPackage.version,
          resolved: installedPackage.resolved,
          integrity: installedPackage.integrity,
          dependencies: installedPackage.dependencies,
        };
      } else {
        packageJson.dependencies[installedPackage.packageName] =
          installedPackage.version;
        lockFileData.dependencies[installedPackage.packageName] = {
          version: installedPackage.version,
          resolved: installedPackage.resolved,
          integrity: installedPackage.integrity,
          dependencies: installedPackage.dependencies,
        };
      }
    });

    await Promise.all(installPromises);

    spinner.text = "Writing package.json";
    writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, 2));

    for (const packageDir of postInstallScripts) {
      await runPostInstallScript(packageDir, spinner);
    }

    await runPostInstallScript(installDir, spinner);

    createLockFile(lockFileData, lockFilePath);

    const endTime = Date.now();
    const duration = endTime - startTime;
    const durationText =
      duration < 1000
        ? `${duration} ms`
        : `${(duration / 1000).toFixed(2)} seconds`;

    spinner.stop();
    logger.logSuccess({
      message: `Successfully installed ${packages.length} packages in ${durationText}`,
      successType: " PACM_INSTALL_SUCCESS ",
    });
    if (alreadyInstalledPackages.length > 0)
      console.log(
        `\n\n${chalk.bgYellow("Packages already installed")} ${alreadyInstalledPackages.join(", ")}`,
      );
  } catch (error) {
    spinner.stop();
    logger.logError({
      message: error.stack,
      exit: true,
      errorType: " PACM_ERROR ",
    });
    console.error(error);
  }
}
