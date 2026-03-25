# swoosher

Instant space switcher for macOS. Switches between macOS Spaces without the default animation by posting synthetic dock swipe gestures.

Based on [InstantSpaceSwitcher](https://github.com/jurplel/InstantSpaceSwitcher), rewritten in Rust as a persistent daemon with a tray icon.

## How it works

The daemon listens on a Unix socket for commands and posts synthetic CGEvent dock swipe gestures to the macOS window server. A client (e.g. [Hammerspoon](https://github.com/Hammerspoon/hammerspoon)) connects to the socket and sends commands on keypress. Because the daemon is always running, there is no process spawn overhead per keypress — just a socket write.

Clients can keep a persistent connection open and send multiple commands over time, or connect and disconnect per command. The server handles each connection in its own thread. Idle connections are automatically closed after a configurable timeout (default: 30 seconds), so stale connections don't leak resources.

## Installation

Download the latest `.app.zip` from [Releases](https://github.com/polina4096/swoosher/releases), extract it, and move it to `/Applications`.

Or build from source:

```sh
./scripts/package.sh 0.1.0
open target/swoosher.app
```

## Configuration

The config file is located at `~/.config/swoosher/config.toml` and is created automatically on first launch. Changes are hot-reloaded — no restart needed.

```toml
# Whether to automatically check for updates on startup.
check_updates = true

# Whether to automatically install updates when available.
auto_update = false

# Connection read timeout in seconds. Idle connections are closed after this duration. Set to 0 to disable.
timeout = 30
```

All config options can also be set via environment variables with the `SWOOSHER_CONFIG_` prefix (e.g. `SWOOSHER_CONFIG_TIMEOUT=60`).

## Usage

The daemon exposes a Unix socket (default: `~/.local/state/swoosher/daemon.sock`) that accepts newline-delimited commands:

| Command   | Description                                                               |
| --------- | ------------------------------------------------------------------------- |
| `left`    | Switch one space to the left                                              |
| `right`   | Switch one space to the right                                             |
| `index N` | Switch to space N (1-based)                                               |
| `info`    | Returns `<current> <count>` (1-based current index and total space count) |

### Hammerspoon example

Connects per command for reliability (Unix socket connect is microseconds):

```lua
local swoosherSocketPath = os.getenv("HOME") .. "/.local/state/swoosher/daemon.sock"

local function swoosherSend(cmd)
  local sock = hs.socket.new()
  sock:connect(swoosherSocketPath)
  sock:write(cmd .. "\n", 1, function() sock:disconnect() end)
end

-- Ctrl+Left/Right to switch spaces.
hs.eventtap.new({ hs.eventtap.event.types.keyDown }, function(event)
  local flags = event:getFlags()
  local keyCode = event:getKeyCode()

  local arrows = { [123] = "left", [124] = "right" }
  local arrow = arrows[keyCode]
  if arrow and flags.ctrl and not flags.cmd and not flags.alt and not flags.shift then
    swoosherSend(arrow)
    return true
  end

  return false
end):start()
```

### CLI

```sh
echo "right" | nc -U ~/.local/state/swoosher/daemon.sock
echo "index 3" | nc -U ~/.local/state/swoosher/daemon.sock
```

## Permissions

The daemon requires **Accessibility** permission to create a CGEvent tap and post synthetic events.

Grant it in System Settings > Privacy & Security > Accessibility.

## Limitations

- **No window moving between spaces.** macOS 15 (Sequoia) blocks programmatic window-to-space moves via private APIs unless SIP is partially disabled (like yabai requires). Only space switching is supported.
- **Problems due to how the instant switching works.** Some edge cases result in certain incorrect behavior. For example switching too fast, or in overview mode.
- **Multi-monitor.** Space switching targets the display under the cursor. Multi-monitor setups work, but each display's spaces are indexed independently.
- **Private APIs.** This app relies on undocumented macOS APIs (`CGSMainConnectionID`, `CGSGetActiveSpace`, `CGSCopyManagedDisplaySpaces`, synthetic `CGEvent` fields). These may break in future macOS updates.

## Environment variables

| Variable                    | Description                                                     |
| --------------------------- | --------------------------------------------------------------- |
| `RUST_LOG`                  | Log level (default: `info`)                                     |
| `SWOOSHER_NO_LOGS`          | Disable all logging                                             |
| `SWOOSHER_NO_DISK_LOGS`     | Disable disk logging (stderr only)                              |
| `SWOOSHER_OVERRIDE_LOG_DIR` | Custom log directory (default: `~/.local/share/swoosher/logs/`) |
| `SWOOSHER_OVERRIDE_VERSION` | Override current version for update checking                    |
| `SWOOSHER_CONFIG_*`         | Config overrides (e.g. `SWOOSHER_CONFIG_TIMEOUT=60`)            |

## License

Distributed under the The Unlicense.
