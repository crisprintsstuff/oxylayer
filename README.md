# ✦ OxyLayer

> **High-Performance Clean-Room Recreation of Pixelmator Pro in Pure Rust for Windows, Linux, and macOS.**

[![Rust Build & Tests](https://img.shields.io/badge/Rust-1.80%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20Linux%20%7C%20macOS-informational)](#cross-platform-support)

---

## Overview

**OxyLayer** is an open-source, non-destructive layer-based image editing and digital art suite written in pure Rust. Originally exclusive to macOS, Pixelmator Pro's workflow, sleek single-window aesthetic, and ultra-responsive non-destructive editing are brought natively to **Windows** (via DirectX 12 and Vulkan) as well as Linux and macOS.

### Key Highlights
- **100% Pure Rust**: Zero memory leaks, SIMD parallelism via Rayon, and predictable low-latency performance.
- **Non-Destructive Layer Pipeline**: Unlimited raster layers, vector shapes, masks, and adjustment stacks that never degrade original pixel fidelity.
- **Complete Blend Engine**: 23 standard Photoshop/Pixelmator blend modes computed in 32-bit linear floating point (Normal, Multiply, Screen, Overlay, Color Dodge, Hard Light, Luminosity, etc.).
- **Pro Color Grading & Adjustments**: Real-time Exposure (EV), Brightness, Contrast, White Balance (Temperature/Tint), Vibrance/Saturation, Highlights/Shadows recovery, and Black & White mixer.
- **Convolution & Filter Stack**: Parallel Gaussian blur (separable 1D kernels), unsharp-mask sharpening, pixelate/mosaic, and artistic vignettes.
- **Dynamic Digital Painting**: Parametric dab spacing, hardness/softness smoothstep falloff, flow, opacity, pressure interpolation, and eraser mode.
- **Vector Engine**: Bézier paths and geometric primitives (Rounded Rectangles, Circles, Ellipses, Stars) with subpixel anti-aliasing powered by `kurbo`.
- **Native Non-Destructive Project Archive (`.oxy`)**: High-speed compressed file format using Zstandard (`zstd`) and JSON, plus standard exports for PNG, JPEG, and WebP.
- **Pixelmator-Inspired Dark Interface**: Sleek graphite UI with an infinite zoomable/pannable canvas, tool palette, layer hierarchy, and property inspector.

---

## Architecture & Crates

OxyLayer is organized as a modular Cargo workspace:

```text
oxylayer/
├── Cargo.toml                  # Workspace definition
├── crates/
│   ├── oxylayer-core/          # Document model, 23 blend modes, buffers, layers, history (undo/redo)
│   ├── oxylayer-filter/        # Non-destructive color adjustments & image convolutions
│   ├── oxylayer-brush/         # Dynamic brush engine, smoothing, flow & stamp dabs
│   ├── oxylayer-vector/        # Kurbo Bézier math, parametric vector shapes & rasterizer
│   ├── oxylayer-io/            # Native .oxy format, PNG, JPEG, WebP encoders/decoders
│   └── oxylayer-app/           # Cross-platform desktop application (eframe / egui)
└── README.md
```

| Crate | Purpose | Key Dependencies |
| :--- | :--- | :--- |
| [`oxylayer-core`](crates/oxylayer-core) | Document model, layer tree, blend modes, transforms, undo/redo manager | `rayon`, `glam`, `uuid`, `serde` |
| [`oxylayer-filter`](crates/oxylayer-filter) | Color adjustments (Exposure, Curves, HSL) and convolution filters (Blur, Sharpen) | `rayon`, `glam` |
| [`oxylayer-brush`](crates/oxylayer-brush) | Parametric brush dab engine, continuous stroke interpolation | `glam` |
| [`oxylayer-vector`](crates/oxylayer-vector) | Vector elements, Bézier paths, and anti-aliased rasterization | `kurbo`, `glam` |
| [`oxylayer-io`](crates/oxylayer-io) | Native `.oxy` compressed archive and standard image codecs | `image`, `zstd`, `serde_json` |
| [`oxylayer-app`](crates/oxylayer-app) | Desktop studio with dark theme UI, inspector, tools, and canvas | `eframe`, `egui` |

---

## Getting Started

### Prerequisites
- [Rust Toolchain](https://www.rust-lang.org/tools/install) (1.80 or later recommended).

### Building and Running
To launch the desktop application:
```bash
cargo run --release -p oxylayer-app
```

To run all unit and integration tests across every crate:
```bash
cargo test
```

---

## Keyboard & Mouse Shortcuts

| Action | Shortcut |
| :--- | :--- |
| **Pan Canvas** | `Space + Click & Drag` or `Middle Click & Drag` |
| **Zoom Canvas** | `Scroll Wheel` or `Fit / 100%` buttons |
| **Brush Tool** | `B` (via Toolbar) |
| **Eraser Tool** | `E` (via Toolbar) |
| **Vector Shapes** | `Rectangle`, `Circle`, `Star` (Click to place) |
| **Undo / Redo** | `↶ Undo` / `↷ Redo` buttons (or 50-step History Stack) |

---

## Cross-Platform Support

| Platform | Rendering Backend | Notes |
| :--- | :--- | :--- |
| **Windows 10/11** | DirectX 12 / Vulkan | Full native support with DirectML ML-acceleration roadmap |
| **Linux** | Vulkan / Wayland / X11 | Tested on Ubuntu, Fedora, Arch, and WSL2 |
| **macOS** | Metal | Native Retina display scaling |

---

## License

Dual-licensed under either:
- **MIT License** ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
- **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

at your option.
