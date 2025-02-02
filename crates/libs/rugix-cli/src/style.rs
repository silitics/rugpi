//! Utilities for styling terminal text.
//!
//! We only care about modern terminals supporting ANSI escape sequences here.
//!
//! We implement this ourselves to avoid external dependencies and to gain full control of
//! the API.

use std::fmt;

/// ANSI terminal color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Color {
    /// Black.
    Black,
    /// Red.
    Red,
    /// Green.
    Green,
    /// Yellow.
    Yellow,
    /// Blue.
    Blue,
    /// Magenta.
    Magenta,
    /// Cyan.
    Cyan,
    /// Gray.
    Gray,
    /// Dark gray.
    DarkGray,
    /// Bright red.
    BrightRed,
    /// Bright green.
    BrightGreen,
    /// Bright yellow.
    BrightYellow,
    /// Bright blue.
    BrightBlue,
    /// Bright magenta.
    BrightMagenta,
    /// Bright cyan.
    BrightCyan,
    /// White.
    White,
}

impl Color {
    /// Foreground ANSI escape sequence.
    pub(crate) const fn foreground_ansi_sequence(self) -> &'static str {
        match self {
            Color::Black => "\x1b[30m",
            Color::Red => "\x1b[31m",
            Color::Green => "\x1b[32m",
            Color::Yellow => "\x1b[33m",
            Color::Blue => "\x1b[34m",
            Color::Magenta => "\x1b[35m",
            Color::Cyan => "\x1b[36m",
            Color::Gray => "\x1b[37m",
            Color::DarkGray => "\x1b[90m",
            Color::BrightRed => "\x1b[91m",
            Color::BrightGreen => "\x1b[92m",
            Color::BrightYellow => "\x1b[93m",
            Color::BrightBlue => "\x1b[94m",
            Color::BrightMagenta => "\x1b[95m",
            Color::BrightCyan => "\x1b[96m",
            Color::White => "\x1b[97m",
        }
    }

    /// Background ANSI escape sequence.
    pub(crate) const fn background_ansi_sequence(self) -> &'static str {
        match self {
            Color::Black => "\x1b[40m",
            Color::Red => "\x1b[41m",
            Color::Green => "\x1b[42m",
            Color::Yellow => "\x1b[43m",
            Color::Blue => "\x1b[44m",
            Color::Magenta => "\x1b[45m",
            Color::Cyan => "\x1b[46m",
            Color::Gray => "\x1b[47m",
            Color::DarkGray => "\x1b[100m",
            Color::BrightRed => "\x1b[101m",
            Color::BrightGreen => "\x1b[102m",
            Color::BrightYellow => "\x1b[103m",
            Color::BrightBlue => "\x1b[104m",
            Color::BrightMagenta => "\x1b[105m",
            Color::BrightCyan => "\x1b[106m",
            Color::White => "\x1b[107m",
        }
    }
}

/// ANSI style modifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Modifier {
    /// Bold font or increased intensity.
    Bold,
    /// Light font or decreased intensity.
    Faint,
    /// Italic.
    Italic,
    /// Underline.
    Underline,
    /// Slow blinking (below 150 times per minute).
    SlowBlink,
    /// Rapid blinking (above 150 times per minute).
    RapidBlink,
}

impl Modifier {
    /// Create a [`ModifierSet`] containing only the modifier.
    pub const fn into_set(self) -> ModifierSet {
        match self {
            Modifier::Bold => ModifierSet::BOLD,
            Modifier::Faint => ModifierSet::FAINT,
            Modifier::Italic => ModifierSet::ITALIC,
            Modifier::Underline => ModifierSet::UNDERLINE,
            Modifier::SlowBlink => ModifierSet::SLOW_BLINK,
            Modifier::RapidBlink => ModifierSet::RAPID_BLINK,
        }
    }
}

