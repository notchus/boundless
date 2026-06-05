import SwiftUI

/// Spacing tokens for the onboarding screens. Colors and typography are deliberately NOT tokenized
/// here: screens use SwiftUI **semantic** colors (`Color.primary`, `Color(.systemBackground)`,
/// the accent tint) and **Dynamic Type** text styles (`.font(.largeTitle/.body/…)`), so dark mode,
/// contrast, and font scaling are correct by construction (a11y bar; forbidden-patterns: no
/// hardcoded colors, no hardcoded point sizes for body text).
public enum Spacing {
    public static let small: CGFloat = 8
    public static let medium: CGFloat = 16
    public static let large: CGFloat = 24
    public static let xLarge: CGFloat = 32
}
