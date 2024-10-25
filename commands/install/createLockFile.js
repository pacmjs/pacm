import { writeFileSync } from "node:fs";

export function createLockFile(lockFileData, lockFilePath) {
  writeFileSync(lockFilePath, JSON.stringify(lockFileData, null, 2));
}
