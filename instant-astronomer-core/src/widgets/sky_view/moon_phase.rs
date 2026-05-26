//! Moon phase computation + painter.
//!
//! Pulled out of `sky_view.rs` to keep that file under the 800-line
//! guardrail; the entry point is [`fill_moon_phase`]. Two pieces of
//! information are required at paint time:
//!
//! - the **illuminated fraction** `k ∈ [0, 1]` (0 = new, 1 = full),
//!   derived from the geocentric elongation between Sun and Moon;
//! - the **screen-space direction to the Sun**, which gives the
//!   orientation of the bright limb. The terminator is perpendicular
//!   to this direction.
//!
//! [`moon_phase_info`] bundles both, and [`fill_moon_phase`] does the
//! drawing — including the < 10 % illuminated "outline only" fallback
//! so the dark Moon stays visible against the night sky.

use agg_gui::color::Color;
use agg_gui::draw_ctx::DrawCtx;
use agg_gui::geometry::Point;

use crate::math::{equatorial_to_horizontal, horizontal_to_cartesian, EquatorialCoords};

/// Phase information for the Moon, gathered once per frame so the
/// painter has everything it needs without re-running the ephemeris.
#[derive(Debug, Clone, Copy)]
pub(super) struct MoonPhaseInfo {
    /// Illuminated fraction in `[0, 1]`. 0 = new, 1 = full.
    pub illumination: f64,
    /// Unit vector pointing from the Moon toward the Sun in screen
    /// space (Y-up). The terminator ellipse's major axis is
    /// perpendicular to this; the bright limb faces this way.
    pub sun_dir: (f64, f64),
}

/// Illuminated fraction of the Moon in `[0, 1]`. From the angle
/// between the Sun's and Moon's geocentric directions (elongation):
/// `k = (1 - cos ψ) / 2`, exact for the simple Earth-Sun-Moon model
/// and well within naked-eye accuracy.
pub(super) fn moon_illumination(sun: EquatorialCoords, moon: EquatorialCoords) -> f64 {
    let cos_e = sun.dec.sin() * moon.dec.sin()
        + sun.dec.cos() * moon.dec.cos() * (sun.ra - moon.ra).cos();
    let cos_e = cos_e.clamp(-1.0, 1.0);
    ((1.0 - cos_e) / 2.0).clamp(0.0, 1.0)
}

/// Bundle illumination + screen-space sun direction for the painter.
/// The screen direction is computed by rotating the Sun-Moon vector
/// (in world Y-up horizontal coords) through the camera matrix and
/// keeping the (x, y) components — perspective doesn't change a
/// direction's screen orientation, only its magnitude, so we don't
/// need to perspective-divide.
pub(super) fn moon_phase_info(
    sun: EquatorialCoords,
    moon: EquatorialCoords,
    lat_rad: f64,
    lst_rad: f64,
    rot: &nalgebra::Matrix3<f64>,
) -> MoonPhaseInfo {
    let illum = moon_illumination(sun, moon);
    let sun_h = equatorial_to_horizontal(sun, lat_rad, lst_rad);
    let moon_h = equatorial_to_horizontal(moon, lat_rad, lst_rad);
    let sun_cart = horizontal_to_cartesian(sun_h);
    let moon_cart = horizontal_to_cartesian(moon_h);
    let dir_world = sun_cart - moon_cart;
    let dir_view = rot * dir_world;
    let dx = dir_view.x;
    let dy = dir_view.y;
    let len = (dx * dx + dy * dy).sqrt();
    let sun_dir = if len > 1e-9 {
        (dx / len, dy / len)
    } else {
        (1.0, 0.0)
    };
    MoonPhaseInfo {
        illumination: illum,
        sun_dir,
    }
}

