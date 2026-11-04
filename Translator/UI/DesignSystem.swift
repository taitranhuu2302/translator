import SwiftUI

/// Design tokens for the whole app.
///
/// Direction: tool-grade chrome (tight 4pt grid, one accent, hairline borders,
/// almost no shadow) with an editorial treatment for translated prose (serif,
/// generous line spacing) inside the Translate Window.
enum DS {
    enum Spacing {
        static let xs: CGFloat = 4
        static let s: CGFloat = 8
        static let m: CGFloat = 12
        static let l: CGFloat = 16
        static let xl: CGFloat = 24
        static let xxl: CGFloat = 32
    }

    enum Radius {
        static let small: CGFloat = 6
        static let medium: CGFloat = 10
        static let large: CGFloat = 14
    }

    enum Palette {
        static let accent = Color(red: 0.94, green: 0.42, blue: 0.18)
        static let surface = Color(nsColor: .windowBackgroundColor)
        static let raised = Color(nsColor: .controlBackgroundColor)
        static let sunken = Color(nsColor: .underPageBackgroundColor)
        static let border = Color.primary.opacity(0.09)
        static let subtleText = Color.secondary
    }

    enum Typography {
        static let caption = Font.system(size: 11, weight: .medium)
        static let body = Font.system(size: 13)
        static let emphasized = Font.system(size: 13, weight: .semibold)
        static let title = Font.system(size: 15, weight: .semibold)
        static let hero = Font.system(size: 20, weight: .semibold)
        static let reading = Font.system(size: 15, design: .serif)
        static let readingSmall = Font.system(size: 13, design: .serif)
    }

    enum Motion {
        static let quick = Animation.easeOut(duration: 0.12)
        static let standard = Animation.easeInOut(duration: 0.18)
    }
}
