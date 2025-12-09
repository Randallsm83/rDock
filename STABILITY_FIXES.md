# rDock Stability Fixes

## Issues Fixed

### 1. Dock Disappearing Randomly
**Problem**: The dock would disappear and not come back even when cursor moved to bottom of screen.

**Fixes**:
- Increased hidden state visibility from 2px to 5px for more reliable cursor detection
- Added window visibility enforcement in `show_dock()` to ensure window is always visible
- Added visibility check during animations to prevent window from being hidden mid-animation
- Fixed race condition in `CursorLeft` event that could trigger hide timer even when dock was already hidden
- Added better state tracking to prevent the dock from getting "stuck" in hidden state

### 2. Windows Taskbar Not Staying Hidden
**Problem**: Windows taskbar would randomly reappear, especially after focus changes or system events.

**Fixes**:
- Added `TASKBAR_CHECK_INTERVAL` (1 second) to periodically re-hide the taskbar
- Implemented `check_taskbar_visibility()` that runs every second to aggressively re-hide taskbar
- Enhanced `set_taskbar_visibility()` to:
  - Use more aggressive hiding by moving taskbar off-screen (`SetWindowPos`)
  - Find ALL secondary taskbars on multi-monitor setups using `FindWindowExW` in a loop
  - Apply both `SW_HIDE` and position offset for maximum effectiveness

## Technical Changes

### New Constants
```rust
const TASKBAR_CHECK_INTERVAL: Duration = Duration::from_secs(1);
```

### New Fields
```rust
last_taskbar_check: Instant,  // Track when we last checked taskbar
```

### Modified Functions

1. **`set_taskbar_visibility()`**
   - Now moves taskbar windows to position (-10000, -10000) in addition to hiding
   - Properly enumerates all secondary taskbars in multi-monitor setups

2. **`check_taskbar_visibility()`** (NEW)
   - Called every redraw
   - Re-hides taskbar every second if configured
   - Ensures taskbar stays hidden even if Windows restores it

3. **`show_dock()`**
   - Now calls `window.set_visible(true)` and `window.focus_window()`
   - Ensures dock stays on top and visible

4. **`update_animations()`**
   - Added `window.set_visible(true)` during position updates
   - Prevents window from disappearing during animation

5. **`CursorLeft` handler**
   - Only starts hide timer if dock is actually visible
   - Prevents race conditions that could hide dock prematurely

## Testing

To test the fixes:

1. **Dock Visibility**:
   - Move cursor to bottom of screen → dock should appear
   - Move cursor away → dock should hide after delay
   - Move cursor back quickly → dock should reappear reliably

2. **Taskbar Hiding**:
   - Launch rdock with `hide_windows_taskbar = true`
   - Switch between windows, open menus, etc.
   - Taskbar should stay hidden (checked every 1 second)

## Building

```powershell
# Stop rdock if running
# Then build:
cargo build --release
```

## Configuration

No config changes needed. The fixes work with existing `config.toml`:

```toml
[dock]
auto_hide = true                # Dock auto-hide behavior
hide_windows_taskbar = true     # Hide Windows taskbar
```

## Future Improvements

If issues persist, consider:
- Reduce `TASKBAR_CHECK_INTERVAL` to check more frequently
- Increase hidden dock visibility beyond 5px
- Add debug logging to track dock state transitions
- Implement Windows message hook to detect taskbar restore events
