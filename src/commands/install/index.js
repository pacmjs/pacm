import { existsSync, mkdirSync, writeFileSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import ora from 'ora';
import { installPackage } from './installPackage.js';
import { runPostInstallScript } from '../utils/runPostInstallScript.js';

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
  let packageJson = {};

  if (existsSync(packageJsonPath)) {
    packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf-8'));
  } else {
    packageJson = { dependencies: {} };
  }

  if (!packageJson.dependencies) {
    packageJson.dependencies = {};
  }

  const spinner = ora('Starting installation').start();
  const postInstallScripts = [];
  const startTime = Date.now();

  try {
    for (const pkg of packages) {
      const [packageName, version] = pkg.split('@');
      spinner.text = `Parsed package: ${packageName}, version: ${version}`;

      if (!packageName) {
        throw new Error(`Invalid package name: ${pkg}`);
      }

      spinner.text = `Installing package: ${packageName}, version: ${version}`;
      const installedPackage = await installPackage(packageName, version, installDir, version, spinner, postInstallScripts);
      spinner.text = `Installed package: ${installedPackage.packageName}, version: ${installedPackage.version}`;
      packageJson.dependencies[installedPackage.packageName] = installedPackage.version;
    }

    spinner.text = 'Writing package.json';
    writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, 2));

    for (const packageDir of postInstallScripts) {
      await runPostInstallScript(packageDir, spinner);
    }

    await runPostInstallScript(installDir, spinner);

    const endTime = Date.now();
    const duration = endTime - startTime;
    const durationText = duration < 1000 ? `${duration} ms` : `${(duration / 1000).toFixed(2)} seconds`;

    spinner.succeed(`Packages installed successfully in ${durationText}.`);
  } catch (error) {
    spinner.fail(`Installation failed: ${error.message}`);
    console.error(error);
  }
}