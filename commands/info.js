import logger from "../lib/logger.js";
import ora from "ora";
import process from "node:process";

export async function info(args) {
  const packageName = args[0];

  if (packageName.length === 0) {
    logger.logError({
      message: "You must specify a package name",
      exit: true,
      infoType: " PACM_INFO_ERROR ",
    });
  }

  const [name, version] = packageName.split("@");

  const spinner = ora("Fetching package info").start();

  try {
    const response = await fetch(
      `https://registry.npmjs.org/${version ? name + "/" + version : name}`,
    );
    const data = await response.json();

    spinner.stop();

    logger.logSuccess({
      message: `Successfully fetched info for ${packageName}`,
      infoType: " PACM_INFO_SUCCESS ",
    });

    console.log(
      `Name: ${data.name}\nDescription: ${data.description}\nLatest Version: ${version ? version : data["dist-tags"].latest}\nAuthor: ${data.author ? data.author.name : "None"}\nLicense: ${data.license}\nHomepage: ${data.homepage}\nRepository: ${data.repository.url}\nKeywords: ${data.keywords.join(", ")}\nDependencies: ${data.dependencies ? Object.keys(data.dependencies).join(", ") : "None"}\n`,
    );

    process.exit(0);
  } catch (error) {
    spinner.stop();
    logger.logError({
      message: error.stack,
      exit: true,
      infoType: " PACM_INFO_ERROR ",
    });
  }
}