impl Modifier {
    /// ANSI escape sequence enabling the modifier.
    pub(crate) const fn enable_ansi_sequence(self) -> &'static str {
        match self {
            Modifier::Bold => "\x1b[1m",
            Modifier::Faint => "\x1b[2m",
            Modifier::Italic => "\x1b[3m",
            Modifier::Underline => "\x1b[4m",
            Modifier::SlowBlink => "\x1b[5m",
            Modifier::RapidBlink => "\x1b[6m",
        }
    }

    /// ANSI escape sequence disabling the modifier.
    pub(crate) const fn disable_ansi_sequence(self) -> &'static str {
        match self {
            Modifier::Bold => "\x1b[22m",
            Modifier::Faint => "\x1b[22m",
            Modifier::Italic => "\x1b[23m",
            Modifier::Underline => "\x1b[24m",
            Modifier::SlowBlink => "\x1b[25m",
            Modifier::RapidBlink => "\x1b[25m",
        }
    }

    /// Helper for iterating over all modifiers.
    const fn iter_start_modifier() -> Self {
        Self::Bold
    }

    /// Helper for iterating over all modifiers.
    const fn iter_next_modifier(self) -> Option<Self> {
        match self {
            Modifier::Bold => Some(Modifier::Faint),
            Modifier::Faint => Some(Modifier::Italic),
            Modifier::Italic => Some(Modifier::Underline),
            Modifier::Underline => Some(Modifier::SlowBlink),
            Modifier::SlowBlink => Some(Modifier::RapidBlink),
            Modifier::RapidBlink => None,
        }
    }
}

/// Set of ANSI style modifiers.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModifierSet {
    bits: u8,
}

impl ModifierSet {
    /// Bold font or increased intensity.
    pub const BOLD: Self = Self::from_bits(1 << 0);
    /// Light font or decreased intensity.
    pub const FAINT: Self = Self::from_bits(1 << 1);
    /// Italic.
    pub const ITALIC: Self = Self::from_bits(1 << 2);
    /// Underline.
    pub const UNDERLINE: Self = Self::from_bits(1 << 3);
    /// Slow blinking (below 150 times per minute).
    pub const SLOW_BLINK: Self = Self::from_bits(1 << 4);
    /// Rapid blinking (above 150 times per minute).
    pub const RAPID_BLINK: Self = Self::from_bits(1 << 5);

    /// Create a modifier from the provided bits.
    const fn from_bits(bits: u8) -> Self {
        Self { bits }
    }

    /// Create an empty modifier set.
    pub const fn new() -> Self {
        Self::from_bits(0)
    }

    /// Intersection of both sets.
    pub const fn intersection(self, other: Self) -> Self {
        Self {
            bits: self.bits & other.bits,
        }
    }

    /// Union of both sets.
    pub const fn union(self, other: Self) -> Self {
        Self {
            bits: self.bits | other.bits,
        }
    }

    /// Difference of both sets.
    pub const fn difference(self, other: Self) -> Self {
        Self {
            bits: self.bits & !other.bits,
        }
    }

    /// Check whether the set is empty.
    pub const fn is_empty(self) -> bool {
        self.bits == 0
    }

    /// Check whether the set contains a modifier.
    pub const fn contains(self, modifier: Modifier) -> bool {
        !self.intersection(modifier.into_set()).is_empty()
    }

    /// Iterate over the contained modifiers.
    pub const fn iter(self) -> modifier_set_iter::ModifierSetIter {
        modifier_set_iter::ModifierSetIter::new(self)
    }

    /// Add the given modifier to the set.
    pub const fn add(&mut self, modifier: Modifier) {
        *self = self.union(modifier.into_set());
    }

    /// Remove the given modifier from the set.
    ///
    /// Returns `true` if the modifier was contained in the set.
    pub const fn remove(&mut self, modifier: Modifier) -> bool {
        let contained = self.contains(modifier);
        self.bits &= !modifier.into_set().bits;
        contained
    }
}

impl std::ops::BitAnd for ModifierSet {
    type Output = ModifierSet;

    fn bitand(self, rhs: Self) -> Self::Output {
        self.intersection(rhs)
    }
}

impl std::ops::BitAndAssign for ModifierSet {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = self.intersection(rhs)
    }
}

