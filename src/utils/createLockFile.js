import { writeFileSync } from 'node:fs';

export function createLockFile(lockFileData, lockFilePath) {
  const filteredLockFileData = {
    dependencies: {},
    devDependencies: {}
  };

  for (const [depName, depData] of Object.entries(lockFileData.dependencies)) {
    if (!lockFileData.devDependencies[depName]) {
      filteredLockFileData.dependencies[depName] = depData;
    }
  }

  for (const [depName, depData] of Object.entries(lockFileData.devDependencies)) {
    if (!lockFileData.dependencies[depName]) {
      filteredLockFileData.devDependencies[depName] = depData;
    }
  }

  const lockFileContent = JSON.stringify(filteredLockFileData, null, 2);
  writeFileSync(lockFilePath, lockFileContent);
}
