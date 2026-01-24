/*
  Generates platform icons for Tauri from the root-level ghost logo.PNG.
  - Produces PNG sizes and ICO/ICNS into src-tauri/icons/
*/

import { mkdirSync, existsSync, copyFileSync } from 'node:fs';
import { resolve } from 'node:path';
import iconGen from 'icon-gen';

async function main() {
  const projectRoot = resolve(process.cwd());
  const sourcePng = resolve(projectRoot, 'ghost logo.PNG');
  const outDir = resolve(projectRoot, 'src-tauri', 'icons');

  if (!existsSync(sourcePng)) {
    console.error(`Source image not found: ${sourcePng}`);
    process.exit(1);
  }
  try {
    mkdirSync(outDir, { recursive: true });
  } catch {}

  console.log('Generating icons from:', sourcePng);

  await iconGen(sourcePng, outDir, {
    report: true,
    ico: { name: 'icon' },
    icns: { name: 'icon' },
    favicon: { generate: false },
    modes: [
      {
        name: 'png',
        sizes: [32, 128, 256, 512],
      },
      {
        name: 'ico',
        sizes: [16, 24, 32, 48, 64, 128, 256],
      },
      {
        name: 'icns',
        sizes: [16, 32, 64, 128, 256, 512, 1024],
      },
    ],
  });

  console.log('Icons generated at:', outDir);

  // Ensure Tauri-referenced filenames exist
  // - 128x128@2x.png should exist; if not, create it from 256x256.png
  const p128x = resolve(outDir, '128x128.png');
  const p128x2 = resolve(outDir, '128x128@2x.png');
  const p256 = resolve(outDir, '256x256.png');
  try {
    if (!existsSync(p128x2)) {
      if (existsSync(p256)) {
        copyFileSync(p256, p128x2);
        console.log('Created 128x128@2x.png from 256x256.png');
      } else if (existsSync(p128x)) {
        copyFileSync(p128x, p128x2);
        console.log('Duplicated 128x128.png to 128x128@2x.png');
      }
    }
  } catch (e) {
    console.warn('Could not ensure 128x128@2x.png:', e);
  }
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});


