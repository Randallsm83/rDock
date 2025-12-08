use crate::config::{parse_hex_color, parse_hex_rgb, Config, DockItem, Spacing, ItemSpacing};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;

pub struct Renderer {
    pub width: u32,
    pub height: u32,
    pub icon_size: u32,
    pub spacing: ItemSpacing,
    pub padding: Spacing,
    pub negative_vertical_offset: i32,
    pub corner_radius: u32,
    pub bg_color: u32,
    pub indicator_color: (u8, u8, u8),
    icons: HashMap<PathBuf, Vec<u32>>,
    icons_large: HashMap<PathBuf, Vec<u32>>,
}

impl Renderer {
    pub fn new(config: &Config, items: &[DockItem]) -> Result<Self> {
        let icon_size = config.dock.icon_size;
        let spacing = config.dock.spacing.clone();
        let padding = config.dock.padding.clone();
        
        // Calculate dock dimensions
        let num_items = items.len() as u32;
        let mag_extra_width = (icon_size as f32 * 0.4) as u32;
        let reflection_h = (icon_size as f32 * 0.2) as u32;
        let width = if num_items > 0 {
            (num_items * icon_size) + ((num_items - 1) * spacing.x) + padding.left + padding.right + mag_extra_width
        } else {
            padding.left + padding.right
        };
        let height = icon_size + padding.top + padding.bottom + reflection_h + 4;

        let bg_color = parse_hex_color(&config.dock.background_color, config.dock.background_opacity);
        let indicator_color = parse_hex_rgb(&config.dock.indicator_color);

        let mut renderer = Self {
            width,
            height,
            icon_size,
            spacing,
            padding,
            negative_vertical_offset: config.dock.negative_vertical_offset,
            corner_radius: config.dock.corner_radius,
            bg_color,
            indicator_color,
            icons: HashMap::new(),
            icons_large: HashMap::new(),
        };

        // Pre-load icons at very high resolution for quality scaling
        // Use 6x size to ensure we have enough data for sharp rendering
        let base_load_size = (icon_size * 6).max(384);
        for item in items {
            if let Some(icon_path) = &item.icon {
                if let Ok(pixels) = renderer.load_icon(icon_path, base_load_size) {
                    renderer.icons.insert(icon_path.clone(), pixels);
                }
            }
        }

        Ok(renderer)
    }

    fn load_icon(&self, path: &PathBuf, size: u32) -> Result<Vec<u32>> {
        use image::DynamicImage;
        use std::io::{BufReader, Seek, SeekFrom};
        
        let file = std::fs::File::open(path)
            .with_context(|| format!("Failed to open icon: {}", path.display()))?;
        
        // Try to load as ICO first to get best resolution
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        let mut img: DynamicImage = if ext == "ico" {
            let mut reader = BufReader::new(file);
            if let Ok(ico) = ico::IconDir::read(&mut reader) {
                // Find the largest icon entry (prefer 256x256 or larger)
                let best = ico.entries().iter()
                    .max_by_key(|e| e.width() as u32 * e.height() as u32);
                
                if let Some(entry) = best {
                    if let Ok(decoded) = entry.decode() {
                        let rgba = decoded.rgba_data();
                        let w = decoded.width();
                        let h = decoded.height();
                        if let Some(img_buf) = image::RgbaImage::from_raw(w, h, rgba.to_vec()) {
                            DynamicImage::ImageRgba8(img_buf)
                        } else {
                            // Fallback to standard image loading
                            reader.seek(SeekFrom::Start(0))?;
                            image::load(reader, image::ImageFormat::Ico)?
                        }
                    } else {
                        reader.seek(SeekFrom::Start(0))?;
                        image::load(reader, image::ImageFormat::Ico)?
                    }
                } else {
                    reader.seek(SeekFrom::Start(0))?;
                    image::load(reader, image::ImageFormat::Ico)?
                }
            } else {
                reader.seek(SeekFrom::Start(0))?;
                image::load(reader, image::ImageFormat::Ico)?
            }
        } else {
            image::open(path)
                .with_context(|| format!("Failed to load icon: {}", path.display()))?
        };
        
        // If source is smaller than target, use Mitchell for upscaling (sharper than Lanczos)
        // If downscaling, use Lanczos3 for best quality
        let current_size = img.width().min(img.height());
        let filter = if current_size < size {
            // Upscaling - use Mitchell (CatmullRom) for sharper results
            image::imageops::FilterType::CatmullRom
        } else {
            // Downscaling - use Lanczos3
            image::imageops::FilterType::Lanczos3
        };
        
        img = img.resize_exact(size, size, filter);

        let mut rgba = img.to_rgba8();
        
        // Apply subtle sharpening to improve edge clarity
        rgba = sharpen_image(rgba, 0.3);
        
        let pixels: Vec<u32> = rgba
            .chunks_exact(4)
            .map(|c| {
                let a = c[3] as u32;
                let r = c[0] as u32;
                let g = c[1] as u32;
                let b = c[2] as u32;
                (a << 24) | (r << 16) | (g << 8) | b
            })
            .collect();
        
        Ok(pixels)
    }

