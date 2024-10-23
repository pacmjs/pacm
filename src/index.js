#!/usr/bin/env node
import { argv } from 'process';
import { help, version, init, install } from './commands/index.js';

function main() {
  const command = argv[2];
  switch (command) {
    case 'help':
      help();
      break;
    case 'version':
      version();
      break;
    case 'install':
      install(argv.slice(3));
      break;
    case 'init':
      init(argv.slice(3));
      break;
    default:
      console.log('Unknown command. Use "pacm help" for a list of commands.');
      break;
  }
}

main();
