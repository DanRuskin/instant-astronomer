//! On-sky "where do I point?" finder for the search feature.
//!
//! Once the user picks a search target ([`crate::search::SearchTarget`]),
//! the sky view calls [`paint`] every frame. It resolves the target to a
//! live direction, then draws:
//!
//! * a pulsing highlight ring when the target is on-screen,
//! * a compact direction arrow hugging the reticle that points toward the
//!   target (including a "behind you" direction when it's behind the
//!   camera), and
//! * a distance gauge ring around the reticle whose fill fraction and
//!   colour encode how far the view centre is from the target — it fills
//!   toward a full red ring as you move away.
//!
//! Pulled into its own module so `sky_view.rs` stays under the workspace
//! 800-line guardrail. The geometry helper [`turn_guidance`] is pure and
//! unit-tested.

use std::cell::RefCell;
use std::rc::Rc;

use agg_gui::color::Color;
use agg_gui::draw_ctx::DrawCtx;
use agg_gui::geometry::Point;
use nalgebra::Matrix3;

use crate::math::{equatorial_to_horizontal, horizontal_to_cartesian, HorizontalCoords};
use crate::search::{resolve_target_coords, SearchTarget};

/// Below this separation the target counts as "found" — the proximity
/// gauge is full and we drop the direction arrow.
const FOUND_THRESHOLD_RAD: f64 = 0.035; // ~2°

/// Separation at which the proximity gauge reads empty. Anything wider
/// than this maps to the same "far" state.
const GAUGE_FULL_SCALE_RAD: f64 = std::f64::consts::FRAC_PI_2; // 90°

/// Radius (logical px) of the proximity gauge ring around the reticle.
const GAUGE_RADIUS: f64 = 26.0;

/// Angular deltas needed to bring a target to the centre of the view.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TurnGuidance {
    /// Total great-circle separation between view centre and the target.
    pub sep_rad: f64,
    /// Wrapped `target_az - view_az` in `[-π, π]`. Positive = the target
    /// is clockwise of the current heading, i.e. "turn right".
    pub delta_az: f64,
    /// `target_alt - centre_alt`. Positive = the target is higher, i.e.
    /// "tilt up".
    pub delta_alt: f64,
}

/// Compute the turn / tilt guidance from the current view direction
/// (`view_az`, `centre_alt`, both radians) to a target in horizontal
/// coordinates. Pure so it can be unit-tested without a render context.
pub fn turn_guidance(target: HorizontalCoords, view_az: f64, centre_alt: f64) -> TurnGuidance {
    TurnGuidance {
        sep_rad: angular_separation(target.alt, target.az, centre_alt, view_az),
        delta_az: wrap_pi(target.az - view_az),
        delta_alt: target.alt - centre_alt,
    }
}

/// Great-circle angle between two horizontal directions (spherical law of
/// cosines). Clamped against float drift so `acos` never sees > 1.
fn angular_separation(alt1: f64, az1: f64, alt2: f64, az2: f64) -> f64 {
    let cos_sep =
        alt1.sin() * alt2.sin() + alt1.cos() * alt2.cos() * (az1 - az2).cos();
    cos_sep.clamp(-1.0, 1.0).acos()
}

/// Wrap an angle into `[-π, π]`.
fn wrap_pi(mut a: f64) -> f64 {
    use std::f64::consts::PI;
    while a > PI {
        a -= 2.0 * PI;
    }
    while a < -PI {
        a += 2.0 * PI;
    }
    a
}

