# Installation

## From Source

```bash
git clone https://github.com/neur0map/polymaster.git
cd polymaster
cargo build --release
```

The binary will be located at `target/release/wwatcher`

## System-wide Installation

```bash
cargo install --path .
```

This installs the binary to your cargo bin directory (typically `~/.cargo/bin`), which should be in your PATH.

## Quick Test

```bash
# Check if installation worked
wwatcher --help

# View status
wwatcher status
```

## First Run

1. **No Setup Required**: The tool works immediately with public APIs
   ```bash
   wwatcher watch
   ```

2. **Optional**: Configure Kalshi credentials for higher rate limits
   ```bash
   wwatcher setup
   ```

## Usage

```bash
# Default: $25k threshold, 5-second polling
wwatcher watch

# Custom threshold
wwatcher watch -t 50000

# Custom interval
wwatcher watch -i 30

# Both
wwatcher watch -t 100000 -i 60
```

## Running in Background

```bash
# With output to log file
nohup wwatcher watch > whale-alerts.log 2>&1 &

# Save the process ID
echo $! > wwatcher.pid

# To stop later
kill $(cat wwatcher.pid)
```

## Monitoring the Log

```bash
# Follow the log in real-time
tail -f whale-alerts.log

# Search for specific markets
grep "Market:" whale-alerts.log

# View only anomalies
grep "ANOMALY" whale-alerts.log
```

## Audio Alerts

The tool plays a system beep on every whale detection. This works on:
- macOS (Terminal beep)
- Linux (System bell)
- Windows (Console beep)

If you don't hear the beep, check your system sound settings.

## Updating

```bash
cd polymaster
git pull
cargo build --release
```

## Requirements

- **Rust**: 1.70 or higher
- **Internet**: Active connection for API access
- **Terminal**: Any modern terminal emulator

## Troubleshooting

### Build Issues

```bash
# Update Rust
rustup update

# Clean build
cargo clean
cargo build --release
```

### Permission Issues

```bash
# Make binary executable
chmod +x target/release/wwatcher
```

### API Issues

- **Polymarket**: No authentication needed, should work immediately
- **Kalshi**: Public endpoint available, authentication optional

### No Audio Alert

- Check system volume
- Verify terminal bell is enabled
- Test with: `echo -e '\a'`

## Platform-Specific Notes

### macOS

Works out of the box. Audio alert uses system beep.

### Linux

May require `libasound2-dev` for some audio features:
```bash
sudo apt-get install libasound2-dev
```

### Windows

Build with:
```bash
cargo build --release
```

Binary will be at `target\release\wwatcher.exe`

## Uninstall

```bash
# If installed system-wide
cargo uninstall wwatcher

# Remove repository
rm -rf polymaster

# Remove config (optional)
rm ~/.config/wwatcher/config.json
```
