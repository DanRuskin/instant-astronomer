//! Rigid-body sensor-fusion for the device-orientation channel.
//!
//! Extracted from `lib.rs` to keep that file under the workspace
//! line-count guardrail. Contains the slerp-weight math, the
//! `apply_device_orientation` entry point WASM calls on every
//! `deviceorientation` event, and the `view_quat_heading_rad`
//! helper the Calibrate button uses to snapshot the current
//! compass heading.

use nalgebra::UnitQuaternion;

use crate::AstronomerHandles;

/// Extract the W3C-convention compass heading (CCW from north, in
/// radians) from a world→view quaternion. Used by the Calibrate
/// button and the HorizonTapeWidget so they agree on "which direction
/// is the camera pointing right now?"
///
/// Implementation: the camera-forward direction in **world** coords is
/// `view_quat.inverse() * (0, 0, 1)`. Heading = `atan2(-x, z)` puts
/// north (0,0,1)→0, east (1,0,0)→-π/2 (i.e. CCW = +90°/east in W3C
/// world). Negating recovers W3C alpha.
pub fn view_quat_heading_rad(view_quat: UnitQuaternion<f64>) -> f64 {
    let forward_world = view_quat.inverse_transform_vector(&nalgebra::Vector3::new(0.0, 0.0, 1.0));
    -forward_world.x.atan2(forward_world.z)
}

/// Slerp weight for **tilt-dominated** events (pitch / roll). Tilt is
/// driven by the device's accelerometer — physical and stable — so we
/// track it aggressively. Matches the high-alpha (0.7) damping Sky Map
/// applies to its `TYPE_ACCELEROMETER` channel.
pub const FUSION_TILT_WEIGHT: f64 = 0.30;

/// Angle gap (radians) at which a **yaw-dominated** event reaches the
/// full tilt-weight pass-through. Below this knee, the slerp weight
/// scales quadratically with the gap — tiny compass jitter (a few
/// tenths of a degree) is essentially frozen, while genuine head
/// turns pass through. Mirrors the `ExponentiallyWeightedSmoother`
/// shape Sky Map runs on its magnetometer channel, lifted to
/// quaternion space.
pub const FUSION_YAW_KNEE_RAD: f64 = 5.0 * std::f64::consts::PI / 180.0;

/// Slerp weight for a sensor-fusion event, computed from the
/// rotation needed to take `current` → `target`. Single coherent
/// rotation, single weight — no per-Euler-axis filtering — but the
/// weight depends on **what kind** of rotation it is.
///
/// Yaw rotations (axis ≈ world-up) get magnitude-gain smoothing:
/// tiny gaps are crushed, large gaps follow. Tilt rotations (axis ≈
/// horizontal) get the full `FUSION_TILT_WEIGHT`. Mixed axes
/// linearly interpolate, so the slerp remains rigid-body coherent —
/// no risk of yaw lagging pitch when the user turns the phone.
fn fusion_slerp_weight(
    current: UnitQuaternion<f64>,
    target: UnitQuaternion<f64>,
) -> f64 {
    let delta = target * current.inverse();
    let angle = delta.angle();
    if angle < 1e-9 {
        return 0.0;
    }
    let yaw_share = delta.axis().map(|a| a.y.abs()).unwrap_or(0.0);
    let yaw_gain = (angle / FUSION_YAW_KNEE_RAD).powi(2).min(1.0);
    let yaw_weight = FUSION_TILT_WEIGHT * yaw_gain;
    yaw_share * yaw_weight + (1.0 - yaw_share) * FUSION_TILT_WEIGHT
}

