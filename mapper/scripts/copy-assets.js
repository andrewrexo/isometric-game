import { existsSync, rmSync, cpSync, lstatSync, readdirSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const publicDir = resolve(__dirname, '../public');

const symlinks = [
  { link: 'assets', target: '../../client/assets' },
  { link: 'entities', target: '../../rust-server/data/entities' },
];

console.log('Preparing assets for production build...');
console.log('Working directory:', process.cwd());
console.log('Public directory:', publicDir);

for (const { link, target } of symlinks) {
  const linkPath = resolve(publicDir, link);
  const targetPath = resolve(publicDir, target);

  // Check if it's a symlink or directory
  if (existsSync(linkPath)) {
    try {
      const stats = lstatSync(linkPath);
      if (stats.isSymbolicLink()) {
        // Check if symlink is working (target exists)
        try {
          const contents = readdirSync(linkPath);
          if (contents.length > 0) {
            console.log(`Symlink ${link} is working, skipping copy`);
            continue;
          }
        } catch {
          // Symlink broken, need to replace it
        }
        console.log(`Removing broken symlink: ${link}`);
        rmSync(linkPath);
      } else if (stats.isDirectory()) {
        console.log(`${link} is already a directory, skipping`);
        continue;
      }
    } catch (err) {
      console.error(`Error checking ${link}:`, err.message);
      continue;
    }
  }

  // Copy the actual files
  if (existsSync(targetPath)) {
    console.log(`Copying ${target} -> ${link}`);
    cpSync(targetPath, linkPath, { recursive: true });
    console.log(`Copied ${link} successfully`);
  } else {
    console.warn(`Warning: Target ${target} does not exist, skipping ${link}`);
  }
}

console.log('Asset preparation complete!');
