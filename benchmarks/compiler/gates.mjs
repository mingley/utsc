import assert from 'node:assert/strict';

function normalized(text) {
  return text.replaceAll('\r\n', '\n');
}

export function checkSuccess(result, label) {
  assert.equal(result.timed_out, false, `${label}: timed out`);
  assert.equal(result.exit_code, 0, `${label}: failed\n${result.stdout}\n${result.stderr}`);
  assert.equal(result.stdout, '', `${label}: unexpected stdout`);
  assert.equal(result.stderr, '', `${label}: unexpected stderr`);
}

export function checkDiagnostics(actual, expected, label, expectedExitCode = expected.exit_code) {
  assert.equal(expected.timed_out, false, 'The reference must complete');
  assert.ok(expected.exit_code > 0, 'The reference must reject invalid input');
  assert.match(expected.stdout + expected.stderr, /error TS\d+:/, 'The reference must issue a TypeScript diagnostic');
  assert.equal(actual.timed_out, false, `${label}: timed out`);
  assert.ok([1, 2].includes(expectedExitCode), 'Diagnostic exit adapters must specify a documented failure status');
  assert.equal(actual.exit_code, expectedExitCode, `${label}: wrong failure status`);
  assert.equal(normalized(actual.stdout), normalized(expected.stdout), `${label}: stdout diagnostics differ`);
  assert.equal(normalized(actual.stderr), normalized(expected.stderr), `${label}: stderr diagnostics differ`);
}

export function checkArtifacts(actual, expected, label) {
  assert.ok(expected.length > 0, 'Reference emission must produce files');
  assert.equal(actual.length, expected.length, `${label}: emitted file count differs`);
  for (let i=0; i<expected.length; i++) {
    assert.deepEqual(actual[i], expected[i], `${label}: emitted file ${expected[i].path} differs`);
  }
}

export function summarize(samples) {
  assert.ok(samples.length >= 3, 'At least three measured samples are required');
  const distribution = key => {
    const values = samples.map(sample => sample[key]).sort((a,b)=>a-b);
    assert.ok(values.every(value=>Number.isFinite(value) && value >= 0));
    const mid = Math.floor(values.length / 2);
    return { median: values.length % 2 ? values[mid] : (values[mid-1]+values[mid])/2, min: values[0], max: values.at(-1) };
  };
  return { wall_ms: distribution('wall_ms'), cpu_ms: distribution('cpu_ms'), max_rss_kib: distribution('max_rss_kib') };
}

export function rotated(items, round) {
  const start = round % items.length;
  return [...items.slice(start), ...items.slice(0, start)];
}
