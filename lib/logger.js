import chalk from "chalk";

export function logError({ message, exit = true, errorType = " PACM_ERROR " }) {
  console.error(`${chalk.bgRed.white(errorType)} ${chalk.red(message)}`);
  if (exit) {
    process.exit(1);
  }
}

export function logWarning({ message, warningType = " PACM_WARNING " }) {
  console.warn(`${chalk.bgYellow.black(warningType)} ${chalk.yellow(message)}`);
}

export function logInfo({ message, infoType = " PACM_INFO " }) {
  console.info(`${chalk.bgBlue.white(infoType)} ${chalk.blue(message)}`);
}

export function logSuccess({ message, successType = " PACM_SUCCESS " }) {
  console.log(`${chalk.bgGreen.white(successType)} ${chalk.green(message)}`);
}

export default {
  logError,
  logWarning,
  logInfo,
  logSuccess,
};