impl std::ops::BitOr for ModifierSet {
    type Output = ModifierSet;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.union(rhs)
    }
}

impl std::ops::BitOrAssign for ModifierSet {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = self.union(rhs);
    }
}

impl std::ops::Sub for ModifierSet {
    type Output = ModifierSet;

    fn sub(self, rhs: Self) -> Self::Output {
        self.difference(rhs)
    }
}

impl std::ops::SubAssign for ModifierSet {
    fn sub_assign(&mut self, rhs: Self) {
        *self = self.difference(rhs)
    }
}

impl std::ops::BitAnd<Modifier> for ModifierSet {
    type Output = ModifierSet;

    fn bitand(self, rhs: Modifier) -> Self::Output {
        self.intersection(rhs.into_set())
    }
}

impl std::ops::BitAndAssign<Modifier> for ModifierSet {
    fn bitand_assign(&mut self, rhs: Modifier) {
        *self = self.intersection(rhs.into_set())
    }
}

impl std::ops::BitOr<Modifier> for ModifierSet {
    type Output = ModifierSet;

    fn bitor(self, rhs: Modifier) -> Self::Output {
        self.union(rhs.into_set())
    }
}

impl std::ops::BitOrAssign<Modifier> for ModifierSet {
    fn bitor_assign(&mut self, rhs: Modifier) {
        *self = self.union(rhs.into_set());
    }
}

impl std::ops::Sub<Modifier> for ModifierSet {
    type Output = ModifierSet;

    fn sub(mut self, rhs: Modifier) -> Self::Output {
        self.remove(rhs);
        self
    }
}

impl std::ops::SubAssign<Modifier> for ModifierSet {
    fn sub_assign(&mut self, rhs: Modifier) {
        self.remove(rhs);
    }
}

impl std::ops::Add<Modifier> for ModifierSet {
    type Output = ModifierSet;

    fn add(mut self, rhs: Modifier) -> Self::Output {
        ModifierSet::add(&mut self, rhs);
        self
    }
}

impl std::ops::AddAssign<Modifier> for ModifierSet {
    fn add_assign(&mut self, rhs: Modifier) {
        self.add(rhs)
    }
}

impl From<Modifier> for ModifierSet {
    fn from(value: Modifier) -> Self {
        value.into_set()
    }
}

impl IntoIterator for ModifierSet {
    type Item = Modifier;

    type IntoIter = modifier_set_iter::ModifierSetIter;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Private module concealing the iterator implementation for [`ModifierSet`].
mod modifier_set_iter {
    use std::iter::FusedIterator;

    use super::{Modifier, ModifierSet};

    /// Iterator over the modifiers contained in a [`ModifierSet`]
    pub struct ModifierSetIter {
        set: ModifierSet,
        check_next: Option<Modifier>,
    }

    impl ModifierSetIter {
        /// Create a new iterator over the given [`ModifierSet`].
        pub(super) const fn new(set: ModifierSet) -> Self {
            Self {
                set,
                check_next: Some(Modifier::iter_start_modifier()),
            }
        }
    }

    impl Iterator for ModifierSetIter {
        type Item = Modifier;

        fn next(&mut self) -> Option<Self::Item> {
            loop {
                let check_now = self.check_next?;
                self.check_next = check_now.iter_next_modifier();
                if self.set.contains(check_now) {
                    return Some(check_now);
                }
            }
        }
    }

    impl FusedIterator for ModifierSetIter {}
}

/// ANSI escape sequence resetting all styling.
pub(crate) const RESET_ALL: &str = "\x1b[0m";

/// Terminal text formatting style.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Style {
    /// Reset all styles before applying new styles.
    pub reset_all: bool,
    /// Set the foreground color to the given color.
    pub foreground_color: Option<Color>,
    /// Set the background color to the given color.
    pub background_color: Option<Color>,
    /// Enable the given modifiers.
    pub enable_modifiers: ModifierSet,
    /// Disable the given modifiers.
    pub disable_modifiers: ModifierSet,
}

