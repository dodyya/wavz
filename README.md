#wavz

A real-time audio visualization tool written in Rust that performs Fast Fourier Transform (FFT) analysis on audio streams and WAV files.

## Features

### Audio Visualization Modes

- **Real-time WAV visualization** (`vis`) - Visualize WAV files with synchronized playback and interactive spectrogram
- **Pre-computed visualization** (`precomp`) - Generate spectrograms from WAV files with pre-calculated FFT
- **Microphone visualization** (`mic`) - Real-time FFT visualization from microphone input
- **ASCII microphone** (`asciimic`) - Terminal-based audio visualization
- **WAV playback** (`play`) - Simple audio player for WAV files

### FFT Implementation

- Cooley-Tukey iterative FFT algorithm
- Optimized for power-of-two window sizes (default: 4096 samples)
- Sliding window analysis with configurable step size (default: 256 samples)
- Efficient spectrum magnitude calculation

### Interactive Controls (real-time mode)

- **Space** - Play/pause audio
- **Arrow Left/Right** - Skip backward/forward 0.5 seconds
- **Arrow Up/Down** - Adjust visual sensitivity
- **+/-** - Zoom in/out
- **Q** - Quit

### Color Schemes

Choose from three different color palettes with the `-c` flag:

- **Viridis** (default, `-c 1`) - Perceptually uniform blue-green-yellow
- **Inferno** (`-c 2`) - Dark purple to bright yellow
- **Bone** (`-c 3`) - Grayscale with blue tint

## Usage

```bash
wavz [-c 1|2|3] <command> [file]
```

### Examples

```bash
# Visualize a WAV file with real-time playback
wavz vis audio.wav

# Use inferno color scheme
wavz -c 2 vis audio.wav

# Pre-compute spectrogram
wavz precomp audio.wav

# Real-time microphone visualization
wavz mic

# ASCII terminal visualization
wavz asciimic

# Play audio without visualization
wavz play audio.wav
```

## WAV File Support

- Supports PCM format WAV files
- Handles 1 or 2 channel audio
- Supports multiple format chunk sizes (16, 18, 40 bytes)
- Automatic channel averaging for stereo files
- Memory-mapped file I/O for efficient large file handling

## Technical Details

- **FFT Window Size**: 4096 samples (2048 frequency bins)
- **Step Size**: 256 samples (overlapping windows)
- **Rendering**: GPU-accelerated using `pixels` crate
- **Audio Output**: Cross-platform support via `cpal`
- **Window Management**: `winit` for cross-platform windowing

## Building

```bash
cargo build --release
```

## Dependencies

- `pixels` - GPU-accelerated pixel buffer rendering
- `cpal` - Cross-platform audio I/O
- `winit` - Cross-platform windowing
- `memmap2` - Memory-mapped file I/O
- `bytemuck` - Zero-copy type conversions
- `ringbuf` - Lock-free ring buffer for audio streaming
