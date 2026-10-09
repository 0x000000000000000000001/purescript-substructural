import { copyFileSync, readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

export function prepare({ input, report }) {
  const root = fileURLToPath(new URL('../../../../../', import.meta.url));
  report.librarySources = {};
  for (const [source, target] of [
    ['src/Data/Function/Sub.purs', 'Sub.purs'], ['src/Data/Array/Unique.purs', 'Unique.purs'],
    ['src/Data/Function/Sub.rs', 'Sub.rs'], ['src/Data/Array/Unique.rs', 'Unique.rs'],
    ['src/Data/Function/Sub/Aff.purs', 'Aff.purs'],
    ['src/Data/Function/Sub/Aff/Unsafe.purs', 'AffUnsafe.purs'],
    ['src/Data/Function/Sub/Aff/Do.purs', 'AffDo.purs'],
    ['test/Data/Function/Sub.purs', 'TestSub.purs'], ['test/Data/Array/Unique.purs', 'TestUnique.purs'],
    ['test/Data/Function/Sub/Aff/Do.purs', 'TestAffDo.purs'],
    ['test/Data/Assert.purs', 'Assert.purs'],
  ]) {
    const path = join(root, source);
    copyFileSync(path, join(input, target));
    report.librarySources[source] = createHash('sha256').update(readFileSync(path)).digest('hex');
  }
}
