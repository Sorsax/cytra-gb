import { useRef } from 'react';
import './TopBar.css';

interface TopBarProps {
  onLoadROM: (file: File) => void;
  onSaveState: () => void;
  onLoadState: () => void;
  onReset: () => void;
  onTogglePlay: () => void;
  isRunning: boolean;
  romLoaded: boolean;
}

function TopBar({
  onLoadROM,
  onSaveState,
  onLoadState,
  onReset,
  onTogglePlay,
  isRunning,
  romLoaded,
}: TopBarProps) {
  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      onLoadROM(file);
    }
  };

  const handleLoadClick = () => {
    fileInputRef.current?.click();
  };

  return (
    <div className="topbar">
      <div className="topbar-left">
        <h1 className="logo">CytraGB</h1>
        <span className="tagline">Game Boy Color Emulator</span>
      </div>

      <div className="topbar-controls">
        <input
          ref={fileInputRef}
          type="file"
          accept=".gb,.gbc"
          onChange={handleFileSelect}
          style={{ display: 'none' }}
        />
        
        <button className="btn btn-primary" onClick={handleLoadClick}>
          Load ROM
        </button>

        {romLoaded && (
          <>
            <button
              className={`btn ${isRunning ? 'btn-danger' : 'btn-success'}`}
              onClick={onTogglePlay}
            >
              {isRunning ? 'Pause' : 'Play'}
            </button>

            <button className="btn" onClick={onReset}>
              Reset
            </button>

            <div className="separator"></div>

            <button className="btn" onClick={onSaveState}>
              Save State
            </button>

            <button className="btn" onClick={onLoadState}>
              Load State
            </button>
          </>
        )}
      </div>
    </div>
  );
}

export default TopBar;
