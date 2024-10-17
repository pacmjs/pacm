import { existsSync, mkdirSync, readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import semver from 'semver';
import { fetchPackageMetadata } from '../../utils/fetchPackageMetadata.js';
import { downloadAndExtractTarball } from '../../utils/downloadAndExtractTarball.js';
import { homedir } from 'node:os';
import process from 'node:process';

const globalCacheDir = join(homedir(), '.pacm-cache');

if (!existsSync(globalCacheDir)) {
  mkdirSync(globalCacheDir);
}

export async function installPackage(spinner, packageName, version, installDir = process.cwd(), postInstallScripts = []) {
  if (typeof packageName !== 'string' || typeof version !== 'string') {
    throw new Error('[ERRNO1] Invalid packageName or version. Both must be strings.');
  }

  if (!packageName || !version) {
    throw new Error('[ERRNO2] Invalid packageName or version. Both must be defined.');
  }

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
  let maxSatisfyingVersion;

  if (versionToInstall !== "latest") {
    maxSatisfyingVersion = semver.maxSatisfying(availableVersions, versionToInstall);
  
    if (!maxSatisfyingVersion) {
      throw new Error(`Version ${versionToInstall} of package ${packageName} not found`);
    }
  } else {
    maxSatisfyingVersion = metadata['dist-tags'].latest;
  }

  const packageVersion = metadata.versions[maxSatisfyingVersion];
  const tarballUrl = packageVersion.dist.tarball;
  const packageDir = join(installDir, 'node_modules', packageName);
  const cachePath = join(globalCacheDir, packageName.startsWith('@') ? packageName.replace('/', '_') : packageName, `${maxSatisfyingVersion}.tgz`);

  if (!existsSync(packageDir)) {
    mkdirSync(packageDir, { recursive: true });
    await downloadAndExtractTarball(tarballUrl, packageDir, cachePath, spinner);
  }

  const packageJsonPath = join(packageDir, 'package.json');
  const maxRetries = 3;
  const retryDelay = 100;

  let packageJsonExists = false;
  for (let attempt = 0; attempt < maxRetries; attempt++) {
    if (existsSync(packageJsonPath)) {
      packageJsonExists = true;
      break;
    }
    await new Promise(resolve => setTimeout(resolve, retryDelay));
  }

  if (!packageJsonExists) {
    throw new Error(`package.json not found in ${packageDir}`);
  }

  const packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf-8'));
  const dependencies = packageJson.dependencies || {};

  for (const [depName, depVersion] of Object.entries(dependencies)) {
    await installPackage(spinner, depName, depVersion, installDir, postInstallScripts);
  }

  postInstallScripts.push(packageDir);

  return { packageName, version: maxSatisfyingVersion };
}
