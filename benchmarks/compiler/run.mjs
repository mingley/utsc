import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { accessSync, constants, cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import os from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { checkArtifacts, checkDiagnostics, checkSuccess, rotated, summarize } from './gates.mjs';
import { createWorkload } from './workload.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, '../..');
const require = createRequire(import.meta.url);
const rootRequire = createRequire(join(repo, 'package.json'));
assert.equal(process.platform, 'linux', 'Compiler resource measurements currently require Linux and Python 3');
const args = process.argv.slice(2);
let samples = 5, modules = 256, utscBinary;
for (let i = 0; i < args.length; i++) {
  if (args[i] === '--samples') samples = Number(args[++i]);
  else if (args[i] === '--modules') modules = Number(args[++i]);
  else if (args[i] === '--utsc-bin') utscBinary = resolve(args[++i]);
  else throw new Error('Usage: npm run bench:compiler -- [--samples 3..30] [--modules 1..2048] [--utsc-bin path]');
}
assert.ok(Number.isInteger(samples) && samples >= 3 && samples <= 30, '--samples must be 3..30');
assert.ok(Number.isInteger(modules) && modules >= 1 && modules <= 2048, '--modules must be 1..2048');
const digest = data=>createHash('sha256').update(data).digest('hex');
const packageRoot = (req, name)=>dirname(req.resolve(`${name}/package.json`));
const platform = `${process.platform}-${process.arch}`;
let typeScriptRoot, jsRoot, rustRoot;
try {
  typeScriptRoot = packageRoot(rootRequire, `@typescript/typescript-${platform}`);
  jsRoot = packageRoot(require, 'typescript-js');
  rustRoot = packageRoot(require, `@tsc-rs/${platform}`);
} catch (error) {
  throw new Error('Install root dependencies and run npm ci --prefix benchmarks/compiler --ignore-scripts before benchmarking', { cause: error });
}
const competitors = [
  { id: 'tsc', diagnostic_exit_code:1, package: 'typescript', package_version: '7.0.2', command: [join(typeScriptRoot, 'lib/tsc')], entry: typeScriptRoot },
  { id: 'tsc-js', diagnostic_exit_code:2, package: 'typescript', package_version: '6.0.3', command: [process.execPath, join(jsRoot, 'lib/tsc.js')], entry: jsRoot },
  { id: 'ts-rust', diagnostic_exit_code:2, package: 'tsc-rs', package_version: '0.7.0', command: [join(rustRoot, 'lib/tsc')], entry: packageRoot(require, 'tsc-rs') },
];
for (const compiler of competitors) {
  const pkg = JSON.parse(readFileSync(join(compiler.entry, 'package.json'), 'utf8'));
  assert.equal(pkg.version, compiler.package_version, `Unexpected ${compiler.id} package`);
  accessSync(compiler.command[0], constants.X_OK);
  const version = spawnSync(compiler.command[0], [...compiler.command.slice(1), '--version'], { encoding: 'utf8', timeout: 10_000 });
  assert.equal(version.status, 0, `${compiler.id}: cannot read version: ${version.stderr}`);
  compiler.reported_version = version.stdout.trim();
}
const unavailable = [];
if (utscBinary) {
  const version = spawnSync(utscBinary, ['--version'], { encoding: 'utf8', timeout: 10_000 });
  assert.equal(version.status, 0, 'Cannot read candidate utsc version');
  if (version.stdout.includes('scanner milestone') || version.stdout.includes('compilation unsupported')) {
    unavailable.push({ id:'utsc', status:'not_implemented', reported_version:version.stdout.trim(), reason:'Scanner CLI cannot type-check or emit; no timing or ranking is produced.' });
  } else competitors.push({ id:'utsc', diagnostic_exit_code:1, package:'utsc', reported_version:version.stdout.trim(), command:[utscBinary] });
} else unavailable.push({ id:'utsc', status:'not_implemented', reason:'No compiler entry point yet. Use --utsc-bin only when compilation is implemented; candidate must pass the same gates.' });

const destination = join(repo, 'target/compiler-bench');
mkdirSync(destination, { recursive:true });
const runRoot = mkdtempSync(join(destination, 'run-'));
const workload = createWorkload(runRoot, join(typeScriptRoot,'lib'), modules);
const report = {
  schema_version:1, created_at:new Date().toISOString(),
  purpose:'Synthetic compiler baselines; not a full conformance suite or production benchmark',
  methodology:{
    samples, warmup_invocations_per_compiler_after_reference:{check:1,emit:1}, extra_reference_invocations:'one emit to obtain artifacts and three invalid-input diagnostic probes', order:'rotating competitors each round',
    timing:'fresh compiler processes, warm filesystem caches; process startup included; cleanup and artifact hashing excluded',
    memory:'Linux RUSAGE_CHILDREN maximum resident set (KiB); largest child process, not simultaneous process-tree sum',
    diagnostic_exit_adapters:'tsc 7 returns 1; tsc 6 and ts-rust return 2 for diagnostic failure. Raw statuses are retained; utsc must use tsc 7 status 1.',
    correctness:'exact diagnostics on three invalid probes including the final module in the complete project graph, no diagnostics on valid input, byte-identical JS/declarations/maps on emit; revalidated every sample',
    shared_libraries:'TypeScript 7.0.2 es5 + decorators + legacy decorators, explicit files, noLib, skipLibCheck=false',
  },
  host:{ platform:os.platform(), arch:os.arch(), kernel:os.release(), cpu:os.cpus()[0]?.model, available_parallelism:os.availableParallelism(), node:process.version },
  git_revision:spawnSync('git',['rev-parse','HEAD'],{cwd:repo,encoding:'utf8'}).stdout.trim(),
  git_dirty:spawnSync('git',['status','--porcelain=v1'],{cwd:repo,encoding:'utf8'}).stdout.length>0,
  lockfiles:{ root:digest(readFileSync(join(repo,'package-lock.json'))), benchmark:digest(readFileSync(join(here,'package-lock.json'))) },
  harness_sha256:digest(['run.mjs','workload.mjs','gates.mjs','measure.py'].map(path=>path+'\0'+readFileSync(join(here,path),'utf8')).join('\0')),
  workload, reference_diagnostics:{}, compilers:unavailable,
};
function save() {
  const path = join(runRoot,'report.json');
  writeFileSync(path, JSON.stringify(report,null,2)+'\n');
  return path;
}
function measure(compiler, project, mode) {
  const flags = ['--pretty','false','--project','tsconfig.json', ...(mode === 'check' ? ['--noEmit'] : [])];
  const request = { command:[...compiler.command, ...flags], cwd:join(runRoot, project), timeout_seconds:60 };
  const execution = spawnSync('python3', [join(here,'measure.py')], { input:JSON.stringify(request), encoding:'utf8', timeout:75_000, maxBuffer:16*1024*1024 });
  if (execution.error) throw execution.error;
  assert.equal(execution.status, 0, `Measurement wrapper failed: ${execution.stderr}`);
  return JSON.parse(execution.stdout);
}
function cleanOutput() {
  // Only this run's generated output; never a user-supplied checkout.
  rmSync(join(runRoot,'valid/out'), { recursive:true, force:true });
}
function artifacts() {
  const files = [];
  function visit(path, relative) {
    for (const entry of readdirSync(path,{withFileTypes:true}).sort((a,b)=>a.name.localeCompare(b.name,'en'))) {
      assert.ok(!entry.isSymbolicLink(), 'Compiler output must not escape through symlinks');
      const name = relative ? `${relative}/${entry.name}` : entry.name;
      if (entry.isDirectory()) visit(join(path,entry.name), name);
      else if (entry.isFile()) files.push({path:name,sha256:digest(readFileSync(join(path,entry.name)))});
      else throw new Error(`Unsupported compiler output: ${name}`);
    }
  }
  visit(join(runRoot,'valid/out'),'');
  return files;
}
function metrics(result) {
  return {wall_ms:result.wall_ms,cpu_ms:result.cpu_ms,max_rss_kib:result.max_rss_kib};
}

try {
  const reference = competitors[0];
  for (const project of ['type-error','syntax-error','graph-error']) {
    const result = measure(reference,project,'check');
    checkDiagnostics(result,result,`reference ${project}`);
    report.reference_diagnostics[project] = { exit_code:result.exit_code,stdout:result.stdout,stderr:result.stderr };
  }
  cleanOutput();
  checkSuccess(measure(reference,'valid','emit'),'reference emission');
  const expectedArtifacts = artifacts();
  const manifest = JSON.stringify(expectedArtifacts);
  writeFileSync(join(runRoot,'reference-artifacts.json'),manifest+'\n');
  report.reference_artifacts = { files:expectedArtifacts.length, manifest_sha256:digest(manifest) };
  assert.equal(expectedArtifacts.length,(modules+1)*4,'Expected JS, .d.ts and both maps for every module');
  const eligible = [];
  for (const compiler of competitors) {
    const row = { id:compiler.id,package:compiler.package,package_version:compiler.package_version,reported_version:compiler.reported_version,command:compiler.command,diagnostic_exit_code:compiler.diagnostic_exit_code,probes:{},status:'checking',check_status:'checking',emit_status:'checking',check:[],emit:[] };
    report.compilers.push(row);
    console.log(`Correctness gates: ${compiler.id} (${compiler.package_version ?? compiler.reported_version})`);
    try {
      cleanOutput();
      checkSuccess(measure(compiler,'valid','check'),`${compiler.id} check`);
      assert.ok(!existsSync(join(runRoot,'valid/out')),`${compiler.id}: --noEmit produced output`);
      for (const project of ['type-error','syntax-error','graph-error']) {
        const probe = measure(compiler,project,'check');
        row.probes[project] = { exit_code:probe.exit_code,timed_out:probe.timed_out,stdout:probe.stdout,stderr:probe.stderr };
        checkDiagnostics(probe,{...report.reference_diagnostics[project],timed_out:false},`${compiler.id} ${project}`,compiler.diagnostic_exit_code);
      }
      row.check_status='eligible';
      eligible.push({compiler,row});
    } catch (error) {
      row.status='correctness_failed';
      row.check_status='correctness_failed';
      row.emit_status='not_run';
      row.failure=error.message;
      console.error(`${compiler.id} is excluded: ${error.message}`);
    }
    if (row.check_status === 'eligible') {
      try {
        cleanOutput();
        checkSuccess(measure(compiler,'valid','emit'),`${compiler.id} emit`);
        const actual = artifacts();
        const snapshot = join(runRoot,'artifacts',compiler.id);
        mkdirSync(dirname(snapshot),{recursive:true});
        cpSync(join(runRoot,'valid/out'),snapshot,{recursive:true});
        const byName = new Map(actual.map(file=>[file.path,file.sha256]));
        row.artifact_differences = expectedArtifacts.filter(file=>byName.get(file.path)!==file.sha256).map(file=>file.path);
        row.artifact_manifest_sha256=digest(JSON.stringify(actual));
        checkArtifacts(actual,expectedArtifacts,compiler.id);
        row.emit_status='eligible';
        row.status='eligible';
      } catch (error) {
        row.status='check_only';
        row.emit_status='correctness_failed';
        row.emit_failure=error.message;
        console.error(`${compiler.id} emit is excluded: ${error.message}`);
      }
    }
    save();
  }
  for (const mode of ['check','emit']) {
    for (let round=0; round<samples; round++) {
      for (const {compiler,row} of rotated(eligible.filter(({row})=>row[`${mode}_status`]==='eligible'),round)) {
        console.log(`${mode} ${round+1}/${samples}: ${compiler.id}`);
        cleanOutput();
        const result = measure(compiler,'valid',mode);
        try {
          checkSuccess(result,`${compiler.id} sample ${mode}`);
          if (mode === 'emit') checkArtifacts(artifacts(),expectedArtifacts,compiler.id);
          else assert.ok(!existsSync(join(runRoot,'valid/out')),`${compiler.id}: --noEmit produced output`);
        } catch (error) {
          row.status='measurement_failed';
          row[`${mode}_status`]='measurement_failed';
          row.failure=error.message;
          throw error;
        }
        row[mode].push(metrics(result));
        save();
      }
    }
  }
  for (const {row} of eligible) {
    row.summary={};
    for (const mode of ['check','emit']) {
      if (row[`${mode}_status`] !== 'eligible') continue;
      assert.equal(row[mode].length,samples);
      row[`${mode}_status`]='measured';
      row.summary[mode]=summarize(row[mode]);
    }
    row.status=row.emit_status==='measured' ? 'measured' : 'check_only';
  }
  for (const row of report.compilers) {
    if (row.summary?.check) {
      const emit = row.summary.emit ? `${row.summary.emit.wall_ms.median.toFixed(1)} ms` : row.emit_status;
      console.log(`${row.id}: check ${row.summary.check.wall_ms.median.toFixed(1)} ms; emit ${emit}; max RSS ${Math.max(...[...row.check,...row.emit].map(sample=>sample.max_rss_kib))} KiB`);
    } else console.log(`${row.id}: ${row.status}`);
  }
  report.all_requested_workloads_passed=!report.compilers.some(row=>row.status.endsWith('_failed') || row.emit_status?.endsWith('_failed'));
  if (!report.all_requested_workloads_passed) process.exitCode=1;
} catch (error) {
  report.failure=error.message;
  process.exitCode=1;
  console.error(error);
} finally {
  console.log(`Report: ${save()}`);
}
