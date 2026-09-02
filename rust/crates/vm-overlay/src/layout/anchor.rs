//! Anchor 的 Rust 移植 (src/ui/layout/Anchor.java)。
//! PORT: Java 枚举常量 SCREAMING_SNAKE (TOP_LEFT) → Rust 变体 PascalCase
//! (TopLeft); `this == TOP_LEFT` 引用判定 → matches! (枚举值比较等价)。

/// Defines anchor points for component alignment.
/// Used for both "Self Anchor" (which point of the component aligns to the
/// target)
/// and "Target Anchor" (which point of the parent/screen is the target).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Anchor {
    TopLeft,
    TopCenter,
    TopRight,
    MiddleLeft,
    Center,
    MiddleRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

impl Anchor {
    pub fn is_left(&self) -> bool {
        matches!(self, Anchor::TopLeft | Anchor::MiddleLeft | Anchor::BottomLeft)
    }

    pub fn is_right(&self) -> bool {
        matches!(self, Anchor::TopRight | Anchor::MiddleRight | Anchor::BottomRight)
    }

    pub fn is_top(&self) -> bool {
        matches!(self, Anchor::TopLeft | Anchor::TopCenter | Anchor::TopRight)
    }

    pub fn is_bottom(&self) -> bool {
        matches!(self, Anchor::BottomLeft | Anchor::BottomCenter | Anchor::BottomRight)
    }

    pub fn is_center_horizontal(&self) -> bool {
        matches!(self, Anchor::TopCenter | Anchor::Center | Anchor::BottomCenter)
    }

    pub fn is_center_vertical(&self) -> bool {
        matches!(self, Anchor::MiddleLeft | Anchor::Center | Anchor::MiddleRight)
    }
}

#[cfg(test)]
mod tests;
