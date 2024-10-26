import path from 'node:path';
import { fileURLToPath } from 'node:url';
import process from 'node:process';
import rcedit from 'rcedit';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const exePath = path.join(__dirname, '../release/pacm.exe');
const iconPath = path.join(__dirname, '../assets/logo.ico');

rcedit(exePath, {
  icon: iconPath,
  'version-string': {
    CompanyName: 'Buzzr Works',
    FileDescription: 'PACM',
    ProductName: 'PACM',
    LegalCopyright: 'Buzzr Works',
    LegalTrademarks1: 'Buzzr Works',
    OriginalFilename: 'pacm-win.exe',
  },
  'product-version': '1.0.0',
  'file-version': '1.0.0'
}, (err) => {
  if (err) {
    console.error(`Error: ${err}`);
    process.exit(1);
  }
  console.log('Icon updated successfully');
});