use anyhow::{Context, Result};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::path::{Path, PathBuf};

/// CSS-style spacing: can be single value, [x, y], or [top, right, bottom, left]
#[derive(Debug, Clone)]
pub struct Spacing {
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
    pub left: u32,
}

impl Serialize for Spacing {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where S: Serializer
    {
        // Serialize as compact form when possible
        if self.top == self.bottom && self.left == self.right {
            if self.top == self.left {
                // All same: single value
                serializer.serialize_u32(self.top)
            } else {
                // [horizontal, vertical] - matches CSS shorthand order
                [self.left, self.top].serialize(serializer)
            }
        } else {
            // [top, right, bottom, left]
            [self.top, self.right, self.bottom, self.left].serialize(serializer)
        }
    }
}

impl Spacing {
    pub fn uniform(v: u32) -> Self {
        Self { top: v, right: v, bottom: v, left: v }
    }
    
    pub fn xy(x: u32, y: u32) -> Self {
        Self { top: y, right: x, bottom: y, left: x }
    }
    
    pub fn x(&self) -> u32 { self.left + self.right }
    pub fn y(&self) -> u32 { self.top + self.bottom }
}

impl<'de> Deserialize<'de> for Spacing {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where D: Deserializer<'de>
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum SpacingValue {
            Single(u32),
            Two([u32; 2]),
            Four([u32; 4]),
        }
        
        let val = SpacingValue::deserialize(deserializer)?;
        Ok(match val {
            SpacingValue::Single(v) => Spacing::uniform(v),
            SpacingValue::Two([x, y]) => Spacing::xy(x, y),
            SpacingValue::Four([t, r, b, l]) => Spacing { top: t, right: r, bottom: b, left: l },
        })
    }
}

impl Default for Spacing {
    fn default() -> Self { Self::uniform(12) }
}

/// Item spacing: single value or [x, y] for horizontal/vertical
#[derive(Debug, Clone)]
pub struct ItemSpacing {
    pub x: u32,
    pub y: u32,
}

impl Serialize for ItemSpacing {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where S: Serializer
    {
        if self.x == self.y {
            serializer.serialize_u32(self.x)
        } else {
            [self.x, self.y].serialize(serializer)
        }
    }
}

impl ItemSpacing {
    pub fn uniform(v: u32) -> Self {
        Self { x: v, y: v }
    }
}

impl<'de> Deserialize<'de> for ItemSpacing {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where D: Deserializer<'de>
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum SpacingValue {
            Single(u32),
            Two([u32; 2]),
        }
        
        let val = SpacingValue::deserialize(deserializer)?;
        Ok(match val {
            SpacingValue::Single(v) => ItemSpacing::uniform(v),
            SpacingValue::Two([x, y]) => ItemSpacing { x, y },
        })
    }
}

impl Default for ItemSpacing {
    fn default() -> Self { Self::uniform(8) }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub dock: DockSettings,
    #[serde(default)]
    pub items: Vec<DockItem>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DockSettings {
    #[serde(default = "default_icon_size")]
    pub icon_size: u32,
    #[serde(default)]
    pub spacing: ItemSpacing,
    #[serde(default)]
    pub padding: Spacing,
    #[serde(default)]
    pub vertical_offset: i32,
    #[serde(default = "default_background_color")]
    pub background_color: String,
    #[serde(default = "default_background_opacity")]
    pub background_opacity: f32,
    #[serde(default = "default_indicator_color")]
    pub indicator_color: String,
    #[serde(default = "default_auto_hide")]
    pub auto_hide: bool,
    #[serde(default = "default_auto_hide_delay")]
    pub auto_hide_delay_ms: u64,
    #[serde(default = "default_corner_radius")]
    pub corner_radius: u32,
    #[serde(default = "default_magnification")]
    pub magnification: f32,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub hide_taskbar: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DockItem {
    pub name: String,
    #[serde(default, skip_serializing_if = "is_default_path")]
    pub path: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub separator: bool,
    /// Special system item type: "start_menu", "recycle_bin", "settings", "show_desktop", 
    /// "task_view", "action_center", "file_explorer", "control_panel", "run_dialog"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub special: Option<String>,
}

fn is_default_path(p: &PathBuf) -> bool {
    p.as_os_str().is_empty()
}

impl DockItem {
    pub fn new_separator() -> Self {
        Self {
            name: "---".to_string(),
            path: PathBuf::new(),
            icon: None,
            args: Vec::new(),
            separator: true,
            special: None,
        }
    }
    
    pub fn is_separator(&self) -> bool {
        self.separator || self.name == "---"
    }
    
    pub fn is_special(&self) -> bool {
        self.special.is_some()
    }
}

fn default_icon_size() -> u32 { 48 }
fn default_background_color() -> String { "#1e1e2e".to_string() }
fn default_background_opacity() -> f32 { 0.9 }
fn default_indicator_color() -> String { "#cba6f7".to_string() }
fn default_auto_hide() -> bool { true }
fn default_auto_hide_delay() -> u64 { 400 }
fn default_corner_radius() -> u32 { 12 }
fn default_magnification() -> f32 { 1.5 }

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        let config: Config = toml::from_str(&content)
            .with_context(|| "Failed to parse config file")?;
        Ok(config)
    }
    
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .with_context(|| "Failed to serialize config")?;
        std::fs::write(path, content)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;
        Ok(())
    }

    pub fn default_path() -> PathBuf {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));
        exe_dir.join("config.toml")
    }
}

/// Parse hex color string to ARGB u32
pub fn parse_hex_color(hex: &str, opacity: f32) -> u32 {
    let hex = hex.trim_start_matches('#');
    let rgb = u32::from_str_radix(hex, 16).unwrap_or(0x1e1e2e);
    let alpha = (opacity * 255.0) as u32;
    (alpha << 24) | rgb
}

/// Parse hex color to RGB tuple
pub fn parse_hex_rgb(hex: &str) -> (u8, u8, u8) {
    let hex = hex.trim_start_matches('#');
    let val = u32::from_str_radix(hex, 16).unwrap_or(0xcba6f7);
    (
        ((val >> 16) & 0xFF) as u8,
        ((val >> 8) & 0xFF) as u8,
        (val & 0xFF) as u8,
    )
}
