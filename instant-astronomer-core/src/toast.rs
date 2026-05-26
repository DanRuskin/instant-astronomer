//! Transient on-screen toast for action feedback.
//!
//! The mobile control bar uses icon-only buttons to save horizontal
//! space, so the labels that previously told the user what just
//! happened ("Locate me", "Calibrate", "Constellations") are gone.
//! Toasts replace that signal: a short message slides in at the top
//! of the sky view when an action fires, then fades out.
//!
//! Cell-based design so action callbacks (geo button click,
//! constellation toggle, etc.) can write without holding a
//! reference to the renderer. The `SkyViewWidget` reads the cell
//! every paint and renders the card with a fading alpha based on
//! how long ago it was set.

use std::cell::RefCell;
use std::rc::Rc;

/// One transient message + when it was set, in Unix epoch ms.
#[derive(Debug, Clone)]
pub struct ToastState {
    pub message: String,
    pub set_at_ms: i64,
}

/// Shared toast cell. `None` means "no active toast". Cloned by the
/// SkyView (read at paint time) and by every action closure that
/// might want to surface feedback (write).
pub type ToastCell = Rc<RefCell<Option<ToastState>>>;

/// Create an empty toast cell.
pub fn new_toast_cell() -> ToastCell {
    Rc::new(RefCell::new(None))
}

/// How long the toast stays fully opaque before it starts fading,
/// plus the fade-out window. The two together make
/// `TOTAL_VISIBLE_MS`.
pub const HOLD_MS: i64 = 1400;
pub const FADE_MS: i64 = 400;

/// Total time after which `paint_toast` skips drawing entirely.
pub const TOTAL_VISIBLE_MS: i64 = HOLD_MS + FADE_MS;

/// Set the toast message and timestamp. Callers should follow up
/// with `agg_gui::animation::request_draw()` so the next frame picks
/// it up — done in the per-action helper below.
pub fn show(cell: &ToastCell, message: impl Into<String>) {
    let now = crate::current_unix_ms();
    *cell.borrow_mut() = Some(ToastState {
        message: message.into(),
        set_at_ms: now,
    });
    agg_gui::animation::request_draw();
}

/// Compute the opacity multiplier `[0, 1]` for a toast based on how
/// long ago it was set. Returns `None` once the toast has fully
/// faded so callers can short-circuit.
pub fn opacity_for(state: &ToastState, now_ms: i64) -> Option<f64> {
    let elapsed = (now_ms - state.set_at_ms).max(0);
    if elapsed >= TOTAL_VISIBLE_MS {
        return None;
    }
    if elapsed <= HOLD_MS {
        return Some(1.0);
    }
    let fade_progress = (elapsed - HOLD_MS) as f64 / FADE_MS as f64;
    Some((1.0 - fade_progress).clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opacity_holds_then_fades() {
        let state = ToastState {
            message: String::from("hi"),
            set_at_ms: 1000,
        };
        // Immediately visible.
        assert_eq!(opacity_for(&state, 1000), Some(1.0));
        // Mid-hold.
        assert_eq!(opacity_for(&state, 1000 + HOLD_MS / 2), Some(1.0));
        // Just past hold = fully opaque still (boundary inclusive).
        assert_eq!(opacity_for(&state, 1000 + HOLD_MS), Some(1.0));
        // Halfway through fade.
        let halfway = opacity_for(&state, 1000 + HOLD_MS + FADE_MS / 2).unwrap();
        assert!((halfway - 0.5).abs() < 0.05, "got {halfway}");
        // Past total — gone.
        assert_eq!(opacity_for(&state, 1000 + TOTAL_VISIBLE_MS), None);
        assert_eq!(opacity_for(&state, 1000 + TOTAL_VISIBLE_MS + 100), None);
    }
}
