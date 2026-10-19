// Representative syntax for the context-free scanner, not a compilation test.
export interface Result<T> {
    readonly value: T;
    readonly error?: Error;
}

export function map<T, U>(result: Result<T>, fn: (value: T) => U): Result<U> {
    const value = fn(result.value);
    return { value, error: result.error } satisfies Result<U>;
}

class Counter {
    #value = 0;
    increment(amount = 1_000): number {
        this.#value += amount;
        return this.#value;
    }
}

const mask = 0xffn | 0b101n | 0o77n;
const ratio = .5e+2;
const greeting = 'hello\nworld';
const message = `no substitutions`;
const fallback = greeting?.length ?? 0;
