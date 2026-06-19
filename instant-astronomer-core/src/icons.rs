//! # Font Awesome Icon Constants
//!
//! Font Awesome 6 Free-Solid code points used by the configuration tray.
//! The TTF lives in `assets/fa.ttf`; load it via [`load_icon_font`] and
//! pass to `Button::with_icon` / `with_icon_sized`.

use agg_gui::text::Font;
use std::sync::Arc;

/// Font Awesome Free-Solid bundled into the binary. Matches the TTF
/// shipped with the Solitaire app (~165 KB).
pub const ICON_FONT_BYTES: &[u8] = include_bytes!("../assets/fa.ttf");

/// Parse the bundled Font Awesome TTF into an `Arc<Font>` usable by
/// `Button::with_icon`. Both shells call this once at startup; the
/// returned `Arc` is cheap to clone per button.
pub fn load_icon_font() -> Arc<Font> {
    Arc::new(Font::from_slice(ICON_FONT_BYTES).expect("instant-astronomer icon font"))
}

// Codepoints below are restricted to Font Awesome 5 Free Solid —
// the bundled `fa.ttf` has 704 glyphs from that era and lacks the
// FA-6-only `e000`-block icons. If you add a codepoint here, check
// it's present in the font (see `scripts` history for the inspector
// one-liner) — otherwise it renders as a tofu box on the button.

/// Crosshairs — used for the "Locate me" geolocation button.
pub const FA_CROSSHAIRS: char = '\u{f05b}';

/// Compass face — used for the "Calibrate to north" button.
pub const FA_COMPASS: char = '\u{f14e}';

/// Expand arrows pointing outward — used for the full-screen toggle.
pub const FA_EXPAND: char = '\u{f065}';

/// Compress arrows pointing inward — used when already full-screen.
pub const FA_COMPRESS: char = '\u{f066}';

/// Five-point star — used for the Constellations overlay toggle.
/// FA 5 codepoint; the FA 6-only `circle-nodes` icon we tried before
/// rendered as a tofu box in this font.
pub const FA_STAR: char = '\u{f005}';

/// Mobile phone — used for the "use compass / accelerometer" toggle.
/// Hints at "phone sensors drive the view"; tap to disable when the
/// magnetometer is mis-calibrated and let mouse / touch swipe take
/// over. (FA 5; the FA-6 `mobile-screen-button` we tried before
/// rendered as a tofu box.)
pub const FA_MOBILE: char = '\u{f10b}';

/// Map marker — used for the "Use geolocation" toggle on mobile so
/// the row stays icon-only.
pub const FA_MAP_MARKER: char = '\u{f041}';

/// Magnifying glass — used for the top-menu "Search" action. FA 5 Free
/// Solid; present in the bundled face.
pub const FA_SEARCH: char = '\u{f002}';

/// Vertical three-dot "kebab" — used for the top-menu "more options"
/// button that opens the lesser-used-options panel. FA 5 Free Solid.
pub const FA_ELLIPSIS_V: char = '\u{f142}';

/// Times / "x" — used for the search overlay's close button. FA 5 Free
/// Solid.
pub const FA_TIMES: char = '\u{f00d}';

/// Left arrow — used for the search overlay's "back to results" button.
/// FA 5 Free Solid.
pub const FA_ARROW_LEFT: char = '\u{f060}';
