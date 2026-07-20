use ratatui::style::Color;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThemeName {
    System,
    #[default]
    Emerald,
    Ocean,
    Rose,
    Monochrome,
}

impl ThemeName {
    pub const ALL: [Self; 5] = [
        Self::System,
        Self::Emerald,
        Self::Ocean,
        Self::Rose,
        Self::Monochrome,
    ];

    pub fn next(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|theme| *theme == self)
            .unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Emerald => "emerald",
            Self::Ocean => "ocean",
            Self::Rose => "rose",
            Self::Monochrome => "monochrome",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "system" => Some(Self::System),
            "emerald" => Some(Self::Emerald),
            "ocean" => Some(Self::Ocean),
            "rose" => Some(Self::Rose),
            "monochrome" => Some(Self::Monochrome),
            _ => None,
        }
    }
}

pub struct Theme {
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,
    pub background: Color,
    pub foreground: Color,
    pub muted: Color,
    pub error: Color,
    pub success: Color,
}

impl ThemeName {
    pub fn palette(self) -> Theme {
        match self {
            Self::System => Theme {
                primary: Color::Cyan,
                secondary: Color::Yellow,
                accent: Color::Reset,
                background: Color::Reset,
                foreground: Color::Reset,
                muted: Color::Gray,
                error: Color::Red,
                success: Color::Green,
            },
            Self::Emerald => Theme {
                primary: Color::Rgb(52, 211, 153),
                secondary: Color::Rgb(251, 191, 36),
                accent: Color::Rgb(6, 24, 20),
                background: Color::Rgb(6, 24, 20),
                foreground: Color::Rgb(236, 253, 245),
                muted: Color::Rgb(148, 163, 184),
                error: Color::Rgb(251, 113, 133),
                success: Color::Rgb(52, 211, 153),
            },
            Self::Ocean => Theme {
                primary: Color::Rgb(56, 189, 248),
                secondary: Color::Rgb(167, 139, 250),
                accent: Color::Rgb(8, 25, 48),
                background: Color::Rgb(8, 25, 48),
                foreground: Color::Rgb(224, 242, 254),
                muted: Color::Rgb(148, 163, 184),
                error: Color::Rgb(251, 113, 133),
                success: Color::Rgb(34, 211, 238),
            },
            Self::Rose => Theme {
                primary: Color::Rgb(251, 113, 133),
                secondary: Color::Rgb(244, 114, 182),
                accent: Color::Rgb(48, 14, 35),
                background: Color::Rgb(48, 14, 35),
                foreground: Color::Rgb(255, 241, 242),
                muted: Color::Rgb(190, 137, 162),
                error: Color::Rgb(253, 164, 175),
                success: Color::Rgb(251, 113, 133),
            },
            Self::Monochrome => Theme {
                primary: Color::Rgb(229, 231, 235),
                secondary: Color::Rgb(156, 163, 175),
                accent: Color::Rgb(24, 24, 27),
                background: Color::Rgb(24, 24, 27),
                foreground: Color::Rgb(244, 244, 245),
                muted: Color::Rgb(113, 113, 122),
                error: Color::Rgb(251, 113, 133),
                success: Color::Rgb(229, 231, 235),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ThemeName;

    #[test]
    fn cycles_through_each_theme() {
        assert_eq!(ThemeName::System.next(), ThemeName::Emerald);
        assert_eq!(ThemeName::Emerald.next(), ThemeName::Ocean);
        assert_eq!(ThemeName::Ocean.next(), ThemeName::Rose);
        assert_eq!(ThemeName::Rose.next(), ThemeName::Monochrome);
        assert_eq!(ThemeName::Monochrome.next(), ThemeName::System);
    }

    #[test]
    fn parses_persisted_names() {
        assert_eq!(ThemeName::parse("ocean"), Some(ThemeName::Ocean));
        assert_eq!(ThemeName::parse("system"), Some(ThemeName::System));
        assert_eq!(ThemeName::parse("rose"), Some(ThemeName::Rose));
        assert_eq!(ThemeName::parse("invalid"), None);
    }
}
