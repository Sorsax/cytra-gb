import { useState, useEffect, useRef, useCallback } from 'react';
import TopBar from './components/TopBar';
import EmulatorScreen from './components/EmulatorScreen';
import './App.css';
import { WasmGameBoy } from './wasm/WasmGameBoyImpl';
import * as Input from './types/input';

function App() {
  const [emulator, setEmulator] = useState<WasmGameBoy | null>(null);
  const [isRunning, setIsRunning] = useState(false);
  const [romLoaded, setRomLoaded] = useState(false);
  const animationFrameRef = useRef<number>();
  const runningRef = useRef(false);

  // Load WASM core once
  useEffect(() => {
    (async () => {
      try {
        const emu = await WasmGameBoy.create();
        setEmulator(emu);
      } catch (e) {
        console.error('Failed to initialize WASM core', e);
        alert('Failed to initialize WebAssembly core. Build it via "npm run build:wasm".');
      }
    })();
  }, []);

  // Main emulation loop
  const emulationLoop = useCallback(() => {
    if (!emulator || !runningRef.current) {
      return;
    }
    
    try {
      emulator.runFrame();
      // Only schedule next frame if still running after this frame
      if (runningRef.current) {
        animationFrameRef.current = requestAnimationFrame(emulationLoop);
      }
    } catch (e) {
      console.error('runFrame threw:', e);
      // Immediately stop to prevent any further frames
      runningRef.current = false;
      setIsRunning(false);
      if (animationFrameRef.current) {
        cancelAnimationFrame(animationFrameRef.current);
        animationFrameRef.current = undefined;
      }
    }
  }, [emulator]);

  // Start emulation
  const startEmulation = useCallback(() => {
    if (!romLoaded || !emulator) return;
    runningRef.current = true;
    emulator.start();
    setIsRunning(true);
    animationFrameRef.current = requestAnimationFrame(emulationLoop);
  }, [emulator, romLoaded, emulationLoop]);

  // Stop emulation
  const stopEmulation = useCallback(() => {
    if (!emulator) return;
    emulator.stop();
    runningRef.current = false;
    setIsRunning(false);
    if (animationFrameRef.current) {
      cancelAnimationFrame(animationFrameRef.current);
    }
  }, [emulator]);

  // Load ROM
  const handleLoadROM = useCallback(async (file: File) => {
    if (!emulator) return;
    try {
      const arrayBuffer = await file.arrayBuffer();
      const data = new Uint8Array(arrayBuffer);
      console.log(`Loading ROM: ${file.name}, size: ${data.length} bytes`);
      emulator.loadROM(data);
      setRomLoaded(true);
      console.log('ROM loaded successfully');
      
      // Start emulation directly (don't rely on async state)
      runningRef.current = true;
      emulator.start();
      setIsRunning(true);
      animationFrameRef.current = requestAnimationFrame(emulationLoop);

    } catch (error) {
      console.error('Failed to load ROM:', error);
      alert('Failed to load ROM file');
    }
  }, [emulator, emulationLoop]);

  // Save state
  const handleSaveState = useCallback(() => {
    if (!romLoaded || !emulator) return;
    const state = emulator.saveState();
    localStorage.setItem('cytra-gb-savestate', state);
    alert('State saved successfully!');
  }, [emulator, romLoaded]);

  // Load state
  const handleLoadState = useCallback(() => {
    if (!romLoaded || !emulator) return;
    const state = localStorage.getItem('cytra-gb-savestate');
    if (state) {
      emulator.loadState(state);
      alert('State loaded successfully!');
    } else {
      alert('No saved state found');
    }
  }, [emulator, romLoaded]);

  // Reset emulator
  const handleReset = useCallback(() => {
    if (!romLoaded || !emulator) return;
    const wasRunning = isRunning;
    stopEmulation();
    emulator.reset();
    if (wasRunning) {
      startEmulation();
    }
  }, [emulator, romLoaded, isRunning, stopEmulation, startEmulation]);

  // Keyboard controls
  useEffect(() => {
    if (!emulator) return;

    const keyMap: { [key: string]: number } = {
      ArrowUp: Input.BUTTON_UP,
      ArrowDown: Input.BUTTON_DOWN,
      ArrowLeft: Input.BUTTON_LEFT,
      ArrowRight: Input.BUTTON_RIGHT,
      z: Input.BUTTON_A,
      x: Input.BUTTON_B,
      Enter: Input.BUTTON_START,
      Shift: Input.BUTTON_SELECT,
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      const button = keyMap[e.key];
      if (button !== undefined) {
        e.preventDefault();
        emulator.input.pressButton(button);
      }
    };

    const handleKeyUp = (e: KeyboardEvent) => {
      const button = keyMap[e.key];
      if (button !== undefined) {
        e.preventDefault();
        emulator.input.releaseButton(button);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    window.addEventListener('keyup', handleKeyUp);

    return () => {
      window.removeEventListener('keydown', handleKeyDown);
      window.removeEventListener('keyup', handleKeyUp);
    };
  }, [emulator]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (animationFrameRef.current) {
        cancelAnimationFrame(animationFrameRef.current);
      }
    };
  }, []);

  return (
    <div className="app">
      <TopBar
        onLoadROM={handleLoadROM}
        onSaveState={handleSaveState}
        onLoadState={handleLoadState}
        onReset={handleReset}
        onTogglePlay={isRunning ? stopEmulation : startEmulation}
        isRunning={isRunning}
        romLoaded={romLoaded}
      />
      <EmulatorScreen emulator={emulator} />
    </div>
  );
}

export default App;
