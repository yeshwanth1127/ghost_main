/*
  Generates platform icons for Tauri from the root-level ghost logo.PNG.
  - Applies Ghost brand peach color (#ff9a8b) to match the in-app logo
  - Produces PNG sizes and ICO/ICNS into src-tauri/icons/
*/

import { mkdirSync, existsSync, copyFileSync, writeFileSync, unlinkSync } from 'node:fs';
import { resolve } from 'node:path';
import iconGen from 'icon-gen';
import sharp from 'sharp';

// Ghost brand peach - same as --primary in global.css
const GHOST_PEACH = { r: 255, g: 154, b: 139 };

// Scale factor: logo fills more of the icon (1.5 = 50% larger within bounds)
const LOGO_SCALE = 1.5;

async function createPeachColoredLogo(sourcePng) {
  const img = sharp(sourcePng).ensureAlpha();
  const { data, info } = await img.raw().ensureAlpha().toBuffer({ resolveWithObject: true });
  const { width, height, channels } = info;

  // Replace any non-transparent pixel with peach; preserve alpha
  for (let i = 0; i < data.length; i += channels) {
    const alpha = data[i + 3];
    if (alpha > 0) {
      data[i] = GHOST_PEACH.r;
      data[i + 1] = GHOST_PEACH.g;
      data[i + 2] = GHOST_PEACH.b;
    }
  }

  return sharp(data, { raw: { width, height, channels } }).png().toBuffer();
}

async function main() {
  const projectRoot = resolve(process.cwd());
  const sourcePng = resolve(projectRoot, 'ghost logo.PNG');
  const outDir = resolve(projectRoot, 'src-tauri', 'icons');
  const coloredSource = resolve(projectRoot, 'src-tauri', 'icons', '.ghost_logo_peach.png');

  if (!existsSync(sourcePng)) {
    console.error(`Source image not found: ${sourcePng}`);
    process.exit(1);
  }
  try {
    mkdirSync(outDir, { recursive: true });
  } catch {}

  // Create peach-colored logo to match in-app branding
  console.log('Applying Ghost peach color (#ff9a8b) to logo...');
  let peachBuffer = await createPeachColoredLogo(sourcePng);

  // Scale logo to appear larger within icon bounds (zoom in)
  const meta = await sharp(peachBuffer).metadata();
  const w = meta.width || 512;
  const h = meta.height || 512;
  const scaledW = Math.ceil(w * LOGO_SCALE);
  const scaledH = Math.ceil(h * LOGO_SCALE);
  peachBuffer = await sharp(peachBuffer)
    .resize(scaledW, scaledH)
    .extract({ left: Math.floor((scaledW - w) / 2), top: Math.floor((scaledH - h) / 2), width: w, height: h })
    .png()
    .toBuffer();

  writeFileSync(coloredSource, peachBuffer);
  console.log('Generating icons from:', coloredSource, '(logo scaled', (LOGO_SCALE * 100 - 100).toFixed(0) + '% larger)');

  await iconGen(coloredSource, outDir, {
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

  // Ensure Tauri-referenced PNG filenames exist (already scaled in peachBuffer)
  await sharp(peachBuffer).resize(32, 32).png().toFile(resolve(outDir, '32x32.png'));
  await sharp(peachBuffer).resize(128, 128).png().toFile(resolve(outDir, '128x128.png'));
  await sharp(peachBuffer).resize(256, 256).png().toFile(resolve(outDir, '128x128@2x.png'));
  console.log('Created Tauri PNG icons');

  // Remove temp peach-colored source
  try {
    if (existsSync(coloredSource)) unlinkSync(coloredSource);
  } catch {}
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});


