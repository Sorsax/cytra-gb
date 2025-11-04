# CytraGB

<div align="center">

![CytraGB Logo](public/logo.png)

**A high-performance Game Boy Color emulator for the web**

Built with Rust (WebAssembly)

[Demo](#) • [Documentation](ARCHITECTURE.md) • [User Guide](USER_GUIDE.md) • [Setup](SETUP.md)

</div>

---

## Features

### High Performance
- **WebAssembly Core** - Emulation runs in compiled WASM for maximum speed
- **Rust-powered** - Memory-safe, zero-cost abstractions
- Direct canvas rendering for minimal overhead
- ~70,000 CPU cycles per frame

### Emulation
- **CPU**: Full Sharp LR35902 instruction set (256 base + 256 CB-prefix opcodes)
- **PPU**: Scanline-accurate background, window, and sprite rendering
- **MMU**: ROM/RAM banking (MBC1-style) with GBC support
- **Timer**: DIV and TIMA with interrupt generation

### Game Boy Color Support
- GBC-enhanced games with color palettes
- VRAM and WRAM banking
- Both DMG and GBC modes

### Save States
- Save your progress at any time
- Stored in browser local storage
- Persists between sessions
- Quick save/load functionality

### Keyboard Controls

| Button | Key |
|--------|-----|
| D-Pad  | Arrow Keys (↑↓←→) |
| A      | Z |
| B      | X |
| Start  | Enter |
| Select | Shift |

## Project Structure

```
cytra-gb/
├── wasm/gb/                # Rust WASM emulator core (~2,000 lines)
│   ├── src/
│   │   ├── lib.rs          # Main GameBoy coordinator (1400+ lines)
│   │   ├── ppu.rs          # PPU scanline renderer (376 lines)
│   │   ├── mmu.rs          # Memory management (270+ lines)
│   │   ├── registers.rs    # CPU registers (58 lines)
│   │   ├── timer.rs        # Timer system (50 lines)
│   │   ├── input.rs        # Joypad input (47 lines)
│   │   └── apu.rs          # Audio stub (34 lines)
│   └── Cargo.toml          # Rust dependencies
├── src/                    # React UI
│   ├── components/         # React UI components
│   │   ├── TopBar.tsx      # Navigation bar
│   │   └── EmulatorScreen.tsx  # Canvas display
│   ├── wasm/               # WASM wrapper and types
│   │   ├── WasmGameBoyImpl.ts  # TypeScript wrapper
│   │   └── types.d.ts      # Type definitions
│   ├── types/              # Shared TypeScript types
│   ├── App.tsx             # Main app component
│   └── main.tsx            # Entry point
├── public/                 # Static assets
└── dist/                   # Build output
```

## Compatibility

### Supported

- Game Boy (.gb) games
- Game Boy Color (.gbc) games
- MBC1, MBC3, MBC5 cartridges

### Limitations

- Sound is simplified (Web Audio integration planned)
- RTC (Real-Time Clock) partially supported
- Some timing-sensitive games may have minor issues

### Not Supported

- Audio output (APU is stub only)
- Special hardware (rumble, camera, tilt sensor)
- Game Boy Advance games
- Multi-ROM cartridge dumps

### Known Issues

- **Runtime panic**: Some ROMs cause WASM panic during execution
- Save states work but emulation stability needs improvement

## Documentation

- **[ARCHITECTURE.md](ARCHITECTURE.md)**: Technical deep-dive into emulator design
- **[USER_GUIDE.md](USER_GUIDE.md)**: Complete user manual with troubleshooting
- **[SETUP.md](SETUP.md)**: Detailed installation and setup instructions
- **[PROJECT_SUMMARY.md](PROJECT_SUMMARY.md)**: Project overview and statistics

## Technology Stack

- **Rust 1.70+**: Emulator core implementation
- **WebAssembly**: Compiled target for web deployment
- **wasm-bindgen**: Rust/JavaScript interop
- **React 18**: UI framework
- **TypeScript 5**: Type-safe JavaScript
- **Vite 5**: Build tool and dev server
- **HTML5 Canvas**: Graphics rendering
- **Local Storage API**: Save state persistence

## Development

### Code Structure

The emulator follows a modular architecture with Rust/WASM core:

```
GameBoy (Rust/WASM)
  ├── CPU (instruction execution - in lib.rs)
  ├── MMU (memory management - mmu.rs)
  ├── PPU (graphics rendering - ppu.rs)
  ├── APU (audio stub - apu.rs)
  ├── Input (keyboard handling - input.rs)
  ├── Timer (timing and interrupts - timer.rs)
  └── Registers (CPU state - registers.rs)
```

### Main Execution Loop

```rust
while running {
  1. step_cpu() - Execute one instruction
  2. timer.step() - Update DIV/TIMA
  3. apu.step() - Track audio timing
  4. ppu.step() - Render scanline, returns true on VBlank
  5. Check interrupts (VBlank, STAT, Timer, Joypad)
}
```

### Build Information

- **WASM binary**: ~86 KB (optimized with wasm-opt)
- **JS wrapper**: ~4 KB (wasm-bindgen generated)
- **Total bundle**: ~148 KB (including React UI)
- **Build time**: ~2-3 seconds for WASM core

## License

MIT License - See [LICENSE](LICENSE) file for details.

## Legal Notice

**Important**: CytraGB is an emulator, not a game distribution platform.

- Emulators are legal
- Downloading copyrighted ROMs without owning the game is **not**
- You must provide your own legally-obtained ROM files

**Legal ways to obtain ROMs:**
1. Dump your own cartridges with a cartridge reader
2. Download homebrew games (free and legal)
3. Purchase digital copies from authorized sellers

**Use responsibly and respect copyright laws.**

---