import logger from "../lib/logger.js";
import process from "node:process";
import ora from "ora";

export async function search(args) {
  const query = args.join(" ");
  const packages = [];

  if (query.length === 0) {
    logger.logError({
      message: "You must specify a search query",
      exit: true,
      infoType: " PACM_SEARCH_ERROR ",
    });
  }

  const spinner = ora("Searching for packages").start();

  const response = await fetch(
    `https://registry.npmjs.org/-/v1/search?text=${query}&size=10`,
  );
  const data = await response.json();

  if (data.objects.length === 0) {
    logger.logError({
      message: `No packages found for "${query}"`,
      exit: true,
      infoType: " PACM_SEARCH_ERROR ",
    });
  }

  data.objects.forEach((pkg) => {
    packages.push(pkg.package.name);
  });

  spinner.stop();

  packages.forEach((pkg) => {
    logger.logInfo({
      message: pkg,
      infoType: " PACM_SEARCH_RESULT ",
    });
  });

  process.exit(0);
}