/// Helper macro for implementing the different color methods on [`Style`].
macro_rules! style_color_methods {
    ($($fg_color:ident, $bg_color:ident, $color:ident;)*) => {
        $(
            #[doc = concat!("Set foreground color to [`Color::", stringify!($color), "`].")]
            pub const fn $fg_color(mut self) -> Self {
                self.foreground_color = Some(Color::$color);
                self
            }

            #[doc = concat!("Set background color to [`Color::", stringify!($color), "`].")]
            pub const fn $bg_color(mut self) -> Self {
                self.background_color = Some(Color::$color);
                self
            }
        )*
    };
}

impl Style {
    /// Create a new empty style.
    pub const fn new() -> Self {
        Self {
            reset_all: false,
            foreground_color: None,
            background_color: None,
            enable_modifiers: ModifierSet::new(),
            disable_modifiers: ModifierSet::new(),
        }
    }

    /// Reset all styles before applying new styles.
    pub const fn reset_all(mut self) -> Self {
        self.reset_all = true;
        self
    }

    /// Set the foreground color of the style.
    pub const fn foreground_color(mut self, color: Color) -> Self {
        self.foreground_color = Some(color);
        self
    }

    /// Set the background color of the style.
    pub const fn background_color(mut self, color: Color) -> Self {
        self.background_color = Some(color);
        self
    }

    /// Enable a modifier.
    pub const fn enable_modifier(mut self, modifier: Modifier) -> Self {
        self.enable_modifiers.add(modifier);
        self.disable_modifiers.remove(modifier);
        self
    }

    /// Disable a modifier.
    pub const fn disable_modifier(mut self, modifier: Modifier) -> Self {
        self.enable_modifiers.remove(modifier);
        self.disable_modifiers.add(modifier);
        self
    }

    /// Enable the [`Modifier::Bold`] modifier.
    pub const fn bold(self) -> Self {
        self.enable_modifier(Modifier::Bold)
    }

    /// Combine both styles.
    ///
    /// The resulting style is equivalent to applying the two styles one after the other.
    pub const fn combine(mut self, other: Self) -> Self {
        if other.reset_all {
            self = Self::new();
        }
        if let Some(color) = other.foreground_color {
            self.foreground_color = Some(color);
        }
        if let Some(color) = other.background_color {
            self.background_color = Some(color);
        }
        self.enable_modifiers = self
            .enable_modifiers
            .difference(other.disable_modifiers)
            .union(other.enable_modifiers);
        self.disable_modifiers = self
            .disable_modifiers
            .difference(other.enable_modifiers)
            .union(other.disable_modifiers);
        self
    }

    style_color_methods! {
        black, on_black, Black;
        red, on_red, Red;
        green, on_green, Green;
        yellow, on_yellow, Yellow;
        blue, on_blue, Blue;
        magenta, on_magenta, Magenta;
        cyan, on_cyan, Cyan;
        gray, on_gray, Gray;
        dark_gray, on_dark_gray, DarkGray;
        bright_red, on_bright_red, BrightRed;
        bright_green, on_bright_green, BrightGreen;
        bright_yellow, on_bright_yellow, BrightYellow;
        bright_blue, on_bright_blue, BrightBlue;
        bright_magenta, on_bright_magenta, BrightMagenta;
        bright_cyan, on_bright_cyan, BrightCyan;
        white, on_white, White;
    }

    /// Write the style's ANSI escape sequences to the provided writer.
    pub(crate) fn write_to<W: fmt::Write>(&self, mut writer: W) -> fmt::Result {
        if self.reset_all {
            writer.write_str(RESET_ALL)?;
        }
        if let Some(color) = self.foreground_color {
            writer.write_str(color.foreground_ansi_sequence())?;
        }
        if let Some(color) = self.background_color {
            writer.write_str(color.background_ansi_sequence())?;
        }
        for modifier in self.enable_modifiers.iter() {
            writer.write_str(modifier.enable_ansi_sequence())?;
        }
        for modifier in self.disable_modifiers.iter() {
            writer.write_str(modifier.disable_ansi_sequence())?;
        }
        Ok(())
    }
}

