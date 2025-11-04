declare module './pkg/cytra_gb_core' {
  export class GameBoy {
    constructor();
    load_rom(data: Uint8Array): void;
    reset(): void;
    start(): void;
    stop(): void;
    is_running(): boolean;
    run_frame(): boolean;
    frame_buffer_ptr(): number;
    frame_buffer_len(): number;
    press_button(button: number): void;
    release_button(button: number): void;
    save_state(): string;
    load_state(state: string): void;
  }
  export function screen_width(): number;
  export function screen_height(): number;
  export const memory: WebAssembly.Memory;
}
