#![allow(dead_code)]

//! Headless microbenchmark for the dock's per-frame software renderer.
//!
//! `Renderer::render` is the hot path that runs every frame while the dock is
//! visible (magnification animation, alpha-blended icon scaling, anti-aliased
//! rounded background, reflections). This benchmark drives it deterministically:
//! a fixed set of items with a procedurally-generated icon fixture, swept by a
//! synthetic magnification cursor over a fixed number of frames.
//!
//! It links the production `config` and `renderer` modules directly (via
//! `#[path]`) so it measures the real code, and is gated behind the `bench`
//! cargo feature so normal builds never compile it.
//!
//! Output: `METRIC` lines on stdout. Primary metric is microseconds per frame
//! (lower is better), taken as the minimum over many timed batches to suppress
//! scheduler noise.

#[path = "../src/config.rs"]
mod config;
#[path = "../src/renderer.rs"]
mod renderer;

use std::hint::black_box;
use std::path::PathBuf;
use std::time::Instant;

use config::{Config, DockItem, DockSettings};
use renderer::Renderer;

// Deterministic workload parameters. Changing these changes the workload, so
// they are fixed constants, never read from the environment.
const ICON_FIXTURE_PX: u32 = 256;
const WARMUP_FRAMES: u32 = 400;
const BATCHES: u32 = 12;
const FRAMES_PER_BATCH: u32 = 250;

/// Build a deterministic RGBA icon fixture and write it as a PNG. Content is a
/// fixed procedural pattern so every run loads identical pixels.
fn write_icon_fixture() -> PathBuf {
    let path = std::env::temp_dir().join("rdock_bench_icon.png");
    let img = image::RgbaImage::from_fn(ICON_FIXTURE_PX, ICON_FIXTURE_PX, |x, y| {
        // Radial + checker pattern with a soft circular alpha mask, so the
        // scaler has real edges and transparency to work with.
        let cx = ICON_FIXTURE_PX as f32 / 2.0;
        let cy = ICON_FIXTURE_PX as f32 / 2.0;
        let dx = x as f32 - cx;
        let dy = y as f32 - cy;
        let dist = (dx * dx + dy * dy).sqrt();
        let radius = ICON_FIXTURE_PX as f32 / 2.0;
        let r = (x * 255 / ICON_FIXTURE_PX) as u8;
        let g = (y * 255 / ICON_FIXTURE_PX) as u8;
        let b = (((x / 16) + (y / 16)) % 2 * 200 + 40) as u8;
        let a = if dist <= radius {
            let edge = ((radius - dist) / 6.0).clamp(0.0, 1.0);
            (edge * 255.0) as u8
        } else {
            0
        };
        image::Rgba([r, g, b, a])
    });
    img.save(&path).expect("write icon fixture png");
    path
}

/// Construct a representative dock: a mix of icon items, separators, and items
/// without icons (placeholder path). Half are marked "running".
fn build_items(icon: &PathBuf) -> Vec<DockItem> {
    let mut items = Vec::new();
    for i in 0..16usize {
        if i == 5 || i == 11 {
            items.push(DockItem::new_separator());
            continue;
        }
        items.push(DockItem {
            name: format!("item{i}"),
            path: PathBuf::new(),
            icon: Some(icon.clone()),
            args: Vec::new(),
            separator: false,
            special: None,
        });
    }
    items
}

/// Magnification scales for a given frame: a cursor sweeps left-to-right across
/// the dock; each item scales up with a smooth falloff near the cursor. This
/// continuously varies the per-icon target size, stressing the bicubic scaler.
fn scales_for_frame(frame: u32, n: usize, magnification: f32) -> Vec<f32> {
    // Cursor position in item-index space, sweeping 0..n over a fixed period.
    let period = 120.0_f32;
    let phase = (frame as f32 % period) / period; // 0..1
    let cursor = phase * n as f32;
    let spread = 1.6_f32;
    (0..n)
        .map(|i| {
            let d = (i as f32 - cursor) / spread;
            let falloff = 1.0 / (1.0 + d * d);
            1.0 + (magnification - 1.0) * falloff
        })
        .collect()
}

fn main() {
    let icon = write_icon_fixture();
    let items = build_items(&icon);

    let config = Config {
        dock: DockSettings::default(),
        items: items.clone(),
    };

    let renderer = Renderer::new(&config, &items).expect("build renderer");
    let n = items.len();
    let magnification = config.dock.magnification;

    let mut buffer = vec![0u32; (renderer.width * renderer.height) as usize];
    let running: Vec<bool> = (0..n).map(|i| i % 2 == 0).collect();

    // Warmup: populate caches, let the CPU settle, and ensure first-touch of the
    // buffer pages happens outside the measured region.
    for f in 0..WARMUP_FRAMES {
        let scales = scales_for_frame(f, n, magnification);
        renderer.render(&mut buffer, &items, &running, None, &scales, None);
        black_box(buffer.as_ptr());
    }

    // Timed batches. Report the minimum per-frame time (most stable estimator
    // for a CPU microbenchmark) plus mean for context.
    let mut min_batch_ns = u128::MAX;
    let mut total_ns: u128 = 0;
    let mut frame: u32 = 0;
    for _ in 0..BATCHES {
        let start = Instant::now();
        for _ in 0..FRAMES_PER_BATCH {
            let scales = scales_for_frame(black_box(frame), n, magnification);
            renderer.render(&mut buffer, &items, &running, None, &scales, None);
            black_box(buffer.as_ptr());
            frame = frame.wrapping_add(1);
        }
        let elapsed = start.elapsed().as_nanos();
        min_batch_ns = min_batch_ns.min(elapsed);
        total_ns += elapsed;
    }

    let min_us_per_frame = min_batch_ns as f64 / FRAMES_PER_BATCH as f64 / 1000.0;
    let mean_us_per_frame =
        total_ns as f64 / (BATCHES as f64 * FRAMES_PER_BATCH as f64) / 1000.0;
    let min_fps = 1_000_000.0 / min_us_per_frame;

    // Primary metric first. Lower is better.
    println!("METRIC render_us_per_frame={min_us_per_frame:.3}");
    println!("METRIC render_mean_us_per_frame={mean_us_per_frame:.3}");
    println!("METRIC render_min_fps={min_fps:.1}");
}
