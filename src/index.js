#!/usr/bin/env node

// Import required modules
import { exec } from 'child_process';
import { argv } from 'process';
import { help, version } from './commands/index.js';

function main() {
  const command = argv[2];

  switch (command) {
    case 'help':
      help();
      break;
    case 'version':
      version();
      break;
    default:
      console.log('Unknown command. Use "pacm help" for a list of commands.');
  }
}

main();