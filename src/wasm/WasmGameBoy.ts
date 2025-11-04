// Wrapper around the Rust+WASM core exposed via wasm-bindgen
// Requires `npm run build:wasm` to generate src/wasm-core outputs

// Types for the generated wasm-bindgen module
// We defer the import until runtime so Vite can resolve after wasm is built
export interface WasmModule {
  default: (input?: RequestInfo | URL | Response | BufferSource | WebAssembly.Module) => Promise<any>;
  Emulator: any;
  memory: WebAssembly.Memory;
}

export class WasmGameBoy {
  private static initPromise: Promise<WasmModule> | null = null;
  private static wasm: WasmModule | null = null;

  private core: any; // wasm Emulator instance
  public readonly SCREEN_WIDTH: number;
  public readonly SCREEN_HEIGHT: number;

  private constructor(wasm: WasmModule, core: any) {
    this.core = core;
    // call static fns on Emulator for dimensions
    this.SCREEN_WIDTH = (wasm as any).Emulator.screen_width();
    this.SCREEN_HEIGHT = (wasm as any).Emulator.screen_height();
  }

  static async create(): Promise<WasmGameBoy> {
    if (!WasmGameBoy.initPromise) {
      // Import the generated JS glue; path must match build:wasm out-dir and name
      // @ts-ignore: path exists after running build:wasm
      WasmGameBoy.initPromise = import('../wasm-core/cytra_gb_wasm.js') as unknown as Promise<WasmModule>;
    }
    const wasm = await WasmGameBoy.initPromise;
    if ((wasm as any).default) {
      await (wasm as any).default(); // initialize module if needed
    }
    WasmGameBoy.wasm = wasm;
    const core = new (wasm as any).Emulator();
    return new WasmGameBoy(wasm, core);
  }

  // API mirrored from the previous TS GameBoy
  reset() { this.core.reset(); }

  loadROM(data: Uint8Array) { this.core.load_rom(data); }

  start() { this.core.start(); }

  stop() { this.core.stop(); }

  isRunning(): boolean { return this.core.is_running(); }

  runFrame(): boolean { return this.core.run_frame(); }

  getFrameBuffer(): Uint8Array {
    const wasm = WasmGameBoy.wasm as unknown as any;
    const ptr = this.core.framebuffer_ptr();
    const len = this.core.framebuffer_len();
    return new Uint8Array(wasm.memory.buffer, ptr, len);
  }

  // Placeholder input passthrough; real mapping will be added when core implements it
  public input = {
    pressButton: (_button: number) => {},
    releaseButton: (_button: number) => {},
  };
}
