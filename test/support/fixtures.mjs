import { cpSync, readdirSync, renameSync } from 'node:fs';
import { join } from 'node:path';

// Fixtures stay outside Spago's **/*.purs glob until a dedicated test selects
// them. Adjacent native/JS foreign files are copied without modification.
export function copyFixtures(source, destination) {
  cpSync(source, destination, { recursive: true });
  function restore(directory) {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const path = join(directory, entry.name);
      if (entry.isDirectory()) restore(path);
      else if (entry.name.endsWith('.purs.fixture')) {
        renameSync(path, path.slice(0, -'.fixture'.length));
      }
    }
  }
  restore(destination);
}
