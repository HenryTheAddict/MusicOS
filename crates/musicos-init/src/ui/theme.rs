use crate::renderer::Color;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeName {
    Seaglass,
    Dark,
    Blue,
    Red,
    Green,
    Purple,
}

impl ThemeName {
    pub fn from_str(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "seaglass" => Self::Seaglass,
            "dark" => Self::Dark,
            "red" => Self::Red,
            "green" => Self::Green,
            "purple" => Self::Purple,
            _ => Self::Blue,
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            Self::Seaglass => "seaglass",
            Self::Dark => "dark",
            Self::Blue => "blue",
            Self::Red => "red",
            Self::Green => "green",
            Self::Purple => "purple",
        }
    }

    pub fn primary_color(&self) -> Color {
        match self {
            Self::Seaglass => Color::rgb(171, 51, 23),
            Self::Dark => Color::rgb(255, 153, 126),
            Self::Blue => Color::rgb(35, 108, 220),
            Self::Red => Color::rgb(207, 62, 82),
            Self::Green => Color::rgb(30, 149, 92),
            Self::Purple => Color::rgb(137, 91, 196),
        }
    }

    pub fn gradient_colors(&self) -> (Color, Color) {
        let accent = match self {
            Self::Seaglass | Self::Dark => self.primary_color(),
            Self::Blue => Color::rgb(37, 112, 190),
            Self::Red => Color::rgb(178, 68, 76),
            Self::Green => Color::rgb(48, 137, 91),
            Self::Purple => Color::rgb(116, 82, 155),
        };
        (accent, accent.mix(Color::WHITE, 0.18))
    }

    pub const OPTIONS: [Self; 6] = [
        Self::Blue,
        Self::Seaglass,
        Self::Red,
        Self::Green,
        Self::Purple,
        Self::Dark,
    ];

    pub fn cycle(self, offset: isize) -> Self {
        let index = Self::OPTIONS
            .iter()
            .position(|theme| *theme == self)
            .unwrap_or(0);
        Self::OPTIONS[(index as isize + offset).rem_euclid(Self::OPTIONS.len() as isize) as usize]
    }
}
