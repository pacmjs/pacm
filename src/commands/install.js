import { exec } from 'node:child_process';
import { existsSync, mkdirSync, writeFileSync, readFileSync, copyFileSync, createWriteStream } from 'node:fs';
import { join } from 'node:path';
import { homedir, tmpdir } from 'node:os';
import fetch from 'node-fetch';
import { x as extract } from 'tar';
import semver from 'semver';
import ora from 'ora';

const globalCacheDir = join(homedir(), '.pacm-cache');

if (!existsSync(globalCacheDir)) {
  mkdirSync(globalCacheDir);
}

async function retryOnECONNRESET(fn, ...args) {
  for (let attempt = 1; attempt <= 3; attempt++) {
    try {
      return await fn(...args);
    } catch (error) {
      if (error.code === 'ECONNRESET') {
        console.warn(`Warning: ${args[0]} error ECONNRESET, retry ${attempt}`);
        if (attempt === 3) throw error;
      } else {
        throw error;
      }
    }
  }
}

async function fetchPackageMetadata(packageName, spinner) {
  spinner.text = `Fetching metadata for ${packageName}`;
  return retryOnECONNRESET(async (packageName) => {
    const response = await fetch(`https://registry.npmjs.org/${packageName}`);
    if (!response.ok) {
      throw new Error(`Failed to fetch metadata for package ${packageName}`);
    }
    return response.json();
  }, packageName);
}

async function downloadAndExtractTarball(url, dest, cachePath, spinner) {
  if (existsSync(cachePath)) {
    spinner.text = `Extracting ${cachePath} to ${dest}`;
    await extract({ file: cachePath, cwd: dest, strip: 1 });
  } else {
    const tempPath = join(tmpdir(), `${Date.now()}-${Math.random().toString(36).substring(7)}.tgz`);
    return retryOnECONNRESET(async (url, dest) => {
      spinner.text = `Downloading tarball from ${url}`;
      const response = await fetch(url);
      if (!response.ok) {
        throw new Error(`Failed to download tarball from ${url}`);
      }
      const fileStream = createWriteStream(tempPath);
      await new Promise((resolve, reject) => {
        response.body.pipe(fileStream).on('finish', resolve).on('error', reject);
      });
      fileStream.on('finish', () => {
        spinner.text = `Writing cache to ${cachePath}`;
        copyFileSync(tempPath, cachePath);
        spinner.text = `Extracting ${tempPath} to ${dest}`;
        extract({ file: tempPath, cwd: dest, strip: 1 }).then(resolve).catch(reject);
      });
      fileStream.on('error', reject);
    }, url, dest);
  }
}

async function runPostInstallScript(packageDir, spinner) {
  const packageJsonPath = join(packageDir, 'package.json');
  if (existsSync(packageJsonPath)) {
    const packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf-8'));
    if (packageJson.scripts && packageJson.scripts.postinstall) {
      spinner.text = `Running postinstall script for ${packageJson.name}`;
      await new Promise((resolve, reject) => {
        exec('npm run postinstall', { cwd: packageDir }, (error, stdout, stderr) => {
          if (error) {
            console.error(`Error running postinstall script for ${packageJson.name}: ${stderr}`);
            reject(error);
          } else {
            console.log(`Postinstall script output for ${packageJson.name}: ${stdout}`);
            resolve();
          }
        });
      });
    }
  }
}

async function installPackage(packageName, version, installDir, originalVersion, spinner, postInstallScripts) {
  let metadata;
  let versionToInstall = version;

  if (version && version.startsWith('npm:')) {
    const [npmPackage, npmVersion] = version.slice(4).split('@');
    metadata = await fetchPackageMetadata(npmPackage, spinner);
    versionToInstall = npmVersion || metadata['dist-tags'].latest;
  } else if (version && version.startsWith('github:')) {
    throw new Error('GitHub packages are not supported yet');
  } else {
    metadata = await fetchPackageMetadata(packageName, spinner);
    versionToInstall = version || metadata['dist-tags'].latest;
  }

  spinner.text = `Validating version for ${packageName}`;
  const availableVersions = Object.keys(metadata.versions);
  const maxSatisfyingVersion = semver.maxSatisfying(availableVersions, versionToInstall);

  if (!maxSatisfyingVersion) {
    throw new Error(`Version ${versionToInstall} of package ${packageName} not found`);
  }

  const packageVersion = metadata.versions[maxSatisfyingVersion];
  const tarballUrl = packageVersion.dist.tarball;
  const packageDir = join(installDir, 'node_modules', packageName);
  const cachePath = join(globalCacheDir, `${packageName}-${maxSatisfyingVersion}.tgz`);

  if (!existsSync(packageDir)) {
    mkdirSync(packageDir, { recursive: true });
    await downloadAndExtractTarball(tarballUrl, packageDir, cachePath, spinner);
  }

  const packageJsonPath = join(packageDir, 'package.json');
  const packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf-8'));
  const dependencies = packageJson.dependencies || {};

  for (const [depName, depVersion] of Object.entries(dependencies)) {
    await installPackage(depName, depVersion, installDir, null, spinner, postInstallScripts);
  }

  postInstallScripts.push(packageDir);

  return { packageName, version: originalVersion || versionToInstall };
}

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