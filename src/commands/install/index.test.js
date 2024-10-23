import { existsSync, writeFileSync, unlinkSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { install } from './index.js';
import { createLockFile } from '../../utils/createLockFile.js';
import { describe, it, expect, beforeEach, afterEach, jest } from '@jest/globals';

const installDir = join(__dirname, 'test-install');
const packageJsonPath = join(installDir, 'package.json');
const lockFilePath = join(installDir, 'pacm.lockp');

jest.mock('./installPackage.js', () => ({
  installPackage: jest.fn(async (spinner, packageName, version, installDir, postInstallScripts, lockFileData, isDevDependency) => ({
    packageName,
    version,
    resolved: `https://registry.npmjs.org/${packageName}/-/${packageName}-${version}.tgz`,
    integrity: 'sha512-abc123',
    dependencies: {}
  }))
}));

jest.mock('../../utils/runPostInstallScript.js', () => ({
  runPostInstallScript: jest.fn(async (packageDir, spinner) => {})
}));

describe('install', () => {
  beforeEach(() => {
    if (existsSync(packageJsonPath)) {
      unlinkSync(packageJsonPath);
    }
    if (existsSync(lockFilePath)) {
      unlinkSync(lockFilePath);
    }
  });

  afterEach(() => {
    if (existsSync(packageJsonPath)) {
      unlinkSync(packageJsonPath);
    }
    if (existsSync(lockFilePath)) {
      unlinkSync(lockFilePath);
    }
  });

  it('should install packages from pacm.lockp if it exists', async () => {
    const lockFileData = {
      dependencies: {
        'axios': {
          version: '1.7.7',
          resolved: 'https://registry.npmjs.org/axios/-/axios-1.7.7.tgz',
          integrity: 'sha512-abc123'
        }
      },
      devDependencies: {
        'jest': {
          version: '29.7.0',
          resolved: 'https://registry.npmjs.org/jest/-/jest-29.7.0.tgz',
          integrity: 'sha512-def456'
        }
      }
    };

    createLockFile(lockFileData, lockFilePath);

    await install([]);

    expect(require('./installPackage.js').installPackage).toHaveBeenCalledWith(
      expect.anything(),
      'axios',
      '1.7.7',
      installDir,
      expect.any(Array),
      lockFileData,
      false
    );

    expect(require('./installPackage.js').installPackage).toHaveBeenCalledWith(
      expect.anything(),
      'jest',
      '29.7.0',
      installDir,
      expect.any(Array),
      lockFileData,
      true
    );
  });

  it('should install packages from package.json if pacm.lockp does not exist', async () => {
    const packageJsonData = {
      dependencies: {
        'axios': '1.7.7'
      },
      devDependencies: {
        'jest': '29.7.0'
      }
    };

    writeFileSync(packageJsonPath, JSON.stringify(packageJsonData, null, 2));

    await install([]);

    expect(require('./installPackage.js').installPackage).toHaveBeenCalledWith(
      expect.anything(),
      'axios',
      '1.7.7',
      installDir,
      expect.any(Array),
      expect.any(Object),
      false
    );

    expect(require('./installPackage.js').installPackage).toHaveBeenCalledWith(
      expect.anything(),
      'jest',
      '29.7.0',
      installDir,
      expect.any(Array),
      expect.any(Object),
      true
    );
  });

  it('should install packages from pacm.lockp or package.json if no packages are specified in the command arguments', async () => {
    const lockFileData = {
      dependencies: {
        'axios': {
          version: '1.7.7',
          resolved: 'https://registry.npmjs.org/axios/-/axios-1.7.7.tgz',
          integrity: 'sha512-abc123'
        }
      },
      devDependencies: {
        'jest': {
          version: '29.7.0',
          resolved: 'https://registry.npmjs.org/jest/-/jest-29.7.0.tgz',
          integrity: 'sha512-def456'
        }
      }
    };

    createLockFile(lockFileData, lockFilePath);

    await install([]);

    expect(require('./installPackage.js').installPackage).toHaveBeenCalledWith(
      expect.anything(),
      'axios',
      '1.7.7',
      installDir,
      expect.any(Array),
      lockFileData,
      false
    );

    expect(require('./installPackage.js').installPackage).toHaveBeenCalledWith(
      expect.anything(),
      'jest',
      '29.7.0',
      installDir,
      expect.any(Array),
      lockFileData,
      true
    );

    unlinkSync(lockFilePath);

    const packageJsonData = {
      dependencies: {
        'axios': '1.7.7'
      },
      devDependencies: {
        'jest': '29.7.0'
      }
    };

    writeFileSync(packageJsonPath, JSON.stringify(packageJsonData, null, 2));

    await install([]);

    expect(require('./installPackage.js').installPackage).toHaveBeenCalledWith(
      expect.anything(),
      'axios',
      '1.7.7',
      installDir,
      expect.any(Array),
      expect.any(Object),
      false
    );

    expect(require('./installPackage.js').installPackage).toHaveBeenCalledWith(
      expect.anything(),
      'jest',
      '29.7.0',
      installDir,
      expect.any(Array),
      expect.any(Object),
      true
    );
  });

  it('should install devDependencies with --save-dev flag', async () => {
    const packageJsonData = {
      devDependencies: {
        'jest': '29.7.0'
      }
    };

    writeFileSync(packageJsonPath, JSON.stringify(packageJsonData, null, 2));

    await install(['--save-dev']);

    expect(require('./installPackage.js').installPackage).toHaveBeenCalledWith(
      expect.anything(),
      'jest',
      '29.7.0',
      installDir,
      expect.any(Array),
      expect.any(Object),
      true
    );
  });

  it('should install devDependencies with -D flag', async () => {
    const packageJsonData = {
      devDependencies: {
        'jest': '29.7.0'
      }
    };

    writeFileSync(packageJsonPath, JSON.stringify(packageJsonData, null, 2));

    await install(['-D']);

    expect(require('./installPackage.js').installPackage).toHaveBeenCalledWith(
      expect.anything(),
      'jest',
      '29.7.0',
      installDir,
      expect.any(Array),
      expect.any(Object),
      true
    );
  });

  it('should save packages as devDependencies when --save-dev flag is used', async () => {
    const packageJsonData = {
      dependencies: {},
      devDependencies: {}
    };

    writeFileSync(packageJsonPath, JSON.stringify(packageJsonData, null, 2));

    await install(['axios@1.7.7', '--save-dev']);

    const updatedPackageJson = JSON.parse(readFileSync(packageJsonPath, 'utf-8'));
    expect(updatedPackageJson.devDependencies).toHaveProperty('axios', '1.7.7');
  });

  it('should save packages as devDependencies when -D flag is used', async () => {
    const packageJsonData = {
      dependencies: {},
      devDependencies: {}
    };

    writeFileSync(packageJsonPath, JSON.stringify(packageJsonData, null, 2));

    await install(['axios@1.7.7', '-D']);

    const updatedPackageJson = JSON.parse(readFileSync(packageJsonPath, 'utf-8'));
    expect(updatedPackageJson.devDependencies).toHaveProperty('axios', '1.7.7');
  });

  it('should force overwrite existing packages when --force flag is used', async () => {
    const packageJsonData = {
      dependencies: {
        'axios': '1.7.6'
      }
    };

    writeFileSync(packageJsonPath, JSON.stringify(packageJsonData, null, 2));

    await install(['axios@1.7.7', '--force']);

    const updatedPackageJson = JSON.parse(readFileSync(packageJsonPath, 'utf-8'));
    expect(updatedPackageJson.dependencies).toHaveProperty('axios', '1.7.7');
  });
});