    /// drag_state: Option<(from_idx, to_idx, cursor_x)>
    pub fn render(&self, buffer: &mut [u32], items: &[DockItem], running: &[bool], _hovered: Option<usize>, scales: &[f32], drag_state: Option<(usize, usize, f32)>) {
        let width = self.width as usize;
        let height = self.height as usize;

        buffer.fill(0);

        // Draw background
        self.draw_background(buffer, width, height);

        // Extract drag info
        let (drag_from, drag_to, drag_cursor_x) = drag_state.unwrap_or((usize::MAX, usize::MAX, -1000.0));
        let is_dragging = drag_state.is_some();

        // First pass: calculate total width with current scales to center properly
        let mut total_width: f32 = 0.0;
        for i in 0..items.len() {
            if is_dragging && i == drag_from {
                continue; // Don't count dragged item in normal layout
            }
            let scale = scales.get(i).copied().unwrap_or(1.0);
            if items[i].is_separator() {
                total_width += (self.icon_size / 3) as f32;
            } else {
                total_width += self.icon_size as f32 * scale;
            }
            if i < items.len() - 1 {
                total_width += self.spacing.x as f32;
            }
        }
        
        // Add gap for drop position if dragging
        if is_dragging {
            total_width += self.spacing.x as f32; // Gap where item will be dropped
        }
        
        // Center the icons
        let start_x = (self.width as f32 - total_width) / 2.0;
        let base_y = self.padding.top as f32;
        
        let mut x_pos = start_x;
        
        // Store icon positions for reflection pass
        let mut icon_draws: Vec<(u32, u32, u32, &Vec<u32>, u32)> = Vec::new();
        
        // Track position for drop indicator
        let mut rendered_count = 0;
        
        for (i, item) in items.iter().enumerate() {
            // Skip the dragged item in normal rendering
            if is_dragging && i == drag_from {
                continue;
            }
            
            // Insert gap at drop position
            if is_dragging && rendered_count == drag_to && drag_to != drag_from {
                // Draw drop indicator line
                self.draw_drop_indicator(buffer, width, x_pos as u32, self.padding.top, self.icon_size);
                x_pos += self.spacing.x as f32;
            }
            
            let scale = scales.get(i).copied().unwrap_or(1.0);
            let scaled_size = (self.icon_size as f32 * scale) as u32;
            
            // Icons rise up when scaled
            let y_lift = (scale - 1.0) * self.icon_size as f32 * 1.5;
            let x = x_pos as u32;
            let y = (base_y - y_lift).max(2.0) as u32;
            
            // Check if this is a separator
            if item.is_separator() {
                self.draw_separator(buffer, width, x, self.padding.top, self.icon_size);
                x_pos += (self.icon_size / 3) as f32 + self.spacing.x as f32;
                rendered_count += 1;
                continue;
            }
            
            // Glow behind magnified icons
            if scale > 1.05 {
                let glow_intensity = ((scale - 1.0) * 2.0).min(1.0);
                self.draw_glow_scaled(buffer, width, x + scaled_size / 2, y + scaled_size / 2, scaled_size, glow_intensity);
            }
            
            // Draw icon
            if let Some(icon_path) = &item.icon {
                let src_size = (self.icon_size * 6).max(384);
                let pixels = if let Some(p) = self.icons.get(icon_path) {
                    p
                } else {
                    self.draw_placeholder(buffer, width, x, y, scaled_size);
                    x_pos += scaled_size as f32 + self.spacing.x as f32;
                    rendered_count += 1;
                    continue;
                };
                
                self.draw_icon_bicubic(buffer, width, pixels, src_size, x, y, scaled_size);
                icon_draws.push((x, y, scaled_size, pixels, src_size));
            } else {
                self.draw_placeholder(buffer, width, x, y, scaled_size);
            }

            // Running indicator
            if running.get(i).copied().unwrap_or(false) {
                let ind_x = x + scaled_size / 2;
                let ind_y = if self.negative_vertical_offset > 0 {
                    (self.height as i32 - 5 - self.negative_vertical_offset).max(self.padding.top as i32 + self.icon_size as i32) as u32
                } else {
                    self.height - 5
                };
                self.draw_indicator_glow(buffer, width, ind_x, ind_y);
            }
            
            x_pos += scaled_size as f32 + self.spacing.x as f32;
            rendered_count += 1;
        }
        
        // Draw drop indicator at end if needed
        if is_dragging && drag_to >= rendered_count {
            self.draw_drop_indicator(buffer, width, x_pos as u32, self.padding.top, self.icon_size);
        }
        
        // Draw reflections (using bicubic for quality)
        for (x, y, scaled_size, pixels, src_size) in icon_draws {
            let reflection_y = y + scaled_size + 2;
            self.draw_reflection_bicubic(buffer, width, pixels, src_size, x, reflection_y, scaled_size);
        }
        
        // Draw dragged icon following cursor
        if is_dragging && drag_from < items.len() {
            let item = &items[drag_from];
            if !item.is_separator() {
                if let Some(icon_path) = &item.icon {
                    if let Some(pixels) = self.icons.get(icon_path) {
                        let src_size = (self.icon_size * 6).max(384);
                        let drag_size = self.icon_size;
                        let drag_x = (drag_cursor_x - drag_size as f32 / 2.0).max(0.0) as u32;
                        let drag_y = self.padding.top;
                        
                        // Draw with slight transparency effect (draw darker/lighter)
                        self.draw_icon_bicubic(buffer, width, pixels, src_size, drag_x, drag_y, drag_size);
                    }
                }
            }
        }
    }

