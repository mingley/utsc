import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

function measure(code, timeout_seconds=2) {
  const result = spawnSync('python3', [fileURLToPath(new URL('./measure.py',import.meta.url))], {
    input:JSON.stringify({command:['python3','-c',code],cwd:process.cwd(),timeout_seconds}),
    encoding:'utf8',timeout:5000,
  });
  assert.equal(result.status,0,result.stderr);
  return JSON.parse(result.stdout);
}

test('measurement preserves child failure and keeps stdout/stderr separate', {skip:process.platform!=='linux'}, () => {
  const result = measure('import sys; print("out"); print("err",file=sys.stderr); sys.exit(7)');
  assert.equal(result.exit_code,7);
  assert.equal(result.timed_out,false);
  assert.equal(result.stdout,'out\n');
  assert.equal(result.stderr,'err\n');
  assert.ok(result.wall_ms>0);
  assert.ok(result.cpu_ms>=0);
  assert.ok(result.max_rss_kib>0);
});
test('timeout kills the compiler process group and reports failure', {skip:process.platform!=='linux'}, () => {
  const result = measure('import subprocess,time; subprocess.Popen(["python3","-c","import time; time.sleep(30)"]); time.sleep(30)',0.05);
  assert.equal(result.timed_out,true);
  assert.equal(result.exit_code,-9);
  assert.ok(result.wall_ms<3000);
});