/// Apply a device-orientation reading to the shared `view_quat` using
/// rigid-body sensor fusion: slerp the **whole** quaternion toward
/// the target each event, with a magnitude- and axis-dependent weight.
///
/// The earlier per-axis approach (low-pass alpha, pass beta through
/// unfiltered) violated the geometric coupling between yaw and pitch
/// — when the user turned the phone, pitch updated immediately and
/// yaw arrived 200 ms later, producing the "view swings around
/// later" feel reported on mobile. Filtering the orientation as a
/// single rigid-body rotation fixes that, but a fixed slerp weight
/// trades off compass jitter against responsive tilt tracking.
///
/// Sky Map (sky-map-team/stardroid) resolves the same trade-off by
/// running separate damping on the gravity channel (alpha 0.7,
/// responsive) and the magnetometer channel (alpha 0.05 plus a cubic
/// `ExponentiallyWeightedSmoother` that crushes sub-degree jitter).
/// We can't separate the channels — the browser hands us a fused
/// Euler triple — but we can recover the same effect by looking at
/// the **axis** of the current→target rotation: if it points along
/// world-up the change is yaw-like (compass-driven, smooth hard);
/// otherwise it's tilt-like (gravity-driven, follow fast). See
/// [`fusion_slerp_weight`] for the math.
///
/// Inputs are radians: `yaw_rad` is W3C alpha (CCW from north),
/// `pitch_rad` is W3C beta minus 90° (so 0 = looking at horizon).
/// Roll is intentionally dropped.
///
/// First event after the handle is created snaps to the target so
/// the view doesn't visibly drift from identity to the device's
/// real orientation over half a second on startup.
pub fn apply_device_orientation(
    handles: &AstronomerHandles,
    yaw_rad: f64,
    pitch_rad: f64,
) {
    if !handles.use_device_orientation.get() {
        return;
    }
    let q_yaw =
        UnitQuaternion::from_axis_angle(&nalgebra::Vector3::y_axis(), yaw_rad);
    let q_pitch =
        UnitQuaternion::from_axis_angle(&nalgebra::Vector3::x_axis(), pitch_rad);
    let target = q_pitch * q_yaw;

    let next = if handles.fusion_seeded.get() {
        let current = handles.view_quat.get();
        let weight = fusion_slerp_weight(current, target);
        current.slerp(&target, weight)
    } else {
        // First event — snap so we don't visibly drift from identity
        // (or from wherever a previous mouse drag left things)
        // toward the device's actual orientation.
        handles.fusion_seeded.set(true);
        target
    };
    handles.view_quat.set(next);
    agg_gui::animation::request_draw();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    fn make_handles() -> AstronomerHandles {
        AstronomerHandles {
            latitude: Rc::new(Cell::new(0.0)),
            longitude: Rc::new(Cell::new(0.0)),
            timestamp_ms: Rc::new(Cell::new(0)),
            view_quat: Rc::new(Cell::new(UnitQuaternion::<f64>::identity())),
            calibration_yaw: Rc::new(Cell::new(0.0)),
            use_device_orientation: Rc::new(Cell::new(true)),
            fusion_seeded: Rc::new(Cell::new(false)),
        }
    }

    /// First `apply_device_orientation` event should snap so the view
    /// doesn't visibly drift from identity to the device's real
    /// orientation over ~500 ms on startup. The user reported this
    /// as a "settling" pause when first turning the compass on.
    #[test]
    fn apply_device_orientation_snaps_first_event() {
        let h = make_handles();
        apply_device_orientation(&h, 1.0, 0.5);
        let q = h.view_quat.get();
        let expected = UnitQuaternion::from_axis_angle(&nalgebra::Vector3::x_axis(), 0.5)
            * UnitQuaternion::from_axis_angle(&nalgebra::Vector3::y_axis(), 1.0);
        assert!(
            q.angle_to(&expected) < 1e-9,
            "first event must snap to target, off by {} rad",
            q.angle_to(&expected)
        );
        assert!(h.fusion_seeded.get(), "fusion_seeded should flip true");
    }

    /// A large yaw-dominated event after the snap should pass
    /// through at `FUSION_TILT_WEIGHT` — well above the knee, the
    /// quadratic gain saturates at 1.0 so yaw weight equals tilt
    /// weight. The slerp is still a single rigid-body rotation.
    #[test]
    fn apply_device_orientation_slerps_large_yaw_at_full_weight() {
        let h = make_handles();
        apply_device_orientation(&h, 1.0, 0.5); // snap
        let q_first = h.view_quat.get();
        apply_device_orientation(&h, 2.0, 0.5); // 1 rad yaw gap, well above knee
        let q_second = h.view_quat.get();
        let target = UnitQuaternion::from_axis_angle(&nalgebra::Vector3::x_axis(), 0.5)
            * UnitQuaternion::from_axis_angle(&nalgebra::Vector3::y_axis(), 2.0);
        let total = q_first.angle_to(&target);
        let moved = q_first.angle_to(&q_second);
        let ratio = moved / total;
        assert!(
            (ratio - FUSION_TILT_WEIGHT).abs() < 0.02,
            "large-gap slerp ratio should be ~{FUSION_TILT_WEIGHT}, got {ratio:.3}"
        );
    }

    /// Sub-degree yaw "jitter" — typical of magnetometer noise — must
    /// be crushed. Mirrors Sky Map's `ExponentiallyWeightedSmoother`
    /// behaviour on its magnetometer channel: quadratic gain below
    /// the knee renders compass noise effectively frozen.
    #[test]
    fn apply_device_orientation_crushes_small_yaw_jitter() {
        let h = make_handles();
        apply_device_orientation(&h, 0.0, 0.0); // snap to identity
        let q_seed = h.view_quat.get();
        let jitter = 0.5_f64.to_radians();
        apply_device_orientation(&h, jitter, 0.0);
        let moved = q_seed.angle_to(&h.view_quat.get());
        let ratio = moved / jitter;
        // (0.5° / 5°)² * 0.30 = 0.003 — view barely moves.
        assert!(
            ratio < 0.01,
            "small yaw jitter must be crushed, ratio={ratio:.4}"
        );
    }

    /// A tilt-dominated event (gravity is the stable channel) must
    /// pass through at full `FUSION_TILT_WEIGHT` even for small
    /// angles. We don't deadband pitch — that's what gave the
    /// "settles late" feel on real motion.
    #[test]
    fn apply_device_orientation_tracks_small_tilt() {
        let h = make_handles();
        apply_device_orientation(&h, 0.0, 0.0); // snap to identity
        let q_seed = h.view_quat.get();
        let tilt = 0.5_f64.to_radians();
        apply_device_orientation(&h, 0.0, tilt);
        let moved = q_seed.angle_to(&h.view_quat.get());
        let ratio = moved / tilt;
        assert!(
            (ratio - FUSION_TILT_WEIGHT).abs() < 0.02,
            "small tilt should track at {FUSION_TILT_WEIGHT}, got {ratio:.3}"
        );
    }

    /// `use_device_orientation = false` should leave view_quat alone
    /// even when an event fires. Also must NOT flip `fusion_seeded`
    /// — otherwise re-enabling the compass would silently skip the
    /// startup snap on the next event.
    #[test]
    fn apply_device_orientation_no_op_when_disabled() {
        let h = make_handles();
        h.use_device_orientation.set(false);
        apply_device_orientation(&h, 1.0, 0.5);
        assert!(h.view_quat.get().angle() < 1e-9, "view_quat must not change");
        assert!(!h.fusion_seeded.get(), "must not seed while disabled");
    }
}