    fn draw_background(&self, buffer: &mut [u32], width: usize, height: usize) {
        let r = self.corner_radius as i32;
        let base_a = ((self.bg_color >> 24) & 0xFF) as f32;
        let base_r = ((self.bg_color >> 16) & 0xFF) as f32;
        let base_g = ((self.bg_color >> 8) & 0xFF) as f32;
        let base_b = (self.bg_color & 0xFF) as f32;

        for y in 0..height {
            let yf = y as f32 / height as f32;
            
            // Glass effect: lighter band at top, gradient down
            let top_highlight = if yf < 0.15 {
                0.25 * (1.0 - yf / 0.15) // Bright highlight at very top
            } else {
                0.0
            };
            
            // Subtle overall gradient
            let grad = 1.0 + (1.0 - yf) * 0.08 + top_highlight;
            let gr = (base_r * grad).min(255.0) as u32;
            let gg = (base_g * grad).min(255.0) as u32;
            let gb = (base_b * grad).min(255.0) as u32;
            
            for x in 0..width {
                let idx = y * width + x;
                let xi = x as i32;
                let yi = y as i32;
                let w = width as i32;
                let h = height as i32;

                // Anti-aliased rounded corners
                let dist = if xi < r && yi < r {
                    let dx = (r - xi) as f32;
                    let dy = (r - yi) as f32;
                    (dx * dx + dy * dy).sqrt() - r as f32
                } else if xi >= w - r && yi < r {
                    let dx = (xi - (w - r - 1)) as f32;
                    let dy = (r - yi) as f32;
                    (dx * dx + dy * dy).sqrt() - r as f32
                } else if xi < r && yi >= h - r {
                    let dx = (r - xi) as f32;
                    let dy = (yi - (h - r - 1)) as f32;
                    (dx * dx + dy * dy).sqrt() - r as f32
                } else if xi >= w - r && yi >= h - r {
                    let dx = (xi - (w - r - 1)) as f32;
                    let dy = (yi - (h - r - 1)) as f32;
                    (dx * dx + dy * dy).sqrt() - r as f32
                } else {
                    -1.0
                };

                if dist < 1.0 {
                    let alpha = if dist < 0.0 {
                        base_a
                    } else {
                        base_a * (1.0 - dist)
                    };
                    
                    buffer[idx] = ((alpha as u32) << 24) | (gr << 16) | (gg << 8) | gb;
                }
            }
        }
    }

