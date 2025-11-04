import { useEffect, useRef } from 'react';
import './EmulatorScreen.css';
import { SCREEN_WIDTH, SCREEN_HEIGHT } from '../types/constants';

interface EmulatorLike {
  getFrameBuffer(): Uint8Array;
}

interface EmulatorScreenProps {
  emulator: EmulatorLike | null;
}

function EmulatorScreen({ emulator }: EmulatorScreenProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const imageData = ctx.createImageData(SCREEN_WIDTH, SCREEN_HEIGHT);

    let raf = 0;
    const render = () => {
      if (emulator) {
        const frameBuffer = emulator.getFrameBuffer();
        imageData.data.set(frameBuffer);
        ctx.putImageData(imageData, 0, 0);
      }
      raf = requestAnimationFrame(render);
    };

    raf = requestAnimationFrame(render);
    return () => cancelAnimationFrame(raf);
  }, [emulator]);

  return (
    <div className="emulator-screen">
      <div className="screen-container">
        <canvas
          ref={canvasRef}
          width={SCREEN_WIDTH}
          height={SCREEN_HEIGHT}
          className="game-canvas"
        />
      </div>
      <div className="controls-hint">
        <p>Controls: Arrow Keys • Z (A) • X (B) • Enter (Start) • Shift (Select)</p>
      </div>
    </div>
  );
}

export default EmulatorScreen;
