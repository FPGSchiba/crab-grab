# crab-grab

**CrabGrab** is a lightning-fast, non-intrusive screenshot utility written in **Rust**.

Built on top of `wgpu` and `eframe`, it runs silently in your system tray using minimal resources. When triggered, it overlays a high-performance rendering layer for pixel-perfect selections.

## Features

* **Instant Capture:** Zero-latency overlay powered by the GPU.
* **Custom Cursors:** Includes a thematic "Crab Claw" cursor for precise pixel selection.
* **Auto-Save:** Automatically saves screenshots to your preferred directory with timestamps.
* **Audio Feedback:** Satisfying "Focus" and "Shutter" sound effects (embedded in the binary).
* **Configurable:** Change hotkeys, save paths, and behaviors via a native Settings UI.
* **Auto-Start:** Cross-platform support to launch automatically on system login.
* **Clipboard Integration:** Copied to clipboard immediately upon release.
* **Invisible:** Lives in the System Tray; no annoying taskbar windows.

## Installation

### Windows

Download the latest `.msi` installer from the [Releases Page](https://github.com/FPGSchiba/crab-grab/releases).
Running the installer will automatically set up the application and add it to your Start Menu.

### Linux & macOS

Check the [Releases Page](https://github.com/FPGSchiba/crab-grab/releases) for `.deb` packages or `.app` bundles, or build from source below.

## Usage

1.  **Launch** CrabGrab. It will minimize to the System Tray (near your clock).
2.  **Trigger** the capture hotkey (Default: `Ctrl + Shift + G`).
3.  **Drag** to select an area on any monitor.
4.  **Release** to capture.
    * The image is copied to your **Clipboard**.
    * If enabled, the image is saved to your **Save Directory**.

### Default Shortcuts

| Action             | Shortcut               |
|:-------------------|:-----------------------|
| **Start Capture**  | `Ctrl` + `Shift` + `G` |
| **Open Settings**  | `Ctrl` + `Shift` + `S` |
| **Cancel Capture** | `Esc`                  |

*Note: You can record new hotkeys in the Settings menu.*

## Building from Source

You need **Rust** installed.

```bash
# 1. Clone the repository
git clone https://github.com/FPGSchiba/crab-grab.git
cd crab-grab

# 2. Run in Release mode (Recommended for performance)
cargo run --release
```

### Linux Requirements

If you are on Linux, you will need the standard GTK and X11 development libraries:

```bash
sudo apt-get install libgtk-3-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libx11-dev libasound2-dev
```

## Architecture

CrabGrab is built with a modern Rust stack:

* **GUI:** `eframe` (egui) + `wgpu` (WebGPU backend)
* **System:** `tray-icon` & `global-hotkey`
* **Async I/O:** `rayon` for non-blocking file saving.
* **Audio:** `rodio` for sound effects.
* **State:** `serde_json` for configuration persistence.

## Roadmap

* [x] Basic Snapping & Clipboard
* [x] Settings UI & Hotkey Recorder
* [x] Audio & Custom Cursors
* [x] Windows MSI Installer
* [ ] Image Editor (Draw arrows/blur)
* [ ] OCR (Text recognition)