    fn draw_glow_scaled(&self, buffer: &mut [u32], buf_width: usize, cx: u32, cy: u32, size: u32, intensity: f32) {
        let (ir, ig, ib) = self.indicator_color;
        let radius = (size as f32 * 0.6) as i32;
        
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let dist_sq = dx * dx + dy * dy;
                if dist_sq <= radius * radius {
                    let x = cx as i32 + dx;
                    let y = cy as i32 + dy;
                    if x >= 0 && y >= 0 {
                        let idx = y as usize * buf_width + x as usize;
                        if idx < buffer.len() {
                            let dist = (dist_sq as f32).sqrt();
                            let falloff = 1.0 - (dist / radius as f32);
                            let alpha = (falloff * falloff * 50.0 * intensity) as u32;
                            if alpha > 0 {
                                let glow = (alpha << 24) | ((ir as u32) << 16) | ((ig as u32) << 8) | (ib as u32);
                                buffer[idx] = alpha_blend(buffer[idx], glow);
                            }
                        }
                    }
                }
            }
        }
    }

    fn draw_reflection_bicubic(&self, buffer: &mut [u32], buf_width: usize, pixels: &[u32], src_size: u32, x: u32, y: u32, dst_size: u32) {
        let scale = src_size as f32 / dst_size as f32;
        let src_w = src_size as usize;
        let reflection_height = (dst_size as f32 * 0.35) as u32;
        
        for iy in 0..reflection_height.min(dst_size) {
            let fade = 1.0 - (iy as f32 / reflection_height as f32);
            let row_alpha = (fade * fade * 60.0) as u32;
            
            for ix in 0..dst_size {
                let src_x = ix as f32 * scale;
                let src_y = (dst_size - 1 - iy) as f32 * scale; // Flip Y
                
                let pixel = bicubic_sample(pixels, src_w, src_x, src_y);
                
                let dst_x = x as usize + ix as usize;
                let dst_y = y as usize + iy as usize;
                let dst_idx = dst_y * buf_width + dst_x;

                if dst_idx < buffer.len() {
                    let src_alpha = (pixel >> 24) & 0xFF;
                    if src_alpha > 0 {
                        let final_alpha = (src_alpha * row_alpha / 255).min(row_alpha);
                        let r = (pixel >> 16) & 0xFF;
                        let g = (pixel >> 8) & 0xFF;
                        let b = pixel & 0xFF;
                        let reflected = (final_alpha << 24) | (r << 16) | (g << 8) | b;
                        buffer[dst_idx] = alpha_blend(buffer[dst_idx], reflected);
                    }
                }
            }
        }
    }
    
    fn draw_reflection(&self, buffer: &mut [u32], buf_width: usize, pixels: &[u32], src_size: u32, x: u32, y: u32, dst_size: u32) {
        let scale = src_size as f32 / dst_size as f32;
        let src_w = src_size as usize;
        let reflection_height = (dst_size as f32 * 0.35) as u32;
        
        for iy in 0..reflection_height.min(dst_size) {
            let fade = 1.0 - (iy as f32 / reflection_height as f32);
            let row_alpha = (fade * fade * 60.0) as u32;
            
            for ix in 0..dst_size {
                let src_xf = ix as f32 * scale;
                let src_yf = (dst_size - 1 - iy) as f32 * scale; // Flip Y
                
                let x0 = src_xf as usize;
                let y0 = src_yf as usize;
                let x1 = (x0 + 1).min(src_w - 1);
                let y1 = (y0 + 1).min(src_w - 1);
                
                let fx = src_xf - x0 as f32;
                let fy = src_yf - y0 as f32;
                
                let p00 = pixels.get(y0 * src_w + x0).copied().unwrap_or(0);
                let p10 = pixels.get(y0 * src_w + x1).copied().unwrap_or(0);
                let p01 = pixels.get(y1 * src_w + x0).copied().unwrap_or(0);
                let p11 = pixels.get(y1 * src_w + x1).copied().unwrap_or(0);
                
                let pixel = bilinear_blend(p00, p10, p01, p11, fx, fy);
                
                let dst_x = x as usize + ix as usize;
                let dst_y = y as usize + iy as usize;
                let dst_idx = dst_y * buf_width + dst_x;

                if dst_idx < buffer.len() {
                    let src_alpha = (pixel >> 24) & 0xFF;
                    if src_alpha > 0 {
                        let final_alpha = (src_alpha * row_alpha / 255).min(row_alpha);
                        let r = (pixel >> 16) & 0xFF;
                        let g = (pixel >> 8) & 0xFF;
                        let b = pixel & 0xFF;
                        let reflected = (final_alpha << 24) | (r << 16) | (g << 8) | b;
                        buffer[dst_idx] = alpha_blend(buffer[dst_idx], reflected);
                    }
                }
            }
        }
    }

    fn draw_icon_bicubic(&self, buffer: &mut [u32], buf_width: usize, pixels: &[u32], src_size: u32, x: u32, y: u32, dst_size: u32) {
        let scale = src_size as f32 / dst_size as f32;
        let src_w = src_size as usize;
        
        for iy in 0..dst_size {
            for ix in 0..dst_size {
                let src_x = ix as f32 * scale;
                let src_y = iy as f32 * scale;
                
                let pixel = bicubic_sample(pixels, src_w, src_x, src_y);
                
                let dst_x = x as usize + ix as usize;
                let dst_y = y as usize + iy as usize;
                let dst_idx = dst_y * buf_width + dst_x;

                if dst_idx < buffer.len() {
                    let alpha = (pixel >> 24) & 0xFF;
                    if alpha > 0 {
                        buffer[dst_idx] = alpha_blend(buffer[dst_idx], pixel);
                    }
                }
            }
        }
    }
    
    fn draw_icon_bilinear(&self, buffer: &mut [u32], buf_width: usize, pixels: &[u32], src_size: u32, x: u32, y: u32, dst_size: u32) {
        let scale = src_size as f32 / dst_size as f32;
        let src_w = src_size as usize;
        
        for iy in 0..dst_size {
            for ix in 0..dst_size {
                let src_xf = ix as f32 * scale;
                let src_yf = iy as f32 * scale;
                
                let x0 = src_xf as usize;
                let y0 = src_yf as usize;
                let x1 = (x0 + 1).min(src_w - 1);
                let y1 = (y0 + 1).min(src_w - 1);
                
                let fx = src_xf - x0 as f32;
                let fy = src_yf - y0 as f32;
                
                // Sample 4 pixels
                let p00 = pixels.get(y0 * src_w + x0).copied().unwrap_or(0);
                let p10 = pixels.get(y0 * src_w + x1).copied().unwrap_or(0);
                let p01 = pixels.get(y1 * src_w + x0).copied().unwrap_or(0);
                let p11 = pixels.get(y1 * src_w + x1).copied().unwrap_or(0);
                
                // Bilinear interpolation for each channel
                let pixel = bilinear_blend(p00, p10, p01, p11, fx, fy);
                
                let dst_x = x as usize + ix as usize;
                let dst_y = y as usize + iy as usize;
                let dst_idx = dst_y * buf_width + dst_x;

                if dst_idx < buffer.len() {
                    let alpha = (pixel >> 24) & 0xFF;
                    if alpha > 0 {
                        buffer[dst_idx] = alpha_blend(buffer[dst_idx], pixel);
                    }
                }
            }
        }
    }

    fn draw_indicator_glow(&self, buffer: &mut [u32], buf_width: usize, center_x: u32, center_y: u32) {
        let (r, g, b) = self.indicator_color;
        
        // Outer glow
        let glow_radius = 8i32;
        for dy in -glow_radius..=glow_radius {
            for dx in -glow_radius..=glow_radius {
                let dist_sq = dx * dx + dy * dy;
                if dist_sq <= glow_radius * glow_radius {
                    let x = center_x as i32 + dx;
                    let y = center_y as i32 + dy;
                    if x >= 0 && y >= 0 {
                        let idx = y as usize * buf_width + x as usize;
                        if idx < buffer.len() {
                            let dist = (dist_sq as f32).sqrt();
                            let falloff = 1.0 - (dist / glow_radius as f32);
                            let alpha = (falloff * falloff * 80.0) as u32;
                            if alpha > 0 {
                                let glow = (alpha << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
                                buffer[idx] = alpha_blend(buffer[idx], glow);
                            }
                        }
                    }
                }
            }
        }

        // Solid center
        let color = 0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
        let radius = 3i32;
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx * dx + dy * dy <= radius * radius {
                    let x = center_x as i32 + dx;
                    let y = center_y as i32 + dy;
                    if x >= 0 && y >= 0 {
                        let idx = y as usize * buf_width + x as usize;
                        if idx < buffer.len() {
                            buffer[idx] = color;
                        }
                    }
                }
            }
        }
    }

    fn draw_placeholder(&self, buffer: &mut [u32], buf_width: usize, x: u32, y: u32, size: u32) {
        // Draw a simple rounded square placeholder for missing icons
        let (ir, ig, ib) = self.indicator_color;
        let color = 0x80000000 | ((ir as u32 / 2) << 16) | ((ig as u32 / 2) << 8) | (ib as u32 / 2);
        let radius = (size / 6) as i32;
        
        for iy in 0..size {
            for ix in 0..size {
                let dst_x = x as usize + ix as usize;
                let dst_y = y as usize + iy as usize;
                let dst_idx = dst_y * buf_width + dst_x;
                
                if dst_idx >= buffer.len() { continue; }
                
                // Rounded corner check
                let ixi = ix as i32;
                let iyi = iy as i32;
                let sz = size as i32;
                
                let in_rect = if ixi < radius && iyi < radius {
                    let dx = radius - ixi;
                    let dy = radius - iyi;
                    dx * dx + dy * dy <= radius * radius
                } else if ixi >= sz - radius && iyi < radius {
                    let dx = ixi - (sz - radius - 1);
                    let dy = radius - iyi;
                    dx * dx + dy * dy <= radius * radius
                } else if ixi < radius && iyi >= sz - radius {
                    let dx = radius - ixi;
                    let dy = iyi - (sz - radius - 1);
                    dx * dx + dy * dy <= radius * radius
                } else if ixi >= sz - radius && iyi >= sz - radius {
                    let dx = ixi - (sz - radius - 1);
                    let dy = iyi - (sz - radius - 1);
                    dx * dx + dy * dy <= radius * radius
                } else {
                    true
                };
                
                if in_rect {
                    buffer[dst_idx] = alpha_blend(buffer[dst_idx], color);
                }
            }
        }
    }

    fn draw_separator(&self, buffer: &mut [u32], buf_width: usize, x: u32, y: u32, icon_size: u32) {
        // Draw a subtle vertical separator line
        let (ir, ig, ib) = self.indicator_color;
        let sep_width = 2u32;
        let sep_height = (icon_size as f32 * 0.6) as u32;
        let y_offset = (icon_size - sep_height) / 2;
        
        // Center the separator in its allocated space (icon_size / 3)
        let sep_x = x + (icon_size / 6) - (sep_width / 2);
        
        for dy in 0..sep_height {
            // Fade at top and bottom
            let fade = {
                let progress = dy as f32 / sep_height as f32;
                let edge_fade = 0.15;
                if progress < edge_fade {
                    progress / edge_fade
                } else if progress > (1.0 - edge_fade) {
                    (1.0 - progress) / edge_fade
                } else {
                    1.0
                }
            };
            
            let alpha = (128.0 * fade) as u32;
            let color = (alpha << 24) | ((ir as u32) << 16) | ((ig as u32) << 8) | (ib as u32);
            
            for dx in 0..sep_width {
                let px = sep_x + dx;
                let py = y + y_offset + dy;
                let idx = py as usize * buf_width + px as usize;
                if idx < buffer.len() {
                    buffer[idx] = alpha_blend(buffer[idx], color);
                }
            }
        }
    }

    fn draw_drop_indicator(&self, buffer: &mut [u32], buf_width: usize, x: u32, y: u32, icon_size: u32) {
        // Draw a bright vertical line indicating where the dragged item will be dropped
        let (ir, ig, ib) = self.indicator_color;
        let line_width = 3u32;
        let line_height = icon_size;
        
        for dy in 0..line_height {
            // Slight fade at edges
            let fade = {
                let progress = dy as f32 / line_height as f32;
                let edge_fade = 0.1;
                if progress < edge_fade {
                    progress / edge_fade
                } else if progress > (1.0 - edge_fade) {
                    (1.0 - progress) / edge_fade
                } else {
                    1.0
                }
            };
            
            let alpha = (220.0 * fade) as u32;
            let color = (alpha << 24) | ((ir as u32) << 16) | ((ig as u32) << 8) | (ib as u32);
            
            for dx in 0..line_width {
                let px = x + dx;
                let py = y + dy;
                let idx = py as usize * buf_width + px as usize;
                if idx < buffer.len() {
                    buffer[idx] = alpha_blend(buffer[idx], color);
                }
            }
        }
    }

    pub fn hit_test(&self, x: i32, y: i32, items: &[DockItem]) -> Option<usize> {
        // Generous vertical hit area
        let extra = (self.icon_size as f32 * 0.3) as i32;
        let top = self.padding.top as i32 - extra;
        let bottom = (self.padding.top + self.icon_size) as i32 + extra;
        
        if y < top || y >= bottom {
            return None;
        }

        // Calculate total width the same way render does (at scale 1.0)
        let mut total_width: f32 = 0.0;
        for (i, item) in items.iter().enumerate() {
            if item.is_separator() {
                total_width += (self.icon_size / 3) as f32;
            } else {
                total_width += self.icon_size as f32;
            }
            if i < items.len() - 1 {
                total_width += self.spacing.x as f32;
            }
        }
        
        // Center the icons (matching render logic)
        let start_x = (self.width as f32 - total_width) / 2.0;
        
        // Walk through items and check hit areas
        let mut x_pos = start_x;
        for (i, item) in items.iter().enumerate() {
            let item_width = if item.is_separator() {
                (self.icon_size / 3) as f32
            } else {
                self.icon_size as f32
            };
            
            // Hit area extends from half the spacing before to half the spacing after
            let half_spacing = self.spacing.x as f32 / 2.0;
            let hit_left = x_pos - half_spacing;
            let hit_right = x_pos + item_width + half_spacing;
            
            if (x as f32) >= hit_left && (x as f32) < hit_right {
                return Some(i);
            }
            
            x_pos += item_width + self.spacing.x as f32;
        }

        None
    }
}

