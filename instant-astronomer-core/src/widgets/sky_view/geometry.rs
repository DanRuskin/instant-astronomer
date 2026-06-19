//! 2-D screen-space geometry + drawing primitives used by `sky_view`.
//! Pulled out of `sky_view.rs` to keep that file under the 800-line
//! guardrail.

use agg_gui::color::Color;
use agg_gui::draw_ctx::DrawCtx;
use agg_gui::geometry::{Point, Rect};

/// Fill an axis-aligned rectangle.
pub(super) fn fill_rect(ctx: &mut dyn DrawCtx, r: Rect, color: Color) {
    ctx.set_fill_color(color);
    ctx.begin_path();
    ctx.rect(r.x, r.y, r.width, r.height);
    ctx.fill();
}

/// Fill a circle (star / planet disc).
pub(super) fn fill_disc(ctx: &mut dyn DrawCtx, p: Point, radius: f64, color: Color) {
    ctx.set_fill_color(color);
    ctx.begin_path();
    ctx.circle(p.x, p.y, radius);
    ctx.fill();
}

/// Stroke a line segment (constellation lines).
pub(super) fn stroke_segment(ctx: &mut dyn DrawCtx, a: Point, b: Point, width: f64, color: Color) {
    ctx.set_stroke_color(color);
    ctx.set_line_width(width);
    ctx.begin_path();
    ctx.move_to(a.x, a.y);
    ctx.line_to(b.x, b.y);
    ctx.stroke();
}

/// Draw a single line of text at `p` (baseline) in the current font.
pub(super) fn draw_text(ctx: &mut dyn DrawCtx, p: Point, size: f64, color: Color, text: &str) {
    ctx.set_fill_color(color);
    ctx.set_font_size(size);
    ctx.fill_text(text, p.x, p.y);
}

/// Shortest distance from point `p` to the line segment `a → b`, plus
/// the closest point on the segment. Used by `SkyViewWidget::hit_test_tap`
/// (tap → constellation line) and by `paint_centre_reticle`
/// (reticle → constellation line). Handles the degenerate `a == b`
/// case as a radial distance so we never divide by zero.
pub(super) fn point_to_segment_distance(p: Point, a: Point, b: Point) -> (f64, Point) {
    let abx = b.x - a.x;
    let aby = b.y - a.y;
    let len_sq = abx * abx + aby * aby;
    if len_sq < 1e-9 {
        let dx = p.x - a.x;
        let dy = p.y - a.y;
        return ((dx * dx + dy * dy).sqrt(), a);
    }
    let t = (((p.x - a.x) * abx + (p.y - a.y) * aby) / len_sq).clamp(0.0, 1.0);
    let closest = Point::new(a.x + t * abx, a.y + t * aby);
    let dx = p.x - closest.x;
    let dy = p.y - closest.y;
    ((dx * dx + dy * dy).sqrt(), closest)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `t`-clamping branches: point projects past the endpoints should
    /// resolve to the endpoint, not to the extended line.
    #[test]
    fn segment_distance_clamps_to_endpoints() {
        let a = Point::new(0.0, 0.0);
        let b = Point::new(10.0, 0.0);
        // Past the start of the segment.
        let (d, c) = point_to_segment_distance(Point::new(-5.0, 0.0), a, b);
        assert_eq!(d, 5.0);
        assert_eq!((c.x, c.y), (0.0, 0.0));
        // Past the end.
        let (d, c) = point_to_segment_distance(Point::new(20.0, 3.0), a, b);
        assert!((d - ((10.0_f64).hypot(3.0))).abs() < 1e-9);
        assert_eq!((c.x, c.y), (10.0, 0.0));
    }

    /// Perpendicular distance to the interior of the segment is the
    /// y-offset for a horizontal segment.
    #[test]
    fn segment_distance_perpendicular_inside() {
        let a = Point::new(0.0, 0.0);
        let b = Point::new(10.0, 0.0);
        let (d, c) = point_to_segment_distance(Point::new(4.0, 7.0), a, b);
        assert_eq!(d, 7.0);
        assert_eq!((c.x, c.y), (4.0, 0.0));
    }

    /// Degenerate segment (a == b) must still yield a sane distance
    /// (radial from the shared point), not divide by zero.
    #[test]
    fn segment_distance_degenerate_handled() {
        let p = Point::new(3.0, 4.0);
        let (d, c) =
            point_to_segment_distance(p, Point::new(0.0, 0.0), Point::new(0.0, 0.0));
        assert_eq!(d, 5.0);
        assert_eq!((c.x, c.y), (0.0, 0.0));
    }
}
