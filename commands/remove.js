import logger from "../lib/logger.js";
import { fetchPackageMetadata } from "../utils/fetchPackageMetadata.js";
import ora from "ora";
import fs, { existsSync, readFileSync, writeFileSync, rmSync } from "node:fs";
import { join } from "node:path";

export const remove = async (args) => {
  const packages = [];
  const flags = [];
  const notInstalledPackages = [];
  const initiallyInstalledPackages = new Set();

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