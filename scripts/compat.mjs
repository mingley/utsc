import assert from 'node:assert/strict';
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { resolve, extname, dirname, join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { createScanner, SyntaxKind } from 'typescript/unstable/ast';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const version = require('typescript/package.json').version;
assert.equal(version, '7.0.2', 'Update the compatibility target deliberately');
const names = new Map();
for (const [name, value] of Object.entries(SyntaxKind)) {
  if (typeof value === 'number' && !names.has(value)) names.set(value, name);
}
const binary = process.env.UTSC_BIN ?? resolve('target/debug/utsc');
if (!process.env.UTSC_BIN) {
  const build = spawnSync('cargo', ['build', '--locked'], { stdio: 'inherit' });
  if (build.error) throw build.error;
  if (build.status !== 0) process.exit(build.status ?? 1);
}

const cases = [];
const add = (name, source) => cases.push({ name, source });
add('empty', '');
add('representative', readFileSync('tests/fixtures/representative.ts', 'utf8'));
add('all keywords', Object.entries(SyntaxKind).filter(([name, value]) => typeof value === 'number' && name.endsWith('Keyword')).map(([name]) => name.slice(0, -7).toLowerCase()).join(' '));
add('operators', '{}()[] . ... ; , < <= << <<= > >= >> >>> >>= >>>= == != === !== => + - * ** / % ++ -- & | ^ ! ~ && || ? : @ ?? = += -= *= **= /= %= &= |= ||= &&= ??= ^= ?. ?.3');
add('merge conflicts', '<<<<<<< ours\nconst x = 1;\n||||||| base\nconst x = 0;\n=======\nconst x = 2;\n>>>>>>> theirs\n');
add('comments', '// comment 😀\r\n/** doc */ /* multiline\n comment */\n');
add('shebang', '#!/usr/bin/env node\nexport {}');
add('identifiers', 'café 变量 𐐀name a\u200cb a\u200db $ _ constructor __proto__ #field #变量');
add('identifier escapes', String.raw`\u0069f \u{61}wait f\u006fo \u{10400}name #\u0061bc \u{24}foo \u{5f}bar`);
add('strings', `'hello' "😀" 'café' 'line\u2028separator' "paragraph\u2029separator" '\\x41' '\\u0041' '\\u{1F600}' 'a\\\r\nb'`);
add('templates', '`hello😀` `escaped\\`backtick` `raw\ntext` `head${value');
add('numbers', '0 1 42 3.14 .5 5. 1e4 1e+4 1e-4 1_000 1.2_3e4_5 0xff 0XFF 0b101 0B10 0o77 0O77 42n 0xffn 0b10n 0o77n');
add('recovery', '0x 0b 0o 1e+ 1__0 1_ 123abc 0b102 0o78 0xg "unterminated\n/* unclosed');
add('legacy numerics', '00 0123 078 099.3 08e2 01.2 0_1 0_1n 1.2n 1e3n');
add('private identifiers', '#a #变量 #\\u0061bc');
add('replacement marker', '\ufffdtext');
add('whitespace', '\t\v\f \u00a0\u2000\u2001\u2002\u2003\u2004\u2005\u2006\u2007\u2008\u2009\u200a\u200b\u202f\u205f\u3000\ufeff\r\n\r\n');
for (const text of ["'", '"', '`', '/*', "'\\", String.raw`'\xQ'`, String.raw`'\u12Z'`, String.raw`'\u{110000}'`, '\\uZZZZ', '\\u0020', '\\u{110000}']) add(`malformed ${JSON.stringify(text)}`, text);

// Fixed seed and no ambient randomness: failures are reproducible.
let seed = 0x735463;
function random(n) {
  seed ^= seed << 13; seed ^= seed >>> 17; seed ^= seed << 5;
  return (seed >>> 0) % n;
}
const atoms = ['let', 'const', 'interface', 'satisfies', 'x', 'type', '变量', '𐐀', 'café', ' ', '\t', '\n', '\r\n', '0', '1', '12', '0xff', '0b01', '0o77', '9n', '1_000', '.5', 'e+', 'e-', 'n', '_', '+', '-', '=', '*', '/', '%', '&', '|', '^', '!', '~', '?', '?.', '??=', ':', ';', ',', '.', '...', '(', ')', '{', '}', '[', ']', '<', '>', '@', '"', "'", '`', '\\', '$', '//a\n', '/*x*/', '#private', '\\u0061', '😀'];
for (let i = 0; i < 4000; i++) {
  let source = '';
  for (let j = 0, n = 1 + random(30); j < n; j++) source += atoms[random(atoms.length)];
  add(`seed-0x735463-${i}`, source);
}

// Exercise every generated Unicode range boundary, for both start and part.
// Explicit upstream whitespace divergences are tested separately below.
const divergentCodePoints = new Set([0x85, 0x1680, 0x1685, 0x2028, 0x2029]);
const boundaries = new Set();
for (const match of readFileSync('src/unicode.rs', 'utf8').matchAll(/\(0x([0-9a-f]+), 0x([0-9a-f]+)\)/g)) {
  for (const edge of [parseInt(match[1], 16), parseInt(match[2], 16)]) {
    for (const cp of [edge-1, edge, edge+1]) {
      if (cp >= 0x80 && cp <= 0x10ffff && !(cp >= 0xd800 && cp <= 0xdfff) && !divergentCodePoints.has(cp)) boundaries.add(cp);
    }
  }
}
for (const cp of [...boundaries].sort((a,b)=>a-b)) {
  const ch = String.fromCodePoint(cp);
  add(`unicode-start-U+${cp.toString(16)}`, ch + 'x');
  add(`unicode-part-U+${cp.toString(16)}`, 'x' + ch);
}

// Whole real-world declaration/implementation files, without slicing tokens.
const platformRoot = dirname(require.resolve(`@typescript/typescript-${process.platform}-${process.arch}/package.json`));
for (const path of [
  join(platformRoot, 'lib/lib.es5.d.ts'),
  join(platformRoot, 'lib/lib.es2015.collection.d.ts'),
  join(platformRoot, 'lib/lib.es2020.bigint.d.ts'),
  'node_modules/typescript/dist/ast/scanner.d.ts',
  'node_modules/typescript/dist/ast/scanner.js',
]) add(path, readFileSync(path, 'utf8'));

function addCorpus(path) {
  if (statSync(path).isDirectory()) {
    for (const entry of readdirSync(path).sort()) if (entry !== '.git' && entry !== 'node_modules' && entry !== 'target') addCorpus(resolve(path, entry));
  } else if (['.ts', '.tsx', '.js', '.jsx', '.mts', '.cts', '.mjs', '.cjs'].includes(extname(path))) {
    add(path, readFileSync(path, 'utf8'));
  }
}
const args = process.argv.slice(2);
if (args.length) {
  assert.equal(args[0], '--corpus', 'Usage: node scripts/compat.mjs [--corpus paths...]');
  assert.ok(args.length > 1, '--corpus requires a path');
  args.slice(1).forEach(addCorpus);
}
function oracle(source) {
  const scanner = createScanner(false, undefined, source);
  const tokens = [];
  for (let steps = 0; steps <= source.length + 1; steps++) {
    const kind = scanner.scan();
    const start = scanner.getTokenStart(), end = scanner.getTokenEnd();
    tokens.push({ kind: names.get(kind), start, end, text: scanner.getTokenText(), unterminated: scanner.isUnterminated() });
    if (kind === SyntaxKind.EndOfFile) return tokens;
    assert.ok(end > start, `Upstream scanner did not advance at ${start}; ${JSON.stringify(source)}`);
  }
  throw new Error('Upstream scanner exceeded its token budget');
}
const input = cases.map(test => JSON.stringify(test)).join('\n') + '\n';
const run = spawnSync(binary, ['scan', '--jsonl'], { input, encoding: 'utf8', maxBuffer: 128 * 1024 * 1024, timeout: 60_000 });
if (run.error) throw run.error;
assert.ok(run.status === 0 || run.status === 1, `Native scanner failed: ${run.stderr}; signal=${run.signal}`);
const responses = run.stdout.trimEnd().split('\n').map(line => JSON.parse(line));
assert.equal(responses.length, cases.length, 'The native process must respond to every case');
let tokenCount = 0;
let failureCount = 0;
for (let i = 0; i < cases.length; i++) {
  const test = cases[i], response = responses[i];
  try {
    assert.equal(response.name, test.name);
    assert.equal(response.tokens.map(token=>token.text).join(''), test.source, 'Native tokens must preserve all input');
    const actual = response.tokens.map(({kind,start,end,text,unterminated})=>({kind,start,end,text,unterminated}));
    const expected = oracle(test.source);
    assert.equal(actual.length, expected.length, `Token count differs: native=${actual.length}, reference=${expected.length}`);
    for (let j = 0; j < actual.length; j++) assert.deepEqual(actual[j], expected[j], `Token ${j}`);
    tokenCount += actual.length;
  } catch (error) {
    failureCount++;
    if (failureCount <= 8) console.error(`FAIL ${test.name}: ${JSON.stringify(test.source.slice(0, 200))}\n${error.message.slice(0, 3500)}`);
  }
}
// Explicit upstream API divergence: bare '#' returns an empty token in 7.0.2.
// Assert the upstream observation rather than hiding it behind a skip/xfail.
const hashOracle = createScanner(false, undefined, '#');
assert.equal(hashOracle.scan(), SyntaxKind.PrivateIdentifier);
assert.equal(hashOracle.getTokenEnd(), 0);
const hash = spawnSync(binary, ['scan', '--stdin'], { input: '#', encoding: 'utf8', timeout: 5000 });
assert.equal(hash.status, 1);
assert.equal(JSON.parse(hash.stdout).tokens[0].text, '#');

// The API's ogham constant is U+1685, not ECMAScript whitespace U+1680.
const oghamOracle = createScanner(false, undefined, '\u1680');
assert.equal(oghamOracle.scan(), SyntaxKind.Unknown);
const ogham = spawnSync(binary, ['scan', '--stdin'], { input: '\u1680', encoding: 'utf8', timeout: 5000 });
assert.equal(ogham.status, 0);
assert.equal(JSON.parse(ogham.stdout).tokens[0].kind, 'WhitespaceTrivia');
for (const separator of ['\u2028', '\u2029']) {
  const separatorOracle = createScanner(false, undefined, separator);
  assert.equal(separatorOracle.scan(), SyntaxKind.EndOfFile);
  const actual = spawnSync(binary, ['scan', '--stdin'], { input: separator, encoding: 'utf8', timeout: 5000 });
  assert.equal(actual.status, 0);
  assert.equal(JSON.parse(actual.stdout).tokens[0].kind, 'NewLineTrivia');
}
for (const [ch, expectedKind, expectedStatus] of [['\u0085', 'Unknown', 1], ['\u1685', 'Identifier', 0]]) {
  const scanner = createScanner(false, undefined, ch);
  const oracleKind = scanner.scan();
  assert.equal(oracleKind, ch === '\u0085' ? SyntaxKind.EndOfFile : SyntaxKind.WhitespaceTrivia);
  const native = spawnSync(binary, ['scan', '--stdin'], { input: ch, encoding: 'utf8', timeout: 5000 });
  assert.equal(native.status, expectedStatus);
  assert.equal(JSON.parse(native.stdout).tokens[0].kind, expectedKind);
}
console.log(`TypeScript ${version}: ${cases.length - failureCount}/${cases.length} exact raw-scanner cases passed; ${tokenCount} matched tokens. 6 upstream API divergences verified separately ('#', U+0085, U+1680, U+1685, U+2028, U+2029).`);
console.log('Compared: kind, UTF-16 start/end, spelling, unterminated. Not compared: cooked values, flags, diagnostics, parser-led rescans, parser/checker/emitter behavior.');
if (failureCount) process.exit(1);
