#!/usr/bin/env node
/* eslint-disable no-case-declarations */
import { argv } from "node:process";
import {
  help,
  version,
  init,
  install,
  remove,
  run,
  clean,
  publish,
  search,
  info,
} from "../commands/index.js";
import { update } from "../commands/update.js";
import { list } from "../commands/list.js";
import checkScriptExists from "../utils/checkScriptExists.js";
import closestScriptMatch from "../utils/closestScriptMatch.js";
import logger from "../lib/logger.js";
import process from "node:process";

function main() {
  const command = argv[2];
  switch (command) {
    case "help":
      help();
      break;
    case "--help":
      help();
      break;
    case "-h":
      help();
      break;
    case "-H":
      help();
      break;
    case "version":
      version();
      break;
    case "--version":
      version();
      break;
    case "-v":
      version();
      break;
    case "-V":
      version();
      break;
    case "install":
      install(argv.slice(3));
      break;
    case "i":
      install(argv.slice(3));
      break;
    case "add":
      install(argv.slice(3));
      break;
    case "remove":
      remove(argv.slice(3));
      break;
    case "rm":
      remove(argv.slice(3));
      break;
    case "uninstall":
      remove(argv.slice(3));
      break;
    case "init":
      init(argv.slice(3));
      break;
    /*case "link":
      link(argv.slice(3));
      break;
    case "unlink":
      unlink(argv.slice(3));
      break;*/
    case "update":
      update(argv.slice(3));
      break;
    case "list":
      list(argv.slice(3));
      break;
    case "run":
      run(argv.slice(3));
      break;
    case "clean":
      clean(argv.slice(3));
      break;
    case "publish":
      publish(argv.slice(3));
      break;
    case "search":
      search(argv.slice(3));
      break;
    case "info":
      info(argv.slice(3));
      break;
    default:
      const scriptExists = checkScriptExists(command);
      if (scriptExists) {
        run([command, ...argv.slice(3)]);
      } else {
        const closestMatch = closestScriptMatch(command);

        if (closestMatch) {
          logger.logError({
            message: scriptExists,
            exit: false,
            errorType: " PACM_RUNTIME_ERROR ",
          });
          console.log(
            `\n\nDid you mean "${closestMatch}"? Run "pacm run ${closestMatch}" to execute.`,
          );
          process.exit(1);
        } else {
          logger.logError({
            message: `Command "${command}" not found. Run "pacm help" for a list of available commands.`,
            exit: true,
            errorType: " PACM_ERROR ",
          });
        }
      }
      break;
  }
}

main();
