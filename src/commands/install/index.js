import { existsSync, writeFileSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import ora from 'ora';
import { installPackage } from './installPackage.js';
import { runPostInstallScript } from '../../utils/runPostInstallScript.js';
import { createLockFile } from '../../utils/createLockFile.js';
import process from 'node:process';

export async function install(args) {
  const packages = [];
  const flags = [];

  args.forEach(arg => {
    if (arg.startsWith('-')) {
      flags.push(arg);
    } else {
      packages.push(arg);
    }
  });

  const installDir = process.cwd();
  const packageJsonPath = join(installDir, 'package.json');
  const lockFilePath = join(installDir, 'pacm.lockp');
  let packageJson = {};
  let lockFileData = { dependencies: {}, devDependencies: {} };

  if (existsSync(packageJsonPath)) {
    packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf-8'));
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
    lockFileData = JSON.parse(readFileSync(lockFilePath, 'utf-8'));
  }

  const spinner = ora('Installing packages').start();
  const postInstallScripts = [];
  const startTime = Date.now();

  try {
    if (packages.length === 0) {
      if (existsSync(lockFilePath)) {
        packages.push(...Object.keys(lockFileData.dependencies));
        packages.push(...Object.keys(lockFileData.devDependencies));
      } else if (existsSync(packageJsonPath)) {
        packages.push(...Object.keys(packageJson.dependencies));
        packages.push(...Object.keys(packageJson.devDependencies));
      }
    }

    const isDevDependency = flags.includes('--save-dev') || flags.includes('-D');

    const totalPackages = packages.length;
    let currentPackageIndex = 0;

    for (const pkg of packages) {
      let packageName, version;

      if (pkg.startsWith('@')) {
        const atIndex = pkg.indexOf('@', 1);
        if (atIndex === -1) {
          packageName = pkg;
          version = 'latest';
        } else {
          packageName = pkg.substring(0, atIndex);
          version = pkg.substring(atIndex + 1) || 'latest';
        }
      } else {
        [packageName, version] = pkg.split('@');
        version = version || 'latest';
      }

      spinner.text = `Parsed package: ${packageName}, version: ${version}`;

      if (!packageName) {
        throw new Error(`Invalid package name: ${pkg}`);
      }

      currentPackageIndex++;
      spinner.text = `[${currentPackageIndex}/${totalPackages}] Installing package: ${packageName}, version: ${version}`;
      const installedPackage = await installPackage(spinner, packageName, version, installDir, postInstallScripts, lockFileData, isDevDependency, currentPackageIndex, totalPackages);
      spinner.text = `[${currentPackageIndex}/${totalPackages}] Installed package: ${installedPackage.packageName}, version: ${installedPackage.version}`;

      if (isDevDependency) {
        packageJson.devDependencies[installedPackage.packageName] = installedPackage.version;
        lockFileData.devDependencies[installedPackage.packageName] = {
          version: installedPackage.version,
          resolved: installedPackage.resolved,
          integrity: installedPackage.integrity,
          dependencies: installedPackage.dependencies
        };
      } else {
        packageJson.dependencies[installedPackage.packageName] = installedPackage.version;
        lockFileData.dependencies[installedPackage.packageName] = {
          version: installedPackage.version,
          resolved: installedPackage.resolved,
          integrity: installedPackage.integrity,
          dependencies: installedPackage.dependencies
        };
      }
    }

    spinner.text = 'Writing package.json';
    writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, 2));

    for (const packageDir of postInstallScripts) {
      await runPostInstallScript(packageDir, spinner);
    }

    await runPostInstallScript(installDir, spinner);

    createLockFile(lockFileData, lockFilePath);

    const endTime = Date.now();
    const duration = endTime - startTime;
    const durationText = duration < 1000 ? `${duration} ms` : `${(duration / 1000).toFixed(2)} seconds`;

    spinner.succeed(`Packages installed successfully in ${durationText}.`);
  } catch (error) {
    spinner.fail(`Installation failed: ${error.message}`);
    console.error(error);
  }
}
