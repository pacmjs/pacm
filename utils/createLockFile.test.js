import { createLockFile } from "./createLockFile";
import { readFileSync, unlinkSync, existsSync } from "node:fs";
import { join } from "node:path";
import { describe, it, expect, beforeEach, afterEach } from "@jest/globals";

const lockFilePath = join(__dirname, "test.lockp");

describe("createLockFile", () => {
  beforeEach(() => {
    if (existsSync(lockFilePath)) {
      unlinkSync(lockFilePath);
    }
  });

  afterEach(() => {
    if (existsSync(lockFilePath)) {
      unlinkSync(lockFilePath);
    }
  });

  it("should create a lock file with the correct content", () => {
    const lockFileData = {
      dependencies: {
        "package-a": {
          version: "1.0.0",
          resolved:
            "https://registry.npmjs.org/package-a/-/package-a-1.0.0.tgz",
          integrity: "sha512-abc123",
          dependencies: {
            "package-b": "^2.0.0",
          },
        },
        "package-b": {
          version: "2.0.0",
          resolved:
            "https://registry.npmjs.org/package-b/-/package-b-2.0.0.tgz",
          integrity: "sha512-def456",
        },
      },
      devDependencies: {
        "package-c": {
          version: "3.0.0",
          resolved:
            "https://registry.npmjs.org/package-c/-/package-c-3.0.0.tgz",
          integrity: "sha512-ghi789",
          dependencies: {
            "package-d": "^4.0.0",
          },
        },
        "package-d": {
          version: "4.0.0",
          resolved:
            "https://registry.npmjs.org/package-d/-/package-d-4.0.0.tgz",
          integrity: "sha512-jkl012",
        },
      },
    };

    createLockFile(lockFileData, lockFilePath);

    const lockFileContent = JSON.parse(readFileSync(lockFilePath, "utf-8"));
    expect(lockFileContent).toEqual(lockFileData);
  });

  it("should overwrite an existing lock file", () => {
    const initialLockFileData = {
      dependencies: {
        "package-a": {
          version: "1.0.0",
          resolved:
            "https://registry.npmjs.org/package-a/-/package-a-1.0.0.tgz",
          integrity: "sha512-abc123",
        },
      },
      devDependencies: {
        "package-c": {
          version: "3.0.0",
          resolved:
            "https://registry.npmjs.org/package-c/-/package-c-3.0.0.tgz",
          integrity: "sha512-ghi789",
        },
      },
    };

    const newLockFileData = {
      dependencies: {
        "package-b": {
          version: "2.0.0",
          resolved:
            "https://registry.npmjs.org/package-b/-/package-b-2.0.0.tgz",
          integrity: "sha512-def456",
        },
      },
      devDependencies: {
        "package-d": {
          version: "4.0.0",
          resolved:
            "https://registry.npmjs.org/package-d/-/package-d-4.0.0.tgz",
          integrity: "sha512-jkl012",
        },
      },
    };

    createLockFile(initialLockFileData, lockFilePath);
    createLockFile(newLockFileData, lockFilePath);

    const lockFileContent = JSON.parse(readFileSync(lockFilePath, "utf-8"));
    expect(lockFileContent).toEqual(newLockFileData);
  });
});