fn bilinear_blend(p00: u32, p10: u32, p01: u32, p11: u32, fx: f32, fy: f32) -> u32 {
    let blend_channel = |shift: u32| -> u32 {
        let c00 = ((p00 >> shift) & 0xFF) as f32;
        let c10 = ((p10 >> shift) & 0xFF) as f32;
        let c01 = ((p01 >> shift) & 0xFF) as f32;
        let c11 = ((p11 >> shift) & 0xFF) as f32;
        
        let top = c00 + (c10 - c00) * fx;
        let bot = c01 + (c11 - c01) * fx;
        (top + (bot - top) * fy) as u32
    };
    
    let a = blend_channel(24);
    let r = blend_channel(16);
    let g = blend_channel(8);
    let b = blend_channel(0);
    
    (a << 24) | (r << 16) | (g << 8) | b
}

fn alpha_blend(dst: u32, src: u32) -> u32 {
    let sa = ((src >> 24) & 0xFF) as u32;
    if sa == 0 {
        return dst;
    }
    if sa == 255 {
        return src;
    }

    let da = ((dst >> 24) & 0xFF) as u32;
    let sr = ((src >> 16) & 0xFF) as u32;
    let sg = ((src >> 8) & 0xFF) as u32;
    let sb = (src & 0xFF) as u32;
    let dr = ((dst >> 16) & 0xFF) as u32;
    let dg = ((dst >> 8) & 0xFF) as u32;
    let db = (dst & 0xFF) as u32;

    let out_a = sa + da * (255 - sa) / 255;
    if out_a == 0 {
        return 0;
    }

    let out_r = (sr * sa + dr * da * (255 - sa) / 255) / out_a;
    let out_g = (sg * sa + dg * da * (255 - sa) / 255) / out_a;
    let out_b = (sb * sa + db * da * (255 - sa) / 255) / out_a;

    (out_a << 24) | (out_r << 16) | (out_g << 8) | out_b
}

