import { readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { format, resolveConfig } from 'prettier';

function findStyleFiles(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = join(directory, entry.name);

    if (entry.isDirectory()) {
      return findStyleFiles(path);
    }

    return /\.(vue|scss)$/.test(path) ? [path] : [];
  });
}

function separateStyleRules(source) {
  const lineEnding = source.includes('\r\n') ? '\r\n' : '\n';
  return source.replace(/^([ \t]*}[ \t]*\r?\n)(?=[ \t]*[^\s}])/gm, `$1${lineEnding}`);
}

function formatSfcSpacing(original) {
  original = original.replace(
    /(<style\b[^>]*>)([\s\S]*?)(<\/style>)/g,
    (_match, opening, body, closing) => opening + separateStyleRules(body) + closing,
  );

  const lineEnding = original.includes('\r\n') ? '\r\n' : '\n';
  const lines = original.split(/\r?\n/);
  const formatted = [];

  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    const openingTag = /^\s*<(script|style)\b[^>]*>\s*$/.test(line);
    const closingTag = /^\s*<\/(script|style)>\s*$/.test(line);

    if (openingTag) {
      formatted.push(line);

      while (index + 1 < lines.length && lines[index + 1].trim() === '') {
        index += 1;
      }

      formatted.push('');
      continue;
    }

    if (closingTag) {
      while (formatted.length > 0 && formatted.at(-1).trim() === '') {
        formatted.pop();
      }

      formatted.push('');
      formatted.push(line);
      continue;
    }

    formatted.push(line);
  }

  return formatted.join(lineEnding);
}

const check = process.argv.includes('--check');
for (const file of findStyleFiles('src')) {
  const original = readFileSync(file, 'utf8');
  const formatted = check
    ? await format(original, { ...(await resolveConfig(file)), filepath: file })
    : original;
  const updated = file.endsWith('.vue')
    ? formatSfcSpacing(formatted)
    : separateStyleRules(formatted);
  if (updated !== original) {
    if (check) {
      console.error(`Formatting or SFC spacing differs in ${file}`);
      process.exitCode = 1;
    } else {
      writeFileSync(file, updated);
    }
  }
}