/// Helper macro for implementing the different color methods on [`Stylize`].
macro_rules! stylize_color_methods {
    ($($fg_color:ident, $bg_color:ident, $color:ident;)*) => {
        $(
            #[doc = concat!("Set foreground color to [`Color::", stringify!($color), "`].")]
            fn $fg_color(mut self) -> Self where Self: Sized {
                self.set_foreground_color(Color::$color);
                self
            }

            #[doc = concat!("Set background color to [`Color::", stringify!($color), "`].")]
            fn $bg_color(mut self) -> Self where Self: Sized {
                self.set_background_color(Color::$color);
                self
            }
        )*
    };
}

/// Trait for objects that can be styled.
pub trait Stylize {
    /// Reset all styles.
    fn reset_all_styles(&mut self) -> &mut Self;

    /// Set the foreground color.
    fn set_foreground_color(&mut self, color: Color) -> &mut Self;
    /// Set the background color.
    fn set_background_color(&mut self, color: Color) -> &mut Self;

    /// Enable a style modifier.
    fn enable_style_modifier(&mut self, modifier: Modifier) -> &mut Self;
    /// Disable a style modifier.
    fn disable_style_modifier(&mut self, modifier: Modifier) -> &mut Self;

    /// Enable the [`Modifier::Bold`] modifier.
    fn bold(mut self) -> Self
    where
        Self: Sized,
    {
        self.enable_style_modifier(Modifier::Bold);
        self
    }

    stylize_color_methods! {
        black, on_black, Black;
        red, on_red, Red;
        green, on_green, Green;
        yellow, on_yellow, Yellow;
        blue, on_blue, Blue;
        magenta, on_magenta, Magenta;
        cyan, on_cyan, Cyan;
        gray, on_gray, Gray;
        dark_gray, on_dark_gray, DarkGray;
        bright_red, on_bright_red, BrightRed;
        bright_green, on_bright_green, BrightGreen;
        bright_yellow, on_bright_yellow, BrightYellow;
        bright_blue, on_bright_blue, BrightBlue;
        bright_magenta, on_bright_magenta, BrightMagenta;
        bright_cyan, on_bright_cyan, BrightCyan;
        white, on_white, White;
    }
}

impl Stylize for Style {
    fn reset_all_styles(&mut self) -> &mut Self {
        *self = self.reset_all();
        self
    }

    fn set_foreground_color(&mut self, color: Color) -> &mut Self {
        *self = self.foreground_color(color);
        self
    }

    fn set_background_color(&mut self, color: Color) -> &mut Self {
        *self = self.background_color(color);
        self
    }

    fn enable_style_modifier(&mut self, modifier: Modifier) -> &mut Self {
        *self = self.enable_modifier(modifier);
        self
    }

    fn disable_style_modifier(&mut self, modifier: Modifier) -> &mut Self {
        *self = self.disable_modifier(modifier);
        self
    }
}

/// Style combined with some value.
#[derive(Debug, Clone)]
pub struct Styled<T> {
    /// Style.
    pub style: Style,
    /// Value.
    pub value: T,
}

impl<T> Styled<T> {
    /// Apply the provided style to the provided widget.
    pub fn new(value: T) -> Self {
        Self {
            style: Style::new(),
            value,
        }
    }

    /// Replaces the style of the styled value.
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl<T> Stylize for Styled<T> {
    fn reset_all_styles(&mut self) -> &mut Self {
        self.style.reset_all_styles();
        self
    }

    fn set_foreground_color(&mut self, color: crate::style::Color) -> &mut Self {
        self.style.set_foreground_color(color);
        self
    }

    fn set_background_color(&mut self, color: crate::style::Color) -> &mut Self {
        self.style.set_background_color(color);
        self
    }

    fn enable_style_modifier(&mut self, modifier: crate::style::Modifier) -> &mut Self {
        self.style.enable_style_modifier(modifier);
        self
    }

    fn disable_style_modifier(&mut self, modifier: crate::style::Modifier) -> &mut Self {
        self.style.disable_style_modifier(modifier);
        self
    }
}
