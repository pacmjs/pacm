import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { exec } from 'node:child_process';

export async function runPostInstallScript(packageDir, spinner) {
  const packageJsonPath = join(packageDir, 'package.json');
  if (existsSync(packageJsonPath)) {
    const packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf-8'));
    if (packageJson.scripts && packageJson.scripts.postinstall) {
      spinner.text = `Running postinstall script for ${packageJson.name}`;
      await new Promise((resolve, reject) => {
        exec('npm run postinstall', { cwd: packageDir }, (error, stdout, stderr) => {
          if (error) {
            console.error(`Error running postinstall script for ${packageJson.name}: ${stderr}`);
            reject(error);
          } else {
            console.log(`Postinstall script output for ${packageJson.name}: ${stdout}`);
            resolve();
          }
        });
      });
    }
  }
}