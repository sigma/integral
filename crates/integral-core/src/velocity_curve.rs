//! Shapes of the velocity curves (V-Curve: FIXED, 1-7) that PCM Synth Tone
//! and PCM Drum Kit TVF and TVA parameters select.

/// How strongly velocity `v` (0.0 soft to 1.0 hard) acts under `curve`, from
/// 0.0 to 1.0. FIXED (0) ignores velocity and always acts fully.
///
/// Curves 2/3 bend down and 4/5 are their mirrors; 6 is an S and 7 an
/// inverted S.
pub fn velocity_curve(curve: u8, v: f64) -> f64 {
    // shortcut: shapes traced by eye from the INTEGRA-7 Parameter Guide figure (PCMS TVF tab, p. 57), not measured; measure the device if exact response matters.
    let v = v.clamp(0.0, 1.0);
    let t = 2.0 * v - 1.0; // -1..1 around the middle velocity
    match curve {
        1 => v,
        2 => v.powi(2),
        3 => v.powi(6),
        4 => 1.0 - (1.0 - v).powi(2),
        5 => 1.0 - (1.0 - v).powi(6),
        6 => 0.5 + 0.5 * t.signum() * (1.0 - (1.0 - t.abs()).powi(4)),
        7 => 0.5 + 0.5 * t.signum() * t.abs().powi(5),
        _ => 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_curve_runs_from_soft_to_full_and_never_falls() {
        for curve in 1..=7 {
            assert_eq!(velocity_curve(curve, 0.0), 0.0, "curve {curve}");
            assert_eq!(velocity_curve(curve, 1.0), 1.0, "curve {curve}");
            let samples: Vec<f64> = (0..=100)
                .map(|i| velocity_curve(curve, f64::from(i) / 100.0))
                .collect();
            assert!(samples.windows(2).all(|w| w[1] >= w[0]), "curve {curve}");
        }
    }

    #[test]
    fn shapes_follow_the_figure() {
        let at = |curve, v| velocity_curve(curve, v);
        assert!(at(3, 0.5) < at(2, 0.5) && at(2, 0.5) < at(1, 0.5)); // bend down, 3 more
        assert!(at(5, 0.5) > at(4, 0.5) && at(4, 0.5) > at(1, 0.5)); // bend up, 5 more
        assert!(at(6, 0.2) < 0.1 && at(6, 0.8) > 0.9); // S: flat ends
        assert!(at(7, 0.2) > 0.3 && at(7, 0.8) < 0.7); // inverted S: flat middle
        assert_eq!(at(0, 0.0), 1.0); // FIXED
    }
}
