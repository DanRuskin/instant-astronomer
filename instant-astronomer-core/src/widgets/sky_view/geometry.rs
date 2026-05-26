//! 2-D screen-space geometry helpers used by `sky_view` for tap +
//! reticle hit-testing. Pulled out of `sky_view.rs` to keep that
//! file under the 800-line guardrail.

use agg_gui::geometry::Point;

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
