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
  let lockFileData = { dependencies: {} };

  if (existsSync(packageJsonPath)) {
    packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf-8'));
  } else {
    packageJson = { dependencies: {} };
  }

  if (!packageJson.dependencies) {
    packageJson.dependencies = {};
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
      } else if (existsSync(packageJsonPath)) {
        packages.push(...Object.keys(packageJson.dependencies));
      }
    }

    for (const pkg of packages) {
      let [packageName, version] = pkg.split('@');

      if (version === undefined || version === '' || version === 'latest' || version === null) {
        version = 'latest';
      }

      spinner.text = `Parsed package: ${packageName}, version: ${version}`;

      if (!packageName) {
        throw new Error(`Invalid package name: ${pkg}`);
      }

      spinner.text = `Installing package: ${packageName}, version: ${version}`;
      const installedPackage = await installPackage(spinner, packageName, version, installDir, postInstallScripts);
      spinner.text = `Installed package: ${installedPackage.packageName}, version: ${installedPackage.version}`;
      packageJson.dependencies[installedPackage.packageName] = installedPackage.version;

      // Add package information to lock file data
      lockFileData.dependencies[installedPackage.packageName] = {
        version: installedPackage.version,
        resolved: installedPackage.resolved,
        integrity: installedPackage.integrity,
        dependencies: installedPackage.dependencies
      };
    }

    spinner.text = 'Writing package.json';
    writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, 2));

    for (const packageDir of postInstallScripts) {
      await runPostInstallScript(packageDir, spinner);
    }

    await runPostInstallScript(installDir, spinner);

    // Create and write the lock file
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
