#!/usr/bin/env node
import { argv } from "process";
import { help, version, init, install, remove } from "./commands/index.js";
import { link } from "./functions/link.js";
import { unlink } from "./functions/unlink.js";
import { update } from "./commands/update.js";
import { list } from "./commands/list.js";

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
    default:
      console.log('Unknown command. Use "pacm help" for a list of commands.');
      break;
  }
}

main();
