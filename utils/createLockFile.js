import { writeFileSync } from "node:fs";

export function createLockFile(lockFileData, lockFilePath) {
  const lockFileContent = JSON.stringify(lockFileData, null, 2);
  writeFileSync(lockFilePath, lockFileContent);
}
