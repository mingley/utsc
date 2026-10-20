import assert from 'node:assert/strict';
import test from 'node:test';
import { checkArtifacts, checkDiagnostics, checkSuccess, rotated, summarize } from './gates.mjs';

const success = { exit_code: 0, timed_out: false, stdout: '', stderr: '' };
const invalid = { ...success, exit_code: 2, stdout: 'src/index.ts(1,14): error TS2322: Type \'string\' is not assignable to type \'number\'.\n' };

test('a compiler that accepts invalid input cannot enter the rankings', () => {
  assert.throws(()=>checkDiagnostics(success, invalid, 'silently accepts errors'));
});
test('missing, moved and changed diagnostics are rejected', () => {
  for (const stdout of ['', invalid.stdout.replace('2322','2339'), invalid.stdout.replace('(1,14)','(2,14)'), invalid.stdout.replace('string','boolean')]) {
    assert.throws(()=>checkDiagnostics({...invalid, stdout}, invalid, 'diagnostic mismatch'));
  }
  assert.doesNotThrow(()=>checkDiagnostics({...invalid, stdout:invalid.stdout.replaceAll('\n','\r\n')}, invalid, 'line endings'));
});
test('crashes, timeouts, and noisy successful invocations are rejected', () => {
  for (const result of [{...success, exit_code:-11}, {...success,timed_out:true}, {...success,stdout:'warning'}]) assert.throws(()=>checkSuccess(result, 'broken compiler'));
});
test('omitted, extra, and changed outputs fail the emit gate', () => {
  const expected = [{path:'index.js', sha256:'abc'}, {path:'index.d.ts',sha256:'def'}];
  assert.throws(()=>checkArtifacts([], expected, 'no emitter'));
  assert.throws(()=>checkArtifacts(expected.slice(0,1), expected, 'missing declarations'));
  assert.throws(()=>checkArtifacts([...expected, {path:'extra.js',sha256:'123'}], expected, 'extra output'));
  assert.throws(()=>checkArtifacts([{path:'index.js',sha256:'bad'},expected[1]], expected, 'wrong JS'));
  assert.doesNotThrow(()=>checkArtifacts(expected, expected, 'correct'));
});
test('sample statistics handle even/odd counts and retain all observations', () => {
  const sample = wall_ms=>({wall_ms,cpu_ms:wall_ms*2,max_rss_kib:wall_ms*10});
  assert.deepEqual(summarize([sample(100),sample(2),sample(1)]).wall_ms, {median:2,min:1,max:100});
  assert.equal(summarize([sample(1),sample(2),sample(3),sample(100)]).wall_ms.median, 2.5);
  assert.throws(()=>summarize([sample(1)]));
  assert.throws(()=>summarize([sample(NaN),sample(1),sample(2)]));
});
test('rotation gives each competitor each position', () => {
  assert.deepEqual([0,1,2].map(i=>rotated(['tsc','tsc-js','ts-rust'],i)[0]), ['tsc','tsc-js','ts-rust']);
});

test('legacy diagnostic exit-code adapter is explicit and does not relax diagnostics', () => {
  const current = {...invalid, exit_code:1};
  assert.throws(()=>checkDiagnostics(invalid,current,'no implicit adapter'));
  assert.doesNotThrow(()=>checkDiagnostics(invalid,current,'declared legacy exit status',2));
  assert.throws(()=>checkDiagnostics({...invalid,stdout:''},current,'missing diagnostic',2));
  assert.throws(()=>checkDiagnostics(success,current,'success cannot be adapted',0));
});
