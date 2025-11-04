// WASM wrapper for the emulator core
// Loads the wasm-bindgen package built to ./pkg

export type WasmModule = typeof import('./pkg/cytra_gb_core');

export class WasmGameBoy {
  private mod!: WasmModule;
  private memory!: WebAssembly.Memory;
  private fb!: Uint8Array;
  private lastTrace: string = '';
  private disposed: boolean = false;
  private traceFrameCounter: number = 0;
  private busy: boolean = false;
  private fbPtr: number = 0;
  private fbLen: number = 0;

  static async create(): Promise<WasmGameBoy> {
    const instance = new WasmGameBoy();
    // Dynamic import to allow dev/build without wasm until built
    const mod = await import('./pkg/cytra_gb_core');
    // Initialize WASM module
    const initOutput = await mod.default();
    instance.memory = initOutput.memory;
    instance.mod = mod;
    // Use free-function API backed by a Rust-side singleton to avoid Rc churn
    (mod as any).gb_create();
    instance.refreshFrameBufferView();
    return instance;
  }

  private refreshFrameBufferView() {
    // Cache framebuffer location/size once; they are stable
  this.fbPtr = (this.mod as any).gb_frame_buffer_ptr();
  this.fbLen = (this.mod as any).gb_frame_buffer_len();
    const bytes = new Uint8Array(this.memory.buffer, this.fbPtr, this.fbLen);
    this.fb = new Uint8Array(this.fbLen);
    this.fb.set(bytes);
  }

  get screenWidth(): number { return this.mod.screen_width(); }
  get screenHeight(): number { return this.mod.screen_height(); }

  loadROM(data: Uint8Array): void {
    if (!this.disposed) {
      (this.mod as any).gb_load_rom(data);
      this.disposed = false; // Allow re-use after ROM load
    }
  }
  reset(): void {
    if (!this.disposed) (this.mod as any).gb_reset();
  }
  start(): void {
    if (!this.disposed) (this.mod as any).gb_start();
  }
  stop(): void {
    if (!this.disposed) (this.mod as any).gb_stop();
  }
  isRunning(): boolean {
    if (this.disposed) return false;
    return (this.mod as any).gb_is_running();
  }

  runFrame(): boolean {
    if (this.disposed) {
      console.warn('Attempted to run frame on disposed GameBoy instance');
      return false;
    }
    if (this.busy) {
      // Prevent any re-entrancy into WASM; skip this frame
      return false;
    }

    try {
      this.busy = true;
  const ready = (this.mod as any).gb_run_frame();
      // Copy out latest framebuffer to keep a stable view for canvas
      const ptr = this.fbPtr;
      const len = this.fbLen;
      
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
          const anyCore: any = this.mod as any;
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
    } finally {
      this.busy = false;
    }
  }

  getFrameBuffer(): Uint8Array { return this.fb; }

  // Provide an Input-like API to minimize UI changes
  public input = {
    pressButton: (button: number) => (this.mod as any).gb_press_button?.(button),
    releaseButton: (button: number) => (this.mod as any).gb_release_button?.(button),
  };

  saveState(): string { return (this.mod as any).gb_save_state?.(); }
  loadState(state: string): void { (this.mod as any).gb_load_state?.(state); }
}