/// Paint the finder for the currently-selected target, if any. No-op when
/// no target is set or it can't be resolved.
#[allow(clippy::too_many_arguments)]
pub(super) fn paint(
    ctx: &mut dyn DrawCtx,
    w: f64,
    h: f64,
    rot: &Matrix3<f64>,
    center: Point,
    focal_length: f64,
    target_cell: &Rc<RefCell<Option<SearchTarget>>>,
    lat: f64,
    lst: f64,
    now_ms: i64,
) {
    let borrow = target_cell.borrow();
    let Some(target) = borrow.as_ref() else {
        return;
    };
    let Some(coords) = resolve_target_coords(target, now_ms) else {
        return;
    };
    let target_horiz = equatorial_to_horizontal(coords, lat, lst);

    // Target direction in camera space (x = right, y = up, z = forward).
    let v_cart = horizontal_to_cartesian(target_horiz);
    let v_rot = rot * v_cart;

    // Current view heading + pitch from the camera-forward row of `rot`
    // (same extraction the altitude HUD uses), so guidance stays in lock-
    // step with what's actually drawn this frame.
    let cam_forward = nalgebra::Vector3::new(rot[(2, 0)], rot[(2, 1)], rot[(2, 2)]);
    let cam_len = cam_forward.norm().max(1e-9);
    let view_az = cam_forward.x.atan2(cam_forward.z);
    let centre_alt = (cam_forward.y / cam_len).asin();
    let guidance = turn_guidance(target_horiz, view_az, centre_alt);

    let accent = Color::from_rgb8(120, 255, 170);

    // Screen-space direction from the reticle toward the target, plus the
    // on-screen highlight when the object itself is visible.
    let angle = if v_rot.z > 0.05 {
        let screen = Point::new(
            center.x + (v_rot.x / v_rot.z) * focal_length,
            center.y + (v_rot.y / v_rot.z) * focal_length,
        );
        let on_screen =
            screen.x >= 0.0 && screen.x <= w && screen.y >= 0.0 && screen.y <= h;
        if on_screen {
            paint_highlight_ring(ctx, screen, now_ms, accent);
        }
        (screen.y - center.y).atan2(screen.x - center.x)
    } else {
        // Behind the camera: point along the target's lateral offset.
        v_rot.y.atan2(v_rot.x)
    };

    // Distance, as a colour-coded fill ring hugging the reticle.
    let prox = proximity(guidance.sep_rad);
    let color = proximity_color(prox);
    paint_proximity_gauge(ctx, center, prox, color);

    // Direction arrow tucked just outside the gauge ring, dropped once the
    // target is essentially centred.
    if guidance.sep_rad >= FOUND_THRESHOLD_RAD {
        paint_center_arrow(ctx, center, angle, color);
    }
}

/// Map a great-circle separation to a 0..1 "closeness", saturating at
/// [`GAUGE_FULL_SCALE_RAD`]. 1.0 = on target, 0.0 = far away.
fn proximity(sep_rad: f64) -> f64 {
    1.0 - (sep_rad / GAUGE_FULL_SCALE_RAD).clamp(0.0, 1.0)
}

/// Lerp the gauge colour from red (far) through to green (close).
fn proximity_color(prox: f64) -> Color {
    let lerp = |a: f64, b: f64| (a + (b - a) * prox).round() as u8;
    Color::from_rgb8(lerp(255.0, 120.0), lerp(90.0, 255.0), lerp(90.0, 170.0))
}

/// Distance gauge: a faint full ring around the reticle with a coloured arc on
/// top whose sweep grows as the target gets farther from the centre.
fn paint_proximity_gauge(ctx: &mut dyn DrawCtx, center: Point, prox: f64, color: Color) {
    use std::f64::consts::PI;

    ctx.set_line_width(3.0);
    ctx.set_stroke_color(Color::from_rgba8(255, 255, 255, 38));
    ctx.begin_path();
    ctx.circle(center.x, center.y, GAUGE_RADIUS);
    ctx.stroke();

    let distance = 1.0 - prox;
    if distance > 0.001 {
        let start = PI * 0.5; // top of the ring
        let end = start + distance * 2.0 * PI;
        ctx.set_stroke_color(color);
        ctx.begin_path();
        ctx.arc_to(center.x, center.y, GAUGE_RADIUS, start, end, false);
        ctx.stroke();
    }
}

