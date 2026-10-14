use gpui::{rgb, Hsla, WindowAppearance};

#[derive(Debug, Clone, Copy)]
pub struct ThemePalette {
    pub window: Hsla,
    pub sidebar: Hsla,
    pub surface: Hsla,
    pub elevated_surface: Hsla,
    pub chrome: Hsla,
    pub row_alternate: Hsla,
    pub hover: Hsla,
    pub selected: Hsla,
    pub text: Hsla,
    pub secondary_text: Hsla,
    pub separator: Hsla,
    pub focus: Hsla,
    pub success: Hsla,
    pub warning: Hsla,
    pub error: Hsla,
}

impl ThemePalette {
    pub fn for_appearance(appearance: WindowAppearance) -> Self {
        if matches!(
            appearance,
            WindowAppearance::Dark | WindowAppearance::VibrantDark
        ) {
            Self::dark()
        } else {
            Self::baseui()
        }
    }

    pub fn baseui() -> Self {
        Self {
            window: rgb(0x80d3ff).into(),
            sidebar: rgb(0xcbf1ff).into(),
            surface: rgb(0x00aeff).into(),
            elevated_surface: rgb(0xd6defd).into(),
            chrome: rgb(0x00baff).into(),
            row_alternate: rgb(0xaae4ff).into(),
            hover: rgb(0xd6defd).into(),
            selected: rgb(0x0064df).into(),
            text: rgb(0x000000).into(),
            secondary_text: rgb(0x000000).into(),
            separator: rgb(0x0075ff).into(),
            focus: rgb(0x0041ff).into(),
            success: rgb(0x1fc800).into(),
            warning: rgb(0xffe100).into(),
            error: rgb(0xff0000).into(),
        }
    }

    pub fn light() -> Self {
        Self::baseui()
    }

    pub fn dark() -> Self {
        Self {
            window: rgb(0x090d16).into(),
            sidebar: rgb(0x0f172a).into(),
            surface: rgb(0x0f172a).into(),
            elevated_surface: rgb(0x1e293b).into(),
            chrome: rgb(0x0f172a).into(),
            row_alternate: rgb(0x131b2e).into(),
            hover: rgb(0x1e293b).into(),
            selected: rgb(0x3b82f6).into(),
            text: rgb(0xf8fafc).into(),
            secondary_text: rgb(0x94a3b8).into(),
            separator: rgb(0x1e293b).into(),
            focus: rgb(0x3b82f6).into(),
            success: rgb(0x22c55e).into(),
            warning: rgb(0xf59e0b).into(),
            error: rgb(0xef4444).into(),
        }
    }

    pub fn with_accent(mut self, accent: &str, dark: bool) -> Self {
        let color = match (accent, dark) {
            ("graphite", false) => rgb(0x475569),
            ("graphite", true) => rgb(0x94a3b8),
            ("jade", false) => rgb(0x0d9488),
            ("jade", true) => rgb(0x2dd4bf),
            ("amber", false) => rgb(0xd97706),
            ("amber", true) => rgb(0xfbbf24),
            ("berry", false) => rgb(0xdb2777),
            ("berry", true) => rgb(0xf472b6),
            (_, false) => rgb(0x2563eb),
            (_, true) => rgb(0x3b82f6),
        };
        self.selected = color.into();
        self.focus = color.into();
        self
    }
}
