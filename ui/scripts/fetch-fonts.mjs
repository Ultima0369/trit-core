// One-shot: fetch Google Fonts woff2 — latin-basic subset only.
// - Inter & JetBrains Mono are variable fonts → 1 file each, declared with a
//   font-weight range so the browser interpolates.
// - DM Mono is static → 3 separate files (300/400/500).
// Run from ui/. Usage: node scripts/fetch-fonts.mjs
import { mkdirSync, writeFileSync, readdirSync, unlinkSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const FONTS_DIR = join(__dirname, '..', 'src', 'assets', 'fonts');
mkdirSync(FONTS_DIR, { recursive: true });
for (const f of readdirSync(FONTS_DIR)) {
  if (f.endsWith('.woff2')) { try { unlinkSync(join(FONTS_DIR, f)); } catch {} }
}

const UA = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36';
const CSS_URL = 'https://fonts.googleapis.com/css2?family=DM+Mono:ital,wght@0,300;0,400;0,500&family=Inter:wght@300;400;500;600&family=JetBrains+Mono:wght@400;500&display=swap';

async function fetchText(url) {
  const res = await fetch(url, { headers: { 'User-Agent': UA } });
  if (!res.ok) throw new Error(`HTTP ${res.status} fetching ${url}`);
  return res.text();
}
async function fetchBuf(url) {
  const res = await fetch(url, { headers: { 'User-Agent': UA } });
  if (!res.ok) throw new Error(`HTTP ${res.status} fetching ${url}`);
  return Buffer.from(await res.arrayBuffer());
}

const css = await fetchText(CSS_URL);

// Collect latin-basic @font-face blocks grouped by (family, style).
// A variable font shares one URL across all weights → dedupe by URL.
const byKey = new Map(); // key: "Family|style" → { urls: Set, weights: [] }
const faceRe = /@font-face\s*\{([^}]*)\}/g;
let m;
while ((m = faceRe.exec(css)) !== null) {
  const b = m[1];
  const ur = b.match(/unicode-range:\s*([^;]+)/)?.[1] ?? '';
  if (!ur.startsWith('U+0000-00FF')) continue;
  const family = b.match(/font-family:\s*'([^']+)'/)?.[1];
  const weight = b.match(/font-weight:\s*(\d+)/)?.[1];
  const style = b.match(/font-style:\s*(\w+)/)?.[1] ?? 'normal';
  const url = b.match(/src:\s*url\(([^)]+)\)/)?.[1];
  if (!family || !weight || !url) continue;
  const key = `${family}|${style}`;
  if (!byKey.has(key)) byKey.set(key, { family, style, weights: [], urls: new Map() });
  const e = byKey.get(key);
  e.weights.push(weight);
  if (!e.urls.has(url)) e.urls.set(url, null); // url → filename, assigned below
}

const slug = (s) => s.toLowerCase().replace(/\s+/g, '-');
const faces = [];
for (const [, e] of byKey) {
  const urls = [...e.urls.keys()];
  const variableLike = urls.length === 1 && e.weights.length > 1;
  if (variableLike) {
    // One file covers all weights (variable font).
    const name = `${slug(e.family)}.woff2`;
    const buf = await fetchBuf(urls[0]);
    writeFileSync(join(FONTS_DIR, name), buf);
    faces.push({ family: e.family, style: e.style, weights: e.weights, file: name });
    console.log(`✓ ${name} (${buf.length} bytes) — variable, weights ${Math.min(...e.weights)}–${Math.max(...e.weights)}`);
  } else {
    // Static font: one file per weight.
    for (const url of urls) {
      const w = e.weights[urls.indexOf(url)];
      const name = `${slug(e.family)}-${w}${e.style === 'italic' ? '-italic' : ''}.woff2`;
      const buf = await fetchBuf(url);
      writeFileSync(join(FONTS_DIR, name), buf);
      faces.push({ family: e.family, style: e.style, weights: [w], file: name });
      console.log(`✓ ${name} (${buf.length} bytes) — static, weight ${w}`);
    }
  }
}

// Emit the @font-face CSS snippet for review / paste into aurora.css.
const cssOut = faces.map(f => {
  const weightDecl = f.weights.length > 1
    ? `font-weight: ${Math.min(...f.weights)} ${Math.max(...f.weights)};`
    : `font-weight: ${f.weights[0]};`;
  return `@font-face {
  font-family: '${f.family}';
  font-style: ${f.style};
  ${weightDecl}
  font-display: swap;
  src: url('./assets/fonts/${f.file}') format('woff2');
  unicode-range: U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC, U+0304, U+0308, U+0329, U+2000-206F, U+20AC, U+2122, U+2191, U+2193, U+2212, U+2215, U+FEFF, U+FFFD;
}`;
}).join('\n\n');
writeFileSync(join(FONTS_DIR, '_font-face-snippet.css'), cssOut + '\n');
console.log(`\nWrote ${faces.length} @font-face blocks → src/assets/fonts/_font-face-snippet.css`);
