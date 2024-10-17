#!/usr/bin/env node

import { exec } from 'node:child_process';
import { argv } from 'node:process';
import { help, version } from '../core/commands'

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