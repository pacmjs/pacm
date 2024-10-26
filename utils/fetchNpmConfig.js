import { readFileSync, existsSync } from "node:fs";
import { join } from "node:path";
import { homedir } from "node:os";
import process from "node:process";

function readNpmrcFile() {
  const projectNpmrcPath = join(process.cwd(), ".npmrc");
  const homeNpmrcPath = join(homedir(), ".npmrc");

  if (existsSync(projectNpmrcPath)) {
    return readFileSync(projectNpmrcPath, "utf-8");
  } else if (existsSync(homeNpmrcPath)) {
    return readFileSync(homeNpmrcPath, "utf-8");
  }

  return null;
}

function readRegistryFromNpmrcOrPackageJson() {
  const npmrcContent = readNpmrcFile();
  if (npmrcContent) {
    const registryMatch = npmrcContent.match(/^registry\s*=\s*(.*)$/m);
    if (registryMatch) {
      return registryMatch[1];
    }
  }

  const packageJsonPath = join(process.cwd(), "package.json");
  if (existsSync(packageJsonPath)) {
    const packageJson = JSON.parse(readFileSync(packageJsonPath, "utf-8"));
    if (packageJson.publishConfig && packageJson.publishConfig.registry) {
      return packageJson.publishConfig.registry;
    }
  }

  return "https://registry.npmjs.org";
}

function fetchNpmConfig() {
  const registry = readRegistryFromNpmrcOrPackageJson();
  return {
    registry,
  };
}

export { fetchNpmConfig };
