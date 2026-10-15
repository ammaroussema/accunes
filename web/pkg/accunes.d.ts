/* tslint:disable */
/* eslint-disable */

export class WasmEmulator {
    free(): void;
    [Symbol.dispose](): void;
    get_audio_samples(): Float32Array;
    get_framebuffer_ptr(): number;
    get_sram(): Uint8Array | undefined;
    load_rom(name: string, data: Uint8Array): void;
    load_sram(sram: Uint8Array): void;
    load_state(state: Uint8Array): boolean;
    constructor(sample_rate: number);
    power_cycle(): void;
    reset(): void;
    save_state(): Uint8Array;
    set_controller_p1(state: number): void;
    set_controller_p2(state: number): void;
    set_theme(theme: string): void;
    step_frame(): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_wasmemulator_free: (a: number, b: number) => void;
    readonly wasmemulator_get_audio_samples: (a: number) => [number, number];
    readonly wasmemulator_get_framebuffer_ptr: (a: number) => number;
    readonly wasmemulator_get_sram: (a: number) => [number, number];
    readonly wasmemulator_load_rom: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmemulator_load_sram: (a: number, b: number, c: number) => void;
    readonly wasmemulator_load_state: (a: number, b: number, c: number) => number;
    readonly wasmemulator_new: (a: number) => number;
    readonly wasmemulator_power_cycle: (a: number) => void;
    readonly wasmemulator_reset: (a: number) => void;
    readonly wasmemulator_save_state: (a: number) => [number, number];
    readonly wasmemulator_set_controller_p1: (a: number, b: number) => void;
    readonly wasmemulator_set_controller_p2: (a: number, b: number) => void;
    readonly wasmemulator_set_theme: (a: number, b: number, c: number) => void;
    readonly wasmemulator_step_frame: (a: number) => void;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __externref_table_dealloc: (a: number) => void;
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
