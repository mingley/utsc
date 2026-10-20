import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

export function createWorkload(root, libraryRoot, modules) {
  const files = new Map();
  const put = (path, text) => {
    const pathParts = path.split('/');
    mkdirSync(join(root, ...pathParts.slice(0, -1)), { recursive: true });
    writeFileSync(join(root, path), text);
    files.set(path, text);
  };
  // Identical actual library inputs for every compiler, not each one's defaults.
  const libraries = ['lib.es5.d.ts', 'lib.decorators.d.ts', 'lib.decorators.legacy.d.ts'];
  for (const name of libraries) put(`lib/${name}`, readFileSync(join(libraryRoot, name), 'utf8'));
  const config = {
    compilerOptions: {
      target: 'ES2022', module: 'ESNext', moduleResolution: 'Bundler',
      strict: true, noLib: true, types: [], skipLibCheck: false,
      incremental: false, noEmitOnError: true, newLine: 'lf',
      rootDir: './src', outDir: './out', declaration: true, declarationMap: true, sourceMap: true,
    },
    files: libraries.map(name=>`../lib/${name}`),
    include: ['src/**/*.ts'],
  };
  for (const name of ['valid', 'type-error', 'syntax-error', 'graph-error']) put(`${name}/tsconfig.json`, JSON.stringify(config, null, 2)+'\n');
  const index = [];
  for (let i = 0; i < modules; i++) {
    const name = `module${i.toString().padStart(4,'0')}`;
    const previous = i ? `import type { Entity as Previous } from './module${(i-1).toString().padStart(4,'0')}.js';\n` : 'type Previous = null;\n';
    put(`valid/src/${name}.ts`, `${previous}
export interface Entity {
  readonly id: \`entity${i}:\${number}\`;
  payload: { name: string; scores: readonly number[] };
  previous?: Previous | null;
}
export type DeepReadonly<T> = T extends object ? { readonly [K in keyof T]: DeepReadonly<T[K]> } : T;
export type Getters = { [K in keyof Entity as \`get\${Capitalize<string & K>}\`]: () => Entity[K] };
export type Event<T> = { kind: 'ok'; value: T } | { kind: 'error'; message: string };
export type Value<E> = E extends { kind: 'ok'; value: infer V } ? V : never;
export const entity = {
  id: 'entity${i}:1', payload: { name: 'item ${i}', scores: [1, 2, 3] }, previous: null,
} satisfies Entity;
export const snapshot: DeepReadonly<Entity> = entity;
export function map<T, U>(event: Event<T>, fn: (value: T) => U): Event<U> {
  return event.kind === 'ok' ? { kind: 'ok', value: fn(event.value) } : event;
}
export function describe(event: Event<Entity>): string {
  switch (event.kind) {
    case 'ok': return event.value.payload.name;
    case 'error': return event.message;
  }
}
export const mapped = map({ kind: 'ok', value: entity }, item => item.payload.name);
export type Extracted = Value<Event<Entity>>;
`);
    index.push(`import { entity as entity${i} } from './${name}.js';`);
  }
  index.push(`export const entities = [${Array.from({length:modules},(_,i)=>`entity${i}`).join(', ')}] as const;`);
  index.push('export type EntityId = typeof entities[number]["id"];');
  put('valid/src/index.ts', index.join('\n')+'\n');
  // A diagnostic in the last module proves that the complete project graph is loaded.
  for (const [path, text] of [...files]) {
    if (path.startsWith('valid/src/')) put(path.replace('valid/', 'graph-error/'), text);
  }
  const last = `graph-error/src/module${(modules-1).toString().padStart(4,'0')}.ts`;
  put(last, files.get(last) + `export const benchmarkError: number = 'intentional error';\n`);
  put('type-error/src/index.ts', 'export const value: number = "wrong";\nexport const other: string = false;\n');
  put('syntax-error/src/index.ts', 'export const value = ;\n');
  const hash = createHash('sha256');
  let bytes = 0;
  for (const [name, text] of [...files].sort(([a],[b])=>a.localeCompare(b,'en'))) {
    hash.update(name+'\0').update(text).update('\0');
    bytes += Buffer.byteLength(text);
  }
  return { modules, timed_source_files:modules+1, timed_source_utf8_bytes:[...files].filter(([name])=>name.startsWith('valid/src/')).reduce((sum,[,text])=>sum+Buffer.byteLength(text),0), files: files.size, utf8_bytes: bytes, sha256: hash.digest('hex'), shared_libraries: libraries };
}
