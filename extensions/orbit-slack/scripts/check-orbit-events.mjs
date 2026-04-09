import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { tmpdir } from 'node:os';
import { execFileSync } from 'node:child_process';

const extensionRoot = resolve(import.meta.dirname, '..');
const workspaceRoot = resolve(extensionRoot, '..', '..');
const generatedFile = resolve(extensionRoot, 'src/generated/orbit-events.ts');
const tempDir = mkdtempSync(join(tmpdir(), 'orbit-events-check-'));
const tempFile = join(tempDir, 'orbit-events.ts');

try {
  execFileSync(
    'cargo',
    [
      'run',
      '-p',
      'orbit-events',
      '--bin',
      'export-typescript',
      '--',
      tempFile,
    ],
    {
      cwd: workspaceRoot,
      stdio: 'pipe',
    }
  );

  const expected = readFileSync(tempFile, 'utf8');
  const actual = readFileSync(generatedFile, 'utf8');
  if (actual !== expected) {
    process.stderr.write(
      [
        'Generated Orbit event bindings are stale.',
        'Run `npm run sync:orbit-events` in extensions/orbit-slack and commit the updated file.',
        '',
      ].join('\n')
    );
    process.exit(1);
  }
} finally {
  rmSync(tempDir, { recursive: true, force: true });
}
