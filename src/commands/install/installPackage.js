import { existsSync, mkdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import semver from 'semver';
import { fetchPackageMetadata } from '../../utils/fetchPackageMetadata.js';
import { downloadAndExtractTarball } from '../../utils/downloadAndExtractTarball.js';

const globalCacheDir = join(homedir(), '.pacm-cache');

if (!existsSync(globalCacheDir)) {
  mkdirSync(globalCacheDir);
}

export async function installPackage(packageName, version, installDir, originalVersion, spinner, postInstallScripts) {
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