/// Pulsing ring + tick marks centred on an on-screen target.
fn paint_highlight_ring(ctx: &mut dyn DrawCtx, p: Point, now_ms: i64, color: Color) {
    use std::f64::consts::PI;
    // Smooth 0..1 pulse on a ~1.1 s cycle.
    let phase = (now_ms.rem_euclid(1100) as f64) / 1100.0;
    let pulse = 0.5 - 0.5 * (phase * 2.0 * PI).cos();
    let base_r = 16.0;
    let r = base_r + pulse * 10.0;
    agg_gui::animation::request_draw_without_invalidation();

    ctx.set_stroke_color(color);
    ctx.set_line_width(2.0);
    ctx.begin_path();
    ctx.circle(p.x, p.y, r);
    ctx.stroke();

    // Four short ticks at the cardinal points of the ring.
    let tick = 6.0;
    for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
        ctx.begin_path();
        ctx.move_to(p.x + dx * r, p.y + dy * r);
        ctx.line_to(p.x + dx * (r + tick), p.y + dy * (r + tick));
        ctx.stroke();
    }
}

/// Compact arrow tucked just outside the gauge ring, pointing toward an
/// off-centre target at `angle` (radians, screen space).
fn paint_center_arrow(ctx: &mut dyn DrawCtx, center: Point, angle: f64, color: Color) {
    let (c, s) = (angle.cos(), angle.sin());
    let inner = GAUGE_RADIUS + 4.0;
    let length = 14.0;
    let half = 7.0;

    let tip = Point::new(center.x + c * (inner + length), center.y + s * (inner + length));
    // Base corners sit on the gauge edge, perpendicular to the direction.
    let base = Point::new(center.x + c * inner, center.y + s * inner);
    let left = Point::new(base.x - s * half, base.y + c * half);
    let right = Point::new(base.x + s * half, base.y - c * half);

    ctx.set_fill_color(color);
    ctx.begin_path();
    ctx.move_to(tip.x, tip.y);
    ctx.line_to(left.x, left.y);
    ctx.line_to(right.x, right.y);
    ctx.close_path();
    ctx.fill();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn pointing_at_target_gives_zero_separation() {
        let target = HorizontalCoords {
            alt: 0.3,
            az: 1.2,
        };
        let g = turn_guidance(target, 1.2, 0.3);
        assert!(g.sep_rad < 1e-9, "sep should be ~0, got {}", g.sep_rad);
        assert!(g.delta_az.abs() < 1e-9);
        assert!(g.delta_alt.abs() < 1e-9);
    }

    #[test]
    fn target_to_the_right_is_positive_delta_az() {
        // Target azimuth slightly greater (clockwise) than the view.
        let target = HorizontalCoords { alt: 0.0, az: 1.0 };
        let g = turn_guidance(target, 0.8, 0.0);
        assert!(g.delta_az > 0.0, "expected turn-right, got {}", g.delta_az);
    }

    #[test]
    fn target_above_is_positive_delta_alt() {
        let target = HorizontalCoords { alt: 0.5, az: 0.0 };
        let g = turn_guidance(target, 0.0, 0.2);
        assert!(g.delta_alt > 0.0, "expected tilt-up, got {}", g.delta_alt);
        assert!((g.delta_alt - 0.3).abs() < 1e-9);
    }

    #[test]
    fn delta_az_takes_short_way_across_the_seam() {
        // View near 350°, target near 10°: shortest turn is +20° (right),
        // not -340°.
        let target = HorizontalCoords {
            alt: 0.0,
            az: 10.0_f64.to_radians(),
        };
        let g = turn_guidance(target, 350.0_f64.to_radians(), 0.0);
        assert!(g.delta_az > 0.0, "should turn right across the seam");
        assert!(
            (g.delta_az - 20.0_f64.to_radians()).abs() < 1e-9,
            "expected +20°, got {}°",
            g.delta_az.to_degrees()
        );
    }

    #[test]
    fn separation_is_symmetric_and_bounded() {
        let a = angular_separation(0.1, 0.2, -0.4, 3.0);
        let b = angular_separation(-0.4, 3.0, 0.1, 0.2);
        assert!((a - b).abs() < 1e-12);
        assert!((0.0..=PI).contains(&a));
    }
}
