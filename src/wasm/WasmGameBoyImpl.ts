// WASM wrapper for the emulator core
// Loads the wasm-bindgen package built to ./pkg

export type WasmModule = typeof import('./pkg/cytra_gb_core');

export class WasmGameBoy {
  private mod!: WasmModule;
  private core!: import('./pkg/cytra_gb_core').GameBoy;
  private memory!: WebAssembly.Memory;
  private fb!: Uint8Array;
  private lastTrace: string = '';
  private disposed: boolean = false;
  private traceFrameCounter: number = 0;

  static async create(): Promise<WasmGameBoy> {
    const instance = new WasmGameBoy();
    // Dynamic import to allow dev/build without wasm until built
    const mod = await import('./pkg/cytra_gb_core');
    // Initialize WASM module
    const initOutput = await mod.default();
    instance.memory = initOutput.memory;
    instance.mod = mod;
    instance.core = new mod.GameBoy();
    // Note: avoid enabling opcode trace by default to reduce WASM allocations
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

  loadROM(data: Uint8Array): void {
    if (!this.disposed) {
      this.core.load_rom(data);
      this.disposed = false; // Allow re-use after ROM load
    }
  }
  reset(): void {
    if (!this.disposed) this.core.reset();
  }
  start(): void {
    if (!this.disposed) this.core.start();
  }
  stop(): void {
    if (!this.disposed) this.core.stop();
  }
  isRunning(): boolean {
    if (this.disposed) return false;
    return this.core.is_running();
  }

  runFrame(): boolean {
    if (this.disposed) {
      console.warn('Attempted to run frame on disposed GameBoy instance');
      return false;
    }

    try {
      const ready = this.core.run_frame();
      // Copy out latest framebuffer to keep a stable view for canvas
      const ptr = this.core.frame_buffer_ptr();
      const len = this.core.frame_buffer_len();
      
      // Validate framebuffer dimensions
      const expectedLen = 160 * 144 * 4;
      if (len !== expectedLen) {
        console.error(`Framebuffer size mismatch: expected ${expectedLen}, got ${len}`);
        this.disposed = true;
        return false;
      }
      
      // Check if pointer is within valid memory range
      if (ptr < 0 || ptr + len > this.memory.buffer.byteLength) {
        console.error(`Framebuffer pointer out of bounds: ptr=${ptr}, len=${len}, memory=${this.memory.buffer.byteLength}`);
        this.disposed = true;
        return false;
      }
      
      const bytes = new Uint8Array(this.memory.buffer, ptr, len);
      this.fb.set(bytes);

      // Snapshot opcode trace infrequently to limit allocator pressure
      // Never call into WASM on error path; only update snapshot during healthy frames
      try {
        this.traceFrameCounter = (this.traceFrameCounter + 1) % 60; // ~once per second at 60fps
        if (this.traceFrameCounter === 0) {
          const anyCore: any = this.core as any;
          if (anyCore.dump_trace) {
            this.lastTrace = anyCore.dump_trace();
          }
        }
      } catch {
        // Ignore trace failures; keep previous snapshot
      }

      return ready;
    } catch (e) {
      // Mark as disposed to prevent further use
      this.disposed = true;
      if (this.lastTrace) {
        console.error('CPU trace (last ops):\n' + this.lastTrace);
      }
      throw e;
    }
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
