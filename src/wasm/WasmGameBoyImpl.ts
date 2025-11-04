// WASM wrapper for the emulator core
// Loads the wasm-bindgen package built to ./pkg

export type WasmModule = typeof import('./pkg/cytra_gb_core');

export class WasmGameBoy {
  private mod!: WasmModule;
  private core!: import('./pkg/cytra_gb_core').GameBoy;
  private memory!: WebAssembly.Memory;
  private fb!: Uint8Array;

  static async create(): Promise<WasmGameBoy> {
    const instance = new WasmGameBoy();
    // Dynamic import to allow dev/build without wasm until built
    const mod = await import('./pkg/cytra_gb_core');
    // Initialize WASM module
    const initOutput = await mod.default();
    instance.memory = initOutput.memory;
    instance.mod = mod;
    instance.core = new mod.GameBoy();
    instance.refreshFrameBufferView();
    return instance;
  }

  private refreshFrameBufferView() {
    const ptr = this.core.frame_buffer_ptr();
    const len = this.core.frame_buffer_len();
    const bytes = new Uint8Array(this.memory.buffer, ptr, len);
    this.fb = new Uint8Array(len);
    this.fb.set(bytes);
  }

  get screenWidth(): number { return this.mod.screen_width(); }
  get screenHeight(): number { return this.mod.screen_height(); }

  loadROM(data: Uint8Array): void { this.core.load_rom(data); }
  reset(): void { this.core.reset(); }
  start(): void { this.core.start(); }
  stop(): void { this.core.stop(); }
  isRunning(): boolean { return this.core.is_running(); }

  runFrame(): boolean {
    const ready = this.core.run_frame();
    // Copy out latest framebuffer to keep a stable view for canvas
    const ptr = this.core.frame_buffer_ptr();
    const len = this.core.frame_buffer_len();
    const bytes = new Uint8Array(this.memory.buffer, ptr, len);
    this.fb.set(bytes);
    return ready;
  }

  getFrameBuffer(): Uint8Array { return this.fb; }

  // Provide an Input-like API to minimize UI changes
  public input = {
    pressButton: (button: number) => this.core.press_button(button),
    releaseButton: (button: number) => this.core.release_button(button),
  };

  saveState(): string { return this.core.save_state(); }
  loadState(state: string): void { this.core.load_state(state); }
}
