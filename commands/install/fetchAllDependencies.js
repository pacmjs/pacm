import { existsSync, mkdirSync } from "node:fs";
import { join } from "node:path";
import { fetchPackageMetadata } from "../../utils/fetchPackageMetadata.js";
import { installPackage } from "./installPackage.js";

export async function fetchAllDependencies(depName, spinner, packageInfoList, packages, installDir) {
  if (!packages.includes(depName)) {
    packages.push(depName);
    const packageInfo = await fetchPackageMetadata(
      depName,
      spinner,
      packageInfoList.length + 1,
      packages.length
    );

    packageInfoList.push({ ...packageInfo, version: "latest" });

    if (packageInfo.dependencies) {
      await Promise.all(
        Object.keys(packageInfo.dependencies).map((subDepName) =>
          fetchAllDependencies(subDepName, spinner, packageInfoList, packages, installDir)
        )
      );
    }

    const depInstallDir = join(installDir, "node_modules");
    if (!existsSync(depInstallDir)) {
      mkdirSync(depInstallDir, { recursive: true });
    }
    await installPackage(
      spinner,
      depName,
      "latest",
      depInstallDir,
      [],
      { dependencies: {}, devDependencies: {} },
      false,
      0,
      0,
      false
    );
  }
}