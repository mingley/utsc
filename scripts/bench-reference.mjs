// In-process reference scanner microbenchmark. This is NOT a compiler benchmark.
import { readFileSync } from 'node:fs';
import { performance } from 'node:perf_hooks';
import { createScanner, SyntaxKind } from 'typescript/unstable/ast';
let sink = 0;
function scan(source) {
  const scanner = createScanner(false, undefined, source);
  let checksum = 0;
  while (true) {
    const kind = scanner.scan();
    checksum += scanner.getTokenEnd() + kind;
    if (kind === SyntaxKind.EndOfFile) return checksum;
  }
}
console.log(`Node ${process.version}, TypeScript 7.0.2; reference computes cooked token values that utsc does not yet compute.`);
for (const [name, fixture] of [['ascii','representative'], ['unicode','unicode']]) {
  const source = readFileSync(`tests/fixtures/${fixture}.ts`, 'utf8').repeat(4096);
  const bytes = Buffer.byteLength(source);
  const warmup = performance.now();
  do { sink = scan(source); } while (performance.now() - warmup < 500);
  const samples = [];
  for (let i = 0; i < 7; i++) {
    const start = performance.now();
    let iterations = 0;
    do { sink = scan(source); iterations++; } while (performance.now() - start < 500);
    samples.push(bytes * iterations / ((performance.now() - start) / 1000) / 1048576);
  }
  samples.sort((a,b)=>a-b);
  console.log(`${name}: ${samples[3].toFixed(1)} MiB/s median (range ${samples[0].toFixed(1)}–${samples[6].toFixed(1)}); ${bytes} bytes, checksum=${sink}; 7 × ≥500 ms; scanner only`);
}
