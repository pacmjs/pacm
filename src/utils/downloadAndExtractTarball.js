import {
  existsSync,
  copyFileSync,
  createWriteStream,
  mkdirSync,
} from "node:fs";
import { join, dirname } from "node:path";
import { tmpdir } from "node:os";
import fetch from "node-fetch";
import { x as extract } from "tar";
import { retryOnECONNRESET } from "./retry.js";

export async function downloadAndExtractTarball(
  url,
  dest,
  cachePath,
  spinner,
  currentPackageIndex,
  totalPackages,
  isForce,
) {
  if (existsSync(cachePath)) {
    spinner.text = `${isForce ? chalk.bgYellow("FORCE") : ""} [${currentPackageIndex}/${totalPackages}] Extracting ${cachePath} to ${dest}`;
    await extract({ file: cachePath, cwd: dest, strip: 1 });
  } else {
    const tempPath = join(
      tmpdir(),
      `${Date.now()}-${Math.random().toString(36).substring(7)}.tgz`,
    );
    return retryOnECONNRESET(
      async (url, dest) => {
        spinner.text = `${isForce ? chalk.bgYellow("FORCE") : ""} [${currentPackageIndex}/${totalPackages}] Downloading tarball from ${url}`;
        const response = await fetch(url);
        if (!response.ok) {
          throw new Error(`Failed to download tarball from ${url}`);
        }
        const fileStream = createWriteStream(tempPath);
        await new Promise((resolve, reject) => {
          response.body
            .pipe(fileStream)
            .on("finish", resolve)
            .on("error", reject);
          fileStream.on("finish", () => {
            spinner.text = `${isForce ? chalk.bgYellow("FORCE") : ""} [${currentPackageIndex}/${totalPackages}] Writing cache to ${cachePath}`;
            const cacheDir = dirname(cachePath);
            if (!existsSync(cacheDir)) {
              mkdirSync(cacheDir, { recursive: true });
            }
            copyFileSync(tempPath, cachePath);
            spinner.text = `${isForce ? chalk.bgYellow("FORCE") : ""} [${currentPackageIndex}/${totalPackages}] Extracting ${tempPath} to ${dest}`;
            extract({ file: tempPath, cwd: dest, strip: 1 })
              .then(resolve)
              .catch(reject);
          });
          fileStream.on("error", reject);
        });
      },
      url,
      dest,
    );
  }
}
