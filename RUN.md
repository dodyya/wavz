# Running Wavez

## 1. Clone the repository

```bash
git clone https://github.com/dodya/wavez.git
cd wavez
```

## 2. Run the code

It is recommended to run in release mode for performance.

```bash
cargo run --release -- COMMAND [FILE_PATH]
```

Examples:

```bash
cargo run --release -- mic
cargo run --release -- visualize foo.wav
```

## Available Commands

- `mic`: Visualize microphone input.
- `asciimic`: Visualize microphone input in ASCII mode.
- `vis <FILE_PATH>`: Visualize a WAV file in real-time.
- `play <FILE_PATH>`: Play a WAV file (audio only).
- `precomp <FILE_PATH>`: Visualize a WAV file (precomputed mode).
