# CytraGB User Guide

Welcome to CytraGB, a high-performance Game Boy Color emulator for the web!

## Getting Started

### First Time Setup

1. **Open CytraGB** in your web browser
2. **Load a ROM** by clicking the "Load ROM" button
3. **Select a .gb or .gbc file** from your computer
4. The game will **start automatically**!

### Where to Get ROMs

**Important**: CytraGB does not include any games. You must provide your own ROM files.

Legal ways to obtain ROMs:
- **Dump your own cartridges** using a cartridge reader
- **Download homebrew games** (free, legal, and fun!)
  - [itch.io Game Boy games](https://itch.io/games/tag-game-boy)
  - [GB Studio games](https://gbstudio.dev/)
- **Purchase digital copies** from authorized sellers

## Controls

### Keyboard Controls

| Button | Key |
|--------|-----|
| D-Pad Up | ↑ (Arrow Up) |
| D-Pad Down | ↓ (Arrow Down) |
| D-Pad Left | ← (Arrow Left) |
| D-Pad Right | → (Arrow Right) |
| A Button | Z |
| B Button | X |
| Start | Enter |
| Select | Shift |

### Tips:
- Use **Z and X** for A/B (comfortable for platformers)
- Press **Enter** to pause/start games
- **Shift** is often used for menu navigation

## Features

### Load ROM
Click "Load ROM" to open a file picker. Select any .gb or .gbc file.

**Supported formats**:
- ✅ .gb (Game Boy)
- ✅ .gbc (Game Boy Color)

**Not supported**:
- ❌ .zip files (extract first)
- ❌ .rar, .7z (extract first)
- ❌ Multi-game ROMs

### Play/Pause
Click "Play" to start or "Pause" to stop emulation.

**Note**: The emulator continues running in the background when paused, but the game state is frozen.

### Reset
Click "Reset" to restart the game from the beginning.

**Warning**: This will lose any unsaved progress unless you've saved a state!

### Save State
Click "Save State" to save your current game position.

**Features**:
- Saves to browser's local storage
- Persists between sessions
- Overrides previous save
- Includes CPU state, memory, and graphics

**Note**: Only one save state per game is supported currently.

### Load State
Click "Load State" to restore your saved position.

**Requirements**:
- Must have a saved state
- Same ROM must be loaded
- Works across browser sessions

## Compatibility

### Supported Games

CytraGB supports most Game Boy and Game Boy Color games, including:

**Memory Bank Controllers**:
- ✅ No MBC (32KB games)
- ✅ MBC1 (most common, up to 2MB)
- ✅ MBC3 (Pokemon, up to 2MB)
- ✅ MBC5 (large games, up to 8MB)

**Special Features**:
- ✅ Game Boy Color enhanced games
- ✅ Game Boy Color exclusive games
- ⚠️ RTC (Real-Time Clock) - partial support
- ❌ Rumble - not supported
- ❌ Camera - not supported
- ❌ Tilt sensor - not supported

### Known Issues

**Audio**:
- Sound is currently simplified
- Some audio effects may not be accurate

**Timing**:
- Most games run at correct speed
- Some games may have minor timing issues

**Graphics**:
- Occasional sprite priority issues
- GBC palette handling is simplified

## Performance

### System Requirements

**Minimum**:
- Modern web browser (Chrome, Firefox, Edge, Safari)
- 2 GB RAM
- Any modern CPU

**Recommended**:
- Latest browser version
- 4 GB RAM
- Multi-core CPU

### Performance Tips

If you experience lag or stuttering:

1. **Close other tabs** - Free up system resources
2. **Disable extensions** - Ad blockers can impact performance
3. **Use Chrome/Edge** - Best WebAssembly performance
4. **Zoom to 100%** - Browser zoom can affect rendering
5. **Check CPU usage** - Close other programs
6. **Update browser** - Latest versions are fastest

### Expected Performance

- **60 FPS** on most modern computers
- **~70,000 CPU cycles per frame**
- **16.7ms frame time** (59.7 Hz)
- **Low latency** input (< 1 frame)

## Troubleshooting

### ROM Won't Load

**Symptoms**: Error message or nothing happens

**Solutions**:
1. Check file format (.gb or .gbc)
2. Ensure file isn't corrupted
3. Try a different ROM
4. Check browser console for errors (F12)
5. Reload the page and try again

### Black Screen

**Symptoms**: Game loads but screen is black

**Solutions**:
1. Click "Reset" to restart
2. Reload the page
3. Check if ROM is compatible
4. Try a different browser

### Controls Not Working

**Symptoms**: Keyboard presses don't affect game

**Solutions**:
1. Click on the emulator screen
2. Check if browser has focus
3. Try refreshing the page
4. Ensure no other application is capturing keys
5. Check for keyboard software conflicts

### Graphical Glitches

**Symptoms**: Sprites flickering, wrong colors, artifacts

**Solutions**:
1. This may be expected for some games
2. Try resetting the emulator
3. Check if ROM file is corrupted
4. Report persistent issues on GitHub

### Slow Performance

**Symptoms**: Game runs slowly, stutters, or freezes

**Solutions**:
1. Close other tabs and programs
2. Disable browser extensions
3. Check CPU usage in Task Manager
4. Try a different browser
5. Restart your computer

### Save State Issues

**Symptoms**: Can't save or load states

**Solutions**:
1. Check if browser allows local storage
2. Clear browser cache and try again
3. Ensure enough disk space
4. Try in incognito mode (won't persist)
5. Check browser settings for storage

## Advanced Usage

### Browser Storage

Save states are stored in your browser's local storage:

**Chrome/Edge**:
- Settings → Privacy → Site Settings → Local Storage

**Firefox**:
- Preferences → Privacy → Cookies and Site Data

**To clear**:
1. Open browser developer tools (F12)
2. Go to Application/Storage tab
3. Expand Local Storage
4. Delete "cytra-gb-savestate"

### Developer Console

Press F12 to open developer tools and see:
- Emulator debug output
- Performance metrics
- Error messages
- Console logs

Useful commands:
```javascript
// Check emulator state
console.log(emulator.isRunning());

// Get CPU cycles
console.log(emulator.cpu.cycles);
```

### Screen Recording

To record gameplay:

**Windows**:
- Use Windows Game Bar (Win+G)

**macOS**:
- Use Screenshot app (Cmd+Shift+5)

**Browser**:
- Use browser screen recording extension

## FAQ

### Q: Where are my save states stored?
**A**: In your browser's local storage. They persist between sessions but are browser-specific.

### Q: Can I play multiplayer games?
**A**: Not currently. Link cable support is planned for the future.

### Q: Why is there no sound?
**A**: Sound is simplified in the current version. Full audio support is in development.

### Q: Can I use a gamepad/controller?
**A**: Not yet. Gamepad support is planned for a future update.

### Q: How accurate is the emulation?
**A**: CytraGB aims for high accuracy. Most games work perfectly, but some edge cases may have issues.

### Q: Can I speed up the emulation?
**A**: Fast forward is not implemented yet, but it's planned.

### Q: Does it work on mobile?
**A**: It works but keyboard controls are required. Touch controls are planned.

### Q: Is my game progress saved automatically?
**A**: No! Many games have in-game saves, but you should use save states to be safe.

### Q: Can I export my save states?
**A**: Not currently, but this feature is planned.

### Q: The colors look wrong?
**A**: Some games may have palette issues. GBC color support is continually improving.

## Keyboard Shortcuts

Currently, only game controls are supported. Future versions may include:
- F11: Fullscreen
- F1: Show help
- F5: Quick save
- F9: Quick load
- Tab: Fast forward

## Best Practices

1. **Save states often** - Don't lose progress!
2. **Test ROMs** - Not all games work perfectly
3. **Use latest browser** - Best performance
4. **Report bugs** - Help improve the emulator
5. **Have fun!** - That's what gaming is about

## Support

### Getting Help

If you need help:
1. Check this user guide
2. Read SETUP.md for technical issues
3. Check browser console for errors
4. Search for similar issues
5. Open an issue on GitHub

### Reporting Bugs

When reporting bugs, include:
- Browser and version
- Operating system
- ROM name (if public domain)
- Steps to reproduce
- Screenshots or videos
- Error messages from console

## Credits

**CytraGB** was built with:
- React for UI
- TypeScript for type safety
- Vite for build tooling
- Modern web APIs

Special thanks to:
- Game Boy community for documentation
- Pan Docs for comprehensive reference
- Homebrew developers for test ROMs

## Legal

CytraGB is an emulator, not a game. You must provide your own legally-obtained ROM files.

Emulators are legal, but downloading copyrighted ROMs without owning the game is not.

**Use responsibly and respect copyright laws.**

---

Enjoy playing your favorite Game Boy games! 🎮
