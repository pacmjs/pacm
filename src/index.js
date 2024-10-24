#!/usr/bin/env node
import { argv } from "process";
import {
  help,
  version,
  init,
  install,
  remove,
  run,
  clean,
} from "./commands/index.js";
import { link } from "./functions/link.js";
import { unlink } from "./functions/unlink.js";
import { update } from "./commands/update.js";
import { list } from "./commands/list.js";
import checkScriptExists from "./utils/checkScriptExists.js";
import closestScriptMatch from "./utils/closestScriptMatch.js";
import logger from "./lib/logger.js";

function main() {
  const command = argv[2];
  switch (command) {
    case "help":
      help();
      break;
    case "version":
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
    case "link":
      link(argv.slice(3));
      break;
    case "unlink":
      unlink(argv.slice(3));
      break;
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
    default:
      const scriptExists = checkScriptExists(command);
      if (scriptExists) {
        run([command, ...argv.slice(3)]);
      } else {
        const closestMatch = closestScriptMatch(command);

        if (closestMatch) {
          logger.logError({
            message: exists,
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
