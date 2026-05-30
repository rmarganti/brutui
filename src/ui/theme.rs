use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    pub base: Style,
    pub panel_border: Style,
    pub focused_panel_border: Style,
    pub selected_item: Style,
    pub focused_selected_item: Style,
    pub emphasized_text: Style,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ThemeConfig {
    pub base: ThemeStyleConfig,
    pub panel_border: ThemeStyleConfig,
    pub focused_panel_border: ThemeStyleConfig,
    pub selected_item: ThemeStyleConfig,
    pub focused_selected_item: ThemeStyleConfig,
    pub emphasized_text: ThemeStyleConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PartialThemeConfig {
    pub base: Option<PartialThemeStyleConfig>,
    pub panel_border: Option<PartialThemeStyleConfig>,
    pub focused_panel_border: Option<PartialThemeStyleConfig>,
    pub selected_item: Option<PartialThemeStyleConfig>,
    pub focused_selected_item: Option<PartialThemeStyleConfig>,
    pub emphasized_text: Option<PartialThemeStyleConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct ThemeStyleConfig {
    pub fg: Option<ThemeColor>,
    pub bg: Option<ThemeColor>,
    pub bold: bool,
    pub italic: bool,
    pub underlined: bool,
    pub reversed: bool,
    pub dim: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PartialThemeStyleConfig {
    pub fg: Option<ThemeColor>,
    pub bg: Option<ThemeColor>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underlined: Option<bool>,
    pub reversed: Option<bool>,
    pub dim: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ThemeColor(String);

impl Theme {
    pub fn from_config(config: &ThemeConfig) -> Self {
        Self {
            base: config.base.style(),
            panel_border: config.panel_border.style(),
            focused_panel_border: config.focused_panel_border.style(),
            selected_item: config.selected_item.style(),
            focused_selected_item: config.focused_selected_item.style(),
            emphasized_text: config.emphasized_text.style(),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::from_config(&ThemeConfig::default())
    }
}

impl ThemeConfig {
    pub fn merge(mut self, partial: PartialThemeConfig) -> Self {
        self.base.merge(partial.base);
        self.panel_border.merge(partial.panel_border);
        self.focused_panel_border
            .merge(partial.focused_panel_border);
        self.selected_item.merge(partial.selected_item);
        self.focused_selected_item
            .merge(partial.focused_selected_item);
        self.emphasized_text.merge(partial.emphasized_text);
        self
    }
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            base: ThemeStyleConfig::default(),
            panel_border: ThemeStyleConfig {
                fg: Some(ThemeColor::new_unchecked("black")),
                ..ThemeStyleConfig::default()
            },
            focused_panel_border: ThemeStyleConfig {
                fg: Some(ThemeColor::new_unchecked("yellow")),
                ..ThemeStyleConfig::default()
            },
            selected_item: ThemeStyleConfig {
                reversed: true,
                ..ThemeStyleConfig::default()
            },
            focused_selected_item: ThemeStyleConfig {
                bold: true,
                reversed: true,
                ..ThemeStyleConfig::default()
            },
            emphasized_text: ThemeStyleConfig {
                bold: true,
                ..ThemeStyleConfig::default()
            },
        }
    }
}

impl ThemeStyleConfig {
    fn merge(&mut self, partial: Option<PartialThemeStyleConfig>) {
        let Some(partial) = partial else {
            return;
        };
        if let Some(fg) = partial.fg {
            self.fg = Some(fg);
        }
        if let Some(bg) = partial.bg {
            self.bg = Some(bg);
        }
        if let Some(bold) = partial.bold {
            self.bold = bold;
        }
        if let Some(italic) = partial.italic {
            self.italic = italic;
        }
        if let Some(underlined) = partial.underlined {
            self.underlined = underlined;
        }
        if let Some(reversed) = partial.reversed {
            self.reversed = reversed;
        }
        if let Some(dim) = partial.dim {
            self.dim = dim;
        }
    }

    fn style(&self) -> Style {
        let mut style = Style::default();
        if let Some(fg) = &self.fg {
            style = style.fg(fg.color());
        }
        if let Some(bg) = &self.bg {
            style = style.bg(bg.color());
        }

        let mut modifiers = Modifier::empty();
        if self.bold {
            modifiers |= Modifier::BOLD;
        }
        if self.italic {
            modifiers |= Modifier::ITALIC;
        }
        if self.underlined {
            modifiers |= Modifier::UNDERLINED;
        }
        if self.reversed {
            modifiers |= Modifier::REVERSED;
        }
        if self.dim {
            modifiers |= Modifier::DIM;
        }
        style.add_modifier(modifiers)
    }
}

impl ThemeColor {
    fn new_unchecked(value: &'static str) -> Self {
        Self(value.to_string())
    }

    pub fn color(&self) -> Color {
        parse_color(&self.0).expect("ThemeColor is validated during deserialization/construction")
    }
}

impl<'de> Deserialize<'de> for ThemeColor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        parse_color(&value).map_err(serde::de::Error::custom)?;
        Ok(Self(value))
    }
}

fn parse_color(value: &str) -> Result<Color, String> {
    let normalized = value.trim().to_ascii_lowercase().replace(['-', ' '], "_");
    let color = match normalized.as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" | "grey" => Color::Gray,
        "dark_gray" | "dark_grey" => Color::DarkGray,
        "light_red" => Color::LightRed,
        "light_green" => Color::LightGreen,
        "light_yellow" => Color::LightYellow,
        "light_blue" => Color::LightBlue,
        "light_magenta" => Color::LightMagenta,
        "light_cyan" => Color::LightCyan,
        "white" => Color::White,
        "reset" => Color::Reset,
        _ => return parse_hex_color(value),
    };
    Ok(color)
}

fn parse_hex_color(value: &str) -> Result<Color, String> {
    let Some(hex) = value.strip_prefix('#') else {
        return Err(format!(
            "unknown color `{value}`; use a named color or #RRGGBB"
        ));
    };
    if hex.len() != 6 || !hex.chars().all(|character| character.is_ascii_hexdigit()) {
        return Err(format!("invalid hex color `{value}`; expected #RRGGBB"));
    }

    let red = u8::from_str_radix(&hex[0..2], 16).expect("validated hex red channel");
    let green = u8::from_str_radix(&hex[2..4], 16).expect("validated hex green channel");
    let blue = u8::from_str_radix(&hex[4..6], 16).expect("validated hex blue channel");
    Ok(Color::Rgb(red, green, blue))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_theme_config_overrides_individual_fields() {
        let partial = PartialThemeConfig {
            focused_selected_item: Some(PartialThemeStyleConfig {
                fg: Some(ThemeColor("#f0f0f0".to_string())),
                bold: Some(false),
                ..PartialThemeStyleConfig::default()
            }),
            ..PartialThemeConfig::default()
        };

        let config = ThemeConfig::default().merge(partial);

        assert_eq!(
            config.focused_selected_item.fg.unwrap().color(),
            Color::Rgb(240, 240, 240)
        );
        assert!(!config.focused_selected_item.bold);
        assert!(config.focused_selected_item.reversed);
    }
}
