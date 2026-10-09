import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';

export function prepare({ input }) {
  const path = join(input, 'Api.rs');
  writeFileSync(path, readFileSync(path, 'utf8').replace(
    '// OWNER_SOURCE: hook.mjs inlines owner.rs here before compilation.',
    readFileSync(join(input, 'owner.rs'), 'utf8')));
}

export function check({ input, workspace, run, report }) {
  report.nativeOwner = {
    sourceSha256: createHash('sha256').update(readFileSync(join(input, 'owner.rs'))).digest('hex'),
    rcPreservedInGeneratedModes: [],
  };
  for (const mode of ['normal', 'threaded']) {
    const generated = readFileSync(join(workspace, 'rust', `LinearLab.ThreadAffinity.Main-${mode}`,
      'Purs_LinearLab_ThreadAffinity_Api', 'src', 'lib.rs'), 'utf8');
    assert.ok(generated.includes('cell: thread_rc::Rc<std::cell::RefCell<i64>>'));
    assert.ok(!/unsafe\s+impl\s+(?:Send|Sync)/.test(generated));
    report.nativeOwner.rcPreservedInGeneratedModes.push(mode);
  }
  report.rustControls = [];
  for (const [name, expected] of [['valid', null], ['send_owner', 'E0277'], ['sync_owner', 'E0277']]) {
    const binary = join(workspace, `rust-control-${name}`);
    const args = ['--edition=2021', join(input, 'rust-controls.rs'), '-o', binary, '--error-format=json'];
    if (expected) args.push('--cfg', name);
    const result = run('rustc', args, { reject: expected !== null });
    const errors = result.stderr.split('\n').flatMap(line => {
      try { const item = JSON.parse(line); return item.level === 'error' ? [item] : []; }
      catch { return []; }
    });
    if (expected) assert.ok(errors.some(error => error.code?.code === expected), result.stderr);
    else assert.equal(run(binary, []).stdout.trim(), 'RUST_THREAD_LOCAL_OWNER_OK');
    report.rustControls.push({ name, outcome: expected ? 'reject' : 'accept-and-run', codes: errors.map(error => error.code?.code).filter(Boolean) });
    console.log(`[thread-affinity] Rust control ${name}: ${expected ?? 'RUST_THREAD_LOCAL_OWNER_OK'}`);
  }
}