fn brighten_pixel(pixel: u32) -> u32 {
    let a = (pixel >> 24) & 0xFF;
    let r = (((pixel >> 16) & 0xFF) as u32 * 120 / 100).min(255);
    let g = (((pixel >> 8) & 0xFF) as u32 * 120 / 100).min(255);
    let b = ((pixel & 0xFF) as u32 * 120 / 100).min(255);
    (a << 24) | (r << 16) | (g << 8) | b
}

// Cubic hermite spline interpolation for smooth scaling
fn cubic_hermite(a: f32, b: f32, c: f32, d: f32, t: f32) -> f32 {
    let a0 = -a / 2.0 + (3.0 * b) / 2.0 - (3.0 * c) / 2.0 + d / 2.0;
    let a1 = a - (5.0 * b) / 2.0 + 2.0 * c - d / 2.0;
    let a2 = -a / 2.0 + c / 2.0;
    let a3 = b;
    a0 * t * t * t + a1 * t * t + a2 * t + a3
}

// Apply unsharp mask sharpening to improve edge clarity
fn sharpen_image(img: image::RgbaImage, strength: f32) -> image::RgbaImage {
    use image::GenericImageView;
    let (width, height) = img.dimensions();
    let mut sharpened = img.clone();
    
    // Simple 3x3 unsharp mask kernel
    let kernel = [
        [0.0, -strength, 0.0],
        [-strength, 1.0 + 4.0 * strength, -strength],
        [0.0, -strength, 0.0],
    ];
    
    for y in 1..(height - 1) {
        for x in 1..(width - 1) {
            let mut sum_r = 0.0;
            let mut sum_g = 0.0;
            let mut sum_b = 0.0;
            let center_a = img.get_pixel(x, y)[3];
            
            // Skip fully transparent pixels
            if center_a == 0 {
                continue;
            }
            
            for ky in 0..3 {
                for kx in 0..3 {
                    let px = (x as i32 + kx - 1) as u32;
                    let py = (y as i32 + ky - 1) as u32;
                    let pixel = img.get_pixel(px, py);
                    let k = kernel[ky as usize][kx as usize];
                    sum_r += pixel[0] as f32 * k;
                    sum_g += pixel[1] as f32 * k;
                    sum_b += pixel[2] as f32 * k;
                }
            }
            
            let new_pixel = image::Rgba([
                sum_r.max(0.0).min(255.0) as u8,
                sum_g.max(0.0).min(255.0) as u8,
                sum_b.max(0.0).min(255.0) as u8,
                center_a,
            ]);
            sharpened.put_pixel(x, y, new_pixel);
        }
    }
    
    sharpened
}

