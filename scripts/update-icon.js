import path from 'node:path';
import { fileURLToPath } from 'node:url';
import process from 'node:process';
import rcedit from 'rcedit';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const architectures = ['x64', 'arm64'];
const iconPath = path.join(__dirname, '../assets/logo.ico');

architectures.forEach(arch => {
  const exePath = path.join(__dirname, `../release/pacm-windows-${arch}.exe`);

  rcedit(exePath, {
    icon: iconPath,
    'version-string': {
      CompanyName: 'Buzzr Works',
      FileDescription: 'PACM',
      ProductName: 'PACM',
      LegalCopyright: 'Buzzr Works',
      LegalTrademarks1: 'Buzzr Works',
      OriginalFilename: `pacm-windows-${arch}.exe`,
    },
    'product-version': '1.0.0-alpha.2',
    'file-version': '1.0.0-alpha.2'
  }, (err) => {
    if (err) {
      console.error(`Error updating icon for ${arch}: ${err}`);
      process.exit(1);
    }
    console.log(`Icon updated successfully for ${arch}`);
  });
});