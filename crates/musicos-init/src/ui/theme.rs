use crate::renderer::Color;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeName {
    Blue,
    Red,
    Green,
    Purple,
}

impl ThemeName {
    pub fn from_str(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "red" => Self::Red,
            "green" => Self::Green,
            "purple" => Self::Purple,
            _ => Self::Blue,
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            Self::Blue => "blue",
            Self::Red => "red",
            Self::Green => "green",
            Self::Purple => "purple",
        }
    }

    pub fn primary_color(&self) -> Color {
        match self {
            Self::Blue => Color::rgb(35, 108, 220),
            Self::Red => Color::rgb(207, 62, 82),
            Self::Green => Color::rgb(30, 149, 92),
            Self::Purple => Color::rgb(137, 91, 196),
        }
    }

    pub fn gradient_colors(&self) -> (Color, Color) {
        let top = match self {
            Self::Blue => Color::rgb(63, 143, 229),
            Self::Red => Color::rgb(225, 94, 103),
            Self::Green => Color::rgb(64, 177, 126),
            Self::Purple => Color::rgb(157, 117, 211),
        };
        let bottom = match self {
            Self::Blue => Color::rgb(233, 243, 255),
            Self::Red => Color::rgb(255, 235, 238),
            Self::Green => Color::rgb(231, 249, 238),
            Self::Purple => Color::rgb(243, 236, 255),
        };
        (top, bottom)
    }
}
