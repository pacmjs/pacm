/* eslint-disable no-unused-vars */
import { execSync } from "node:child_process";
import ora from "ora";
import logger from "../lib/logger.js";

export function publish(args) {
  const flags = [];

  for (const arg of args) {
    if (arg.startsWith("--")) {
      flags.push(arg);
    }
  }

  const spinner = ora("Passing you over to the custom CLI").start();

  try {
    execSync("npm --version");
  } catch (error) {
    spinner.fail("NPM is not installed");
    return;
  }

  spinner.text = "Checking Git Installation";

  try {
    execSync("git --version");
  } catch (error) {
    spinner.fail("Git is not installed");
    return;
  }

  spinner.text = "Checking NPM Authentication";

  try {
    execSync("npm whoami");
  } catch (error) {
    spinner.fail("Not authenticated with NPM. Run `npm login` to authenticate");
    return;
  }

  spinner.text = "Passing you over to the custom CLI";
  spinner.stop();

  const command = `npm publish ${flags.join(" ")}`;

  try {
    const { stdout, stderr } = execSync(command, {
      encoding: "utf-8",
      stdio: "pipe",
    });
    const output = stdout + stderr;

    const packageInfo = output.match(/npm notice 📦\s+(.+?)\n/);
    const tarballDetails = output.match(
      /npm notice Tarball Details\n([\s\S]+?)\n\n/,
    );
    const publishInfo = output.match(
      /npm notice Publishing to (.+?) with tag (.+?) and default access/,
    );
    const errorInfo = output.match(/npm error (.+)/).slice(1);

    if (packageInfo) {
      console.log(`Package: ${packageInfo[1]}`);
    }
    if (tarballDetails) {
      console.log(`Tarball Details:\n${tarballDetails[1]}`);
    }
    if (publishInfo) {
      console.log(
        `Publishing to: ${publishInfo[1]} with tag ${publishInfo[2]}`,
      );
    }
    if (errorInfo) {
      logger.logError({
        message: errorInfo[1],
        exit: true,
        errorType: " PACM_PUBLISH_ERROR ",
      });
    }
  } catch (error) {
    logger.logError({
      message: error.message,
      exit: true,
      errorType: " PACM_PUBLISH_ERROR ",
    });
  }
}
