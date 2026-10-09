import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, globSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { copyFixtures } from './support/fixtures.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));
const artifacts = join(root, 'test/artifacts');
const compiler = process.env.PURS || 'purs';
const keep = process.argv.includes('--keep-output');
assert.ok(existsSync(join(root, '.spago/p')), 'Install dependencies first: spago install');
mkdirSync(artifacts, { recursive: true });
const workspace = mkdtempSync(join(tmpdir(), 'substructural-tests-'));
const commands = [];
const report = { complete: false, tools: {}, cases: [], executions: [] };

function run(command, args, { reject = false } = {}) {
  const result = spawnSync(command, args, { cwd: root, encoding: 'utf8', timeout: 60_000, maxBuffer: 16 * 1024 * 1024 });
  commands.push({ command, args, status: result.status, error: result.error?.message, stdout: result.stdout, stderr: result.stderr });
  writeFileSync(join(artifacts, 'commands.json'), JSON.stringify(commands, null, 2) + '\n');
  assert.equal(result.signal, null, `${command}: ${result.error ?? result.stderr}`);
  if (reject) assert.ok(Number.isInteger(result.status) && result.status !== 0, `Expected rejection: ${command}`);
  else assert.equal(result.status, 0, `${command}: ${result.error ?? ''}\n${result.stdout}\n${result.stderr}`);
  return result;
}

try {
  report.tools = { purs: run(compiler, ['--version']).stdout.trim(), node: process.version };
  assert.match(report.tools.purs, /^0\.15\./, `Expected PureScript 0.15.x, got ${report.tools.purs}. Run npm install or set PURS to the intended compiler.`);
  const cases = [];
  for (const family of ['compile-pass', 'compile-fail']) {
    const source = join(root, 'test', family);
    cases.push(...JSON.parse(readFileSync(join(source, 'cases.json'), 'utf8')));
    copyFixtures(source, join(workspace, family));
  }

  const modules = new Map();
  function index(paths) {
    for (const path of paths) {
      const source = readFileSync(path, 'utf8');
      const name = source.match(/^\s*module\s+([\w.]+)/m)?.[1];
      if (name) modules.set(name, { path, imports: [...source.matchAll(/^\s*import\s+([\w.]+)/gm)].map(match => match[1]) });
    }
  }
  const sources = (pattern, cwd) => globSync(pattern, { cwd }).map(path => join(cwd, path));
  index(sources('*/src/**/*.purs', join(root, '.spago/p')));
  index(sources('src/**/*.purs', root));
  index(sources('test/**/*.purs', root));
  index(sources('**/*.purs', workspace));

  const selected = new Map();
  function visit(name) {
    if (name === 'Prim' || name.startsWith('Prim.') || selected.has(name)) return;
    const item = modules.get(name);
    assert.ok(item, `Missing module ${name}; run spago install`);
    selected.set(name, item.path);
    item.imports.forEach(visit);
  }
  ['Test.Main', ...cases.map(test => test.module)].forEach(visit);
  // The regex above only discovers dependency files. PureScript itself checks
  // the graph and all types; no source-text heuristic determines a verdict.
  const graph = JSON.parse(run(compiler, ['graph', ...selected.values()]).stdout);
  function sourcesFor(name) {
    const result = new Map();
    function add(name) {
      if (name === 'Prim' || name.startsWith('Prim.') || result.has(name)) return;
      assert.ok(graph[name], `Missing graph node ${name}`);
      result.set(name, resolve(root, graph[name].path));
      graph[name].depends.forEach(add);
    }
    add(name);
    return [...result.values()];
  }

  for (const test of cases) {
    const output = join(workspace, 'typing', test.module);
    const result = run(compiler, ['compile', ...sourcesFor(test.module), '--codegen', 'corefn', '--output', output, '--json-errors'], { reject: test.expect === 'reject' });
    const errors = (result.stdout + '\n' + result.stderr).split('\n').flatMap(line => {
      try { return JSON.parse(line).errors ?? []; } catch { return []; }
    });
    if (test.expect === 'reject') {
      assert.ok(test.codes?.length, `Missing diagnostic codes for ${test.module}`);
      assert.ok(errors.some(error => test.codes.includes(error.errorCode) && error.moduleName === test.module), `Wrong rejection for ${test.module}: ${result.stdout}\n${result.stderr}`);
    } else {
      assert.equal(test.expect, 'accept');
      assert.equal(errors.length, 0);
    }
    report.cases.push({ ...test, errors: errors.map(error => ({ code: error.errorCode, module: error.moduleName, message: error.message })) });
    console.log(`${test.expect}: ${test.module}`);
  }

  const output = join(workspace, 'javascript');
  run(compiler, ['compile', ...sourcesFor('Test.Main'), '--codegen', 'js', '--output', output]);
  const entry = pathToFileURL(join(output, 'Test.Main/index.js')).href;
  const result = run(process.execPath, ['--input-type=module', '--eval', `const { main } = await import(${JSON.stringify(entry)}); main();`]);
  assert.ok(result.stdout.split('\n').includes('SUBSTRUCTURAL_JS_OK'), result.stdout);
  report.executions.push({ module: 'Test.Main', target: 'javascript', stdout: result.stdout });
  report.sourceSha256 = Object.fromEntries(sources('src/**/*', root).filter(path => /\.(purs|js)$/.test(path)).map(path => [path.slice(root.length), createHash('sha256').update(readFileSync(path)).digest('hex')]));
  report.complete = true;
  console.log(result.stdout.trim());
} finally {
  report.retainedWorkspace = keep || !report.complete ? workspace : null;
  writeFileSync(join(artifacts, 'report.json'), JSON.stringify(report, null, 2) + '\n');
  if (report.retainedWorkspace) console.log(`Retained workspace: ${workspace}`);
  else rmSync(workspace, { recursive: true, force: true });
  console.log(`Report: ${join(artifacts, 'report.json')}`);
}
