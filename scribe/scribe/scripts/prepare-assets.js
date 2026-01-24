import { mkdirSync, existsSync, copyFileSync } from 'node:fs';
import { resolve } from 'node:path';

const projectRoot = resolve(process.cwd());
const source = resolve(projectRoot, 'ghost logo.PNG');
const publicDir = resolve(projectRoot, 'public');
const target = resolve(publicDir, 'ghost_logo.png');

try {
  mkdirSync(publicDir, { recursive: true });
  if (!existsSync(source)) {
    console.warn('ghost logo.PNG not found at project root; skipping public copy');
    process.exit(0);
  }
  copyFileSync(source, target);
  console.log('Copied ghost logo to public/ghost_logo.png');
} catch (e) {
  console.error('Failed to prepare assets:', e);
  process.exit(1);
}