/// Paint the Moon with its current phase. The lit region is a classic
/// lune: half of the Moon's circumference on the Sun-facing side, plus
/// the terminator (an ellipse arc whose semi-minor axis along the Sun
/// direction is `r * (2k - 1)`). When the Moon is more than 90 % new
/// (less than 10 % illuminated) we drop the lit fill entirely and
/// render the disc outline so the dark Moon still tells the user
/// where it is.
pub(super) fn fill_moon_phase(
    ctx: &mut dyn DrawCtx,
    pos: Point,
    r: f64,
    info: Option<MoonPhaseInfo>,
) {
    let Some(info) = info else {
        // No Sun coords available — fall back to a plain bright disc.
        ctx.set_fill_color(Color::from_rgb8(220, 220, 240));
        ctx.begin_path();
        ctx.circle(pos.x, pos.y, r);
        ctx.fill();
        return;
    };
    let bright = Color::from_rgb8(230, 230, 245);
    let outline = Color::from_rgba8(220, 220, 240, 200);

    if info.illumination < 0.10 {
        // Near-new: nothing lit worth filling. Outline ring keeps the
        // body locatable against the night sky.
        ctx.set_stroke_color(outline);
        ctx.set_line_width(1.2);
        ctx.begin_path();
        ctx.circle(pos.x, pos.y, r);
        ctx.stroke();
        return;
    }
    if info.illumination > 0.99 {
        // Full: simpler to just fill a disc than to build a
        // degenerate path.
        ctx.set_fill_color(bright);
        ctx.begin_path();
        ctx.circle(pos.x, pos.y, r);
        ctx.fill();
        return;
    }

    // Lit region path. θ runs from -π/2 to +π/2:
    //   bright arc:  pos + r·(cos θ · sun + sin θ · term)
    //   terminator:  pos + r·(2k-1)·cos θ · sun + r · sin θ · term
    // Both share the two "horns" (θ = ±π/2). Sample as a polygon —
    // agg-gui's path API tessellates it.
    let (sx, sy) = info.sun_dir;
    // Terminator direction: rotate sun_dir 90° CCW.
    let (tx, ty) = (-sy, sx);
    let k = info.illumination;
    let term_scale = 2.0 * k - 1.0;
    const SAMPLES: usize = 48;
    ctx.set_fill_color(bright);
    ctx.begin_path();
    for i in 0..=SAMPLES {
        let t = (i as f64) / (SAMPLES as f64);
        let theta = -std::f64::consts::FRAC_PI_2 + t * std::f64::consts::PI;
        let x = pos.x + r * (theta.cos() * sx + theta.sin() * tx);
        let y = pos.y + r * (theta.cos() * sy + theta.sin() * ty);
        if i == 0 {
            ctx.move_to(x, y);
        } else {
            ctx.line_to(x, y);
        }
    }
    for i in 0..=SAMPLES {
        let t = (i as f64) / (SAMPLES as f64);
        let theta = std::f64::consts::FRAC_PI_2 - t * std::f64::consts::PI;
        let cx_local = r * term_scale * theta.cos();
        let cy_local = r * theta.sin();
        let x = pos.x + cx_local * sx + cy_local * tx;
        let y = pos.y + cx_local * sy + cy_local * ty;
        ctx.line_to(x, y);
    }
    ctx.fill();
    // Outline the full disc so the dark limb is still discernible
    // even at near-quarter phase.
    ctx.set_stroke_color(outline);
    ctx.set_line_width(0.8);
    ctx.begin_path();
    ctx.circle(pos.x, pos.y, r);
    ctx.stroke();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    /// Sun and Moon at the same ecliptic longitude (new moon) → 0%
    /// illumination. Sun and Moon opposed (full moon) → 100%.
    #[test]
    fn illumination_extremes() {
        let sun = EquatorialCoords { ra: 0.0, dec: 0.0 };
        let moon_new = EquatorialCoords { ra: 0.0, dec: 0.0 };
        let moon_full = EquatorialCoords { ra: PI, dec: 0.0 };
        let moon_quarter = EquatorialCoords {
            ra: PI / 2.0,
            dec: 0.0,
        };
        assert!(moon_illumination(sun, moon_new) < 1e-9);
        assert!((moon_illumination(sun, moon_full) - 1.0).abs() < 1e-9);
        assert!((moon_illumination(sun, moon_quarter) - 0.5).abs() < 1e-9);
    }
}
