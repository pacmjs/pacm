import logger from "../lib/logger.js";
import { fetchPackageMetadata } from "../utils/fetchPackageMetadata.js";
import ora from "ora";
import fs, { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import process from "node:process";

export const remove = async (args) => {
  const packages = [];
  const flags = [];
  const notInstalledPackages = [];
  const initiallyInstalledPackages = new Set();
  let lockfileDeleted = false;

  if (args.length === 0) {
    logger.logError({
      message: "Please provide the package name(s) to remove",
      exit: true,
      errorType: " PACM_REMOVAL_ERROR ",
    });
  }

  args.forEach((arg) => {
    if (arg.startsWith("-")) {
      flags.push(arg);
    } else {
      if (arg.startsWith("@")) {
        const [scope, name] = arg.split("/");
        packages.push(`${scope}/${name.split("@")[0]}`);
      } else {
        packages.push(arg.split("@")[0]);
      }
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

  Object.keys(packageJson.dependencies).forEach((pkg) =>
    initiallyInstalledPackages.add(pkg),
  );
  Object.keys(packageJson.devDependencies).forEach((pkg) =>
    initiallyInstalledPackages.add(pkg),
  );

  const spinner = ora(
    `Removing package${packages.length > 1 ? "s" : ""}`,
  ).start();

  const removeAllDependencies = async (pkg) => {
    const packageInfo = await fetchPackageMetadata(pkg, spinner, 1, 1);
    const latestVersion = packageInfo["dist-tags"].latest;
    const dependencies = packageInfo.versions[latestVersion].dependencies || {};

    try {
    if (packageJson.dependencies[pkg]) {
      delete packageJson.dependencies[pkg];
    } else if (packageJson.devDependencies[pkg]) {
      delete packageJson.devDependencies[pkg];
    } else if (initiallyInstalledPackages.has(pkg)) {
      notInstalledPackages.push(pkg);
    }

    if (lockFileData.dependencies[pkg]) {
      delete lockFileData.dependencies[pkg];
    } else if (lockFileData.devDependencies[pkg]) {
      delete lockFileData.devDependencies[pkg];
    }
    } catch (error) {
      spinner.stop();

      logger.logError({
        message: `Package ${pkg} is not installed`,
        exit: true,
        errorType: " PACM_REMOVAL_ERROR ",
      });
    }

    if (existsSync(join(installDir, "node_modules", pkg))) {
      fs.rmSync(join(installDir, "node_modules", pkg), { recursive: true });
    }

    for (const depName in dependencies) {
      await removeAllDependencies(depName);
    }
  };

  for (const pkg of packages) {
    spinner.text = `Removing ${pkg}`;
    await removeAllDependencies(pkg);
  }

  if (Object.keys(packageJson.dependencies).length === 0) {
    delete packageJson.dependencies;
  }

  if (Object.keys(packageJson.devDependencies).length === 0) {
    delete packageJson.devDependencies;
  }

  if (Object.keys(lockFileData.dependencies).length === 0) {
    delete lockFileData.dependencies;
  }

  if (Object.keys(lockFileData.devDependencies).length === 0) {
    delete lockFileData.devDependencies;
  }

  if (fs.existsSync(join(installDir, "node_modules"))) {
    const nodeModules = fs.readdirSync(join(installDir, "node_modules"));
    if (nodeModules.length === 0) {
      fs.rmdirSync(join(installDir, "node_modules"));
    }
  }

  writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, 2));
  if (!lockfileDeleted) writeFileSync(lockFilePath, JSON.stringify(lockFileData, null, 2));

  spinner.succeed(`Completed package${packages.length > 1 ? "s" : ""} removal`);
  if (notInstalledPackages.length > 0) {
    logger.logError({
      message: `The following package${
        notInstalledPackages.length > 1 ? "s are" : " is"
      } not installed: ${notInstalledPackages.join(", ")}`,
      exit: false,
      errorType: "PACM_REMOVAL_ERROR",
    });
  }

  writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, 2));
  writeFileSync(lockFilePath, JSON.stringify(lockFileData, null, 2));
};
