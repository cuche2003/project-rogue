/* tslint:disable */
/* eslint-disable */

/**
 * JavaScript-facing handle for the game simulation.
 */
export class Engine {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Advances the simulation by one turn.
     */
    advance_turn(): void;
    /**
     * Dispatches one named player command and returns result, events, and snapshot as JSON.
     */
    command(action: string): string;
    /**
     * Creates a new game simulation.
     */
    constructor();
    /**
     * Returns the current player-visible render state as JSON.
     */
    snapshot(): string;
    /**
     * Returns the current simulation turn.
     */
    turn(): bigint;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_engine_free: (a: number, b: number) => void;
    readonly engine_advance_turn: (a: number) => void;
    readonly engine_command: (a: number, b: number, c: number) => [number, number, number, number];
    readonly engine_new: () => number;
    readonly engine_snapshot: (a: number) => [number, number, number, number];
    readonly engine_turn: (a: number) => bigint;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
