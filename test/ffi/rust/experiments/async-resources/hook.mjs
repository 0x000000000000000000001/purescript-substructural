import assert from 'node:assert/strict';
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

export function prepare({ input }) {
  const wrapper = join(input, 'Native.rs');
  const source = readFileSync(wrapper, 'utf8');
  const include = 'include!("native-core.rs");';
  assert.equal(source.split(include).length, 2);
  writeFileSync(wrapper, source.replace(include, readFileSync(join(input, 'native-core.rs'), 'utf8')));
}

export function check({ input, workspace, artifacts, run, report, compiled, backend }) {
  const binary = join(workspace, 'native-controls');
  const source = join(input, 'native-controls.rs');
  run('rustc', ['--edition=2021', source, '-o', binary]);
  const result = run(binary, []);
  assert.ok(result.stdout.startsWith('NATIVE_ASYNC_CONTROLS_OK races=32 '));
  const rejected = run('rustc', ['--edition=2021', source, '--cfg', 'require_unpin', '--error-format=json', '-o', binary], { reject: true });
  const rustErrors = rejected.stderr.split('\n').flatMap(line => {
    try { const error = JSON.parse(line); return error.level === 'error' ? [error] : []; } catch { return []; }
  });
  assert.ok(rustErrors.some(error => error.code?.code === 'E0277' && error.message.includes('PhantomPinned')));
  report.nativeControls = { outcome: 'passed-without-PureScript', stdout: result.stdout, pinRejection: 'E0277' };

  // The simple Aff program has no dependency on Native.rs. Its normal-mode
  // rejection isolates the currently installed port from the wrapper design.
  const output = join(workspace, 'normal-aff-probe');
  run(process.execPath, ['--stack-size=65536', backend, '--source', compiled.get('LinearLab.AsyncResources.AffNormal'),
    '--out', output, '--main', 'LinearLab.AsyncResources.AffNormal']);
  const normal = run('cargo', ['check', '--offline', '--message-format=json'], { cwd: output, reject: true });
  const diagnostics = normal.stdout.split('\n').flatMap(line => {
    try {
      const value = JSON.parse(line);
      return value.reason === 'compiler-message' && value.message.level === 'error'
        ? [{ code: value.message.code?.code, message: value.message.message,
          files: value.message.spans.map(span => span.file_name) }] : [];
    } catch { return []; }
  });
  assert.ok(diagnostics.some(error => ['E0277', 'E0631'].includes(error.code)
    && error.files.some(file => file.includes('Purs_Effect_Aff'))), JSON.stringify(diagnostics));
  report.normalMode = { outcome: 'existing-Aff-port-build-rejected',
    note: 'Normal mode is not an end-to-end success; no production compiler or port was changed.',
    diagnostics };
  writeFileSync(join(artifacts, 'normal-aff-failure.json'), JSON.stringify(report.normalMode, null, 2) + '\n');
  console.log('[async-resources] Native controls passed; normal Aff port rejection recorded separately.');
}