fn bicubic_sample(pixels: &[u32], src_w: usize, x: f32, y: f32) -> u32 {
    let x0 = x.floor() as isize;
    let y0 = y.floor() as isize;
    let fx = x - x0 as f32;
    let fy = y - y0 as f32;
    
    let mut channels = [0f32; 4]; // ARGB
    
    for ch in 0..4 {
        let shift = (3 - ch) * 8;
        let mut cols = [0f32; 4];
        
        for j in 0..4 {
            let py = (y0 - 1 + j as isize).max(0).min(src_w as isize - 1) as usize;
            let mut row = [0f32; 4];
            
            for i in 0..4 {
                let px = (x0 - 1 + i as isize).max(0).min(src_w as isize - 1) as usize;
                let idx = py * src_w + px;
                let pixel = pixels.get(idx).copied().unwrap_or(0);
                row[i] = ((pixel >> shift) & 0xFF) as f32;
            }
            
            cols[j] = cubic_hermite(row[0], row[1], row[2], row[3], fx);
        }
        
        channels[ch] = cubic_hermite(cols[0], cols[1], cols[2], cols[3], fy).max(0.0).min(255.0);
    }
    
    ((channels[0] as u32) << 24) | ((channels[1] as u32) << 16) | ((channels[2] as u32) << 8) | (channels[3] as u32)
}
