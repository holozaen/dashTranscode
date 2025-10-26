# DASH Transcoding Service

A Linux systemd service that monitors a folder for video files and automatically converts them to DASH (Dynamic Adaptive Streaming over HTTP) format.

## Features

- **Automatic Monitoring**: Watches a configured folder for new video files
- **DASH Conversion**: Converts videos to H.264/AAC and segments them for DASH streaming
- **Organized Output**: Each video gets its own subfolder with manifest and segment files
- **Systemd Integration**: Runs as a service with automatic restart on failure
- **Configurable**: All settings via environment variables in the systemd service file
- **Logging**: Integrated with systemd journal for easy log viewing

## Prerequisites

### System Requirements

- Linux with systemd
- Rust 1.70+ (for building)
- FFmpeg with libx264 and AAC support

### Install FFmpeg

```bash
sudo apt-get update
sudo apt-get install ffmpeg
```

## Installation

### 1. Build the Application

```bash
cargo build --release
```

### 2. Create Service User

```bash
sudo useradd -r -s /bin/false dashtranscode
```

### 3. Create Watch Directory

```bash
sudo mkdir -p /var/watch/videos
sudo chown dashtranscode:dashtranscode /var/watch/videos
sudo chmod 755 /var/watch/videos
```

### 4. Install Binary

```bash
sudo cp target/release/dashTranscode /usr/local/bin/dashtranscode
sudo chmod +x /usr/local/bin/dashtranscode
```

### 5. Install Systemd Service

```bash
sudo cp dashtranscode.service /etc/systemd/system/
sudo systemctl daemon-reload
```

### 6. Enable and Start Service

```bash
sudo systemctl enable dashtranscode
sudo systemctl start dashtranscode
```

### 7. Verify Service Status

```bash
sudo systemctl status dashtranscode
```

## Configuration

All configuration is done through environment variables in the systemd service file (`/etc/systemd/system/dashtranscode.service`).

### Available Configuration Options

| Environment Variable | Description | Default |
|---------------------|-------------|---------|
| `WATCH_FOLDER` | Directory to monitor for video files | `/var/watch/videos` |
| `VIDEO_EXTENSIONS` | Comma-separated list of video file extensions | `mp4,avi,mkv,mov,wmv,flv,webm` |
| `FFMPEG_PATH` | Path to FFmpeg binary | `/usr/bin/ffmpeg` |
| `FFMPEG_PRESET` | FFmpeg encoding preset (ultrafast to veryslow) | `medium` |
| `FFMPEG_CRF` | Constant Rate Factor for quality (0-51, lower=better) | `23` |
| `AUDIO_BITRATE` | Audio bitrate | `128k` |
| `SEGMENT_DURATION` | DASH segment duration in seconds | `4` |
| `RUST_LOG` | Logging level (error, warn, info, debug, trace) | `info` |

### Modifying Configuration

Edit the service file:

```bash
sudo systemctl edit --full dashtranscode
```

Or directly edit:

```bash
sudo nano /etc/systemd/system/dashtranscode.service
```

After changes, reload and restart:

```bash
sudo systemctl daemon-reload
sudo systemctl restart dashtranscode
```

## Usage

### Testing

Drop a video file into the watch folder:

```bash
sudo cp /path/to/video.mp4 /var/watch/videos/
```

### Output Structure

For a video file named `example.mp4`, the service creates:

```
/var/watch/videos/
├── example.mp4                           # Original file
└── example/                              # Output folder
    ├── manifest.mpd                      # DASH manifest
    ├── init-stream0.m4s                  # Video initialization segment
    ├── init-stream1.m4s                  # Audio initialization segment
    ├── chunk-stream0-00001.m4s           # Video chunk 1
    ├── chunk-stream0-00002.m4s           # Video chunk 2
    ├── chunk-stream1-00001.m4s           # Audio chunk 1
    └── chunk-stream1-00002.m4s           # Audio chunk 2
```

### Viewing Logs

Follow logs in real-time:

```bash
sudo journalctl -u dashtranscode -f
```

View recent logs:

```bash
sudo journalctl -u dashtranscode -n 50
```

View logs from today:

```bash
sudo journalctl -u dashtranscode --since today
```

## Service Management

### Start Service

```bash
sudo systemctl start dashtranscode
```

### Stop Service

```bash
sudo systemctl stop dashtranscode
```

### Restart Service

```bash
sudo systemctl restart dashtranscode
```

### Check Status

```bash
sudo systemctl status dashtranscode
```

### Enable Auto-Start on Boot

```bash
sudo systemctl enable dashtranscode
```

### Disable Auto-Start

```bash
sudo systemctl disable dashtranscode
```

## Troubleshooting

### Service Won't Start

Check the logs:

```bash
sudo journalctl -u dashtranscode -n 100
```

Verify FFmpeg is installed:

```bash
which ffmpeg
ffmpeg -version
```

### Permission Issues

Ensure the watch folder has correct permissions:

```bash
sudo chown dashtranscode:dashtranscode /var/watch/videos
sudo chmod 755 /var/watch/videos
```

### Files Not Being Processed

Check if the service is running:

```bash
sudo systemctl is-active dashtranscode
```

Verify the watch folder path in the service configuration:

```bash
systemctl cat dashtranscode | grep WATCH_FOLDER
```

Check file extensions are included:

```bash
systemctl cat dashtranscode | grep VIDEO_EXTENSIONS
```

## Uninstallation

```bash
# Stop and disable service
sudo systemctl stop dashtranscode
sudo systemctl disable dashtranscode

# Remove service file
sudo rm /etc/systemd/system/dashtranscode.service
sudo systemctl daemon-reload

# Remove binary
sudo rm /usr/local/bin/dashtranscode

# Remove user (optional)
sudo userdel dashtranscode

# Remove watch folder (optional)
sudo rm -rf /var/watch/videos
```

## Development

### Building

```bash
cargo build
```

### Running Locally (for testing)

```bash
# Set environment variables
export WATCH_FOLDER=/tmp/test-videos
export RUST_LOG=debug

# Create test directory
mkdir -p /tmp/test-videos

# Run the application
cargo run
```

### Updating After Code Changes

```bash
cargo build --release
sudo systemctl stop dashtranscode
sudo cp target/release/dashTranscode /usr/local/bin/dashtranscode
sudo systemctl start dashtranscode
```

## License

This project is provided as-is for your use.

## Support

For issues or questions, check the systemd journal logs for detailed information about what the service is doing.