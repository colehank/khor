//! Where the machines sit when the network is drawn as a mandala.
//!
//! # Why this is derived here and not by whoever paints it
//!
//! Same rule as [`crate::avatar`], and the same reason: a picture that
//! two faces work out separately is two pictures. mandala settled this
//! for its own map — layout derives in one Rust module and every face
//! only paints — and khor inherits the arrangement rather than the
//! arithmetic.
//!
//! # What khor's map is not, and why it is smaller than mandala's
//!
//! mandala's map has **rings that are hop counts** and **edges that are
//! connections**. khor has neither fact to draw with:
//!
//! - There are no hops. One pairing puts a machine in the whole network
//!   (docs/NET.md), and no machine is another's server, so there is no
//!   distance to be at. **One ring.**
//! - There is no known topology. khor knows *membership*; who can dial
//!   whom right now, over which relay, is not something it is told.
//!   **No edges.**
//!
//! That is not this map being a poorer version of that one — it is
//! mandala's own first rule ("数据没有的不画": an unprobed link is drawn
//! neutral, never as online) applied to a network with a different shape.
//! A line is the most believable mark a diagram can make, and here it
//! would be a claim nobody checked.
//!
//! # The seats say nothing either
//!
//! Evenly spaced, one radius. Neither angle nor distance carries
//! anything, which is exactly why they are uniform: a varying one gets
//! read as data within about a second of somebody noticing it.

use serde::{Deserialize, Serialize};

/// Where one machine sits, as a percentage of the side of the square the
/// map is drawn in.
///
/// A proportion rather than a length, so one derivation serves a phone
/// and a wide window without the library being told how big the picture
/// is — which it cannot know and must not guess.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Seat {
    pub left: f32,
    pub top: f32,
}

/// The middle: where the machine you are sitting at goes.
pub const CENTRE: Seat = Seat { left: 50.0, top: 50.0 };

/// How far out the ring sits, as a percentage of the side.
///
/// Under half, because a seat is placed by its centre and carries a face
/// with a caption under it — at 50 the outermost ones would hang off the
/// edge.
const RING: f32 = 34.0;

/// The seats for `n` machines around the middle, clockwise.
///
/// **Half a step past twelve o'clock, and that half step is the only
/// decision in here.** With a seat *at* twelve, two machines land
/// directly above and below the middle — three faces in a column, which
/// reads as a list rather than as anything surrounding anything, and two
/// machines is the ordinary case for this product. The half step puts
/// those two at left and right instead; three become a triangle and four
/// a diamond. One formula, and the case it rescues is the common one.
pub fn ring(n: usize) -> Vec<Seat> {
    if n == 0 {
        return Vec::new();
    }
    let step = std::f32::consts::TAU / n as f32;
    (0..n)
        .map(|i| {
            let angle = -std::f32::consts::FRAC_PI_2 + step / 2.0 + i as f32 * step;
            Seat {
                left: 50.0 + angle.cos() * RING,
                top: 50.0 + angle.sin() * RING,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn radius(s: &Seat) -> f32 {
        ((s.left - CENTRE.left).powi(2) + (s.top - CENTRE.top).powi(2)).sqrt()
    }

    /// Everyone the same distance out, because the distance says nothing
    /// and a distance that varied would be read as saying something.
    #[test]
    fn every_seat_is_the_same_distance_from_the_middle() {
        for n in 1..=12 {
            for seat in ring(n) {
                assert!(
                    (radius(&seat) - RING).abs() < 0.01,
                    "n={n}: {seat:?} sits at {}, not {RING}",
                    radius(&seat)
                );
            }
        }
    }

    /// **Two machines go left and right, not above and below.**
    ///
    /// This is what the half step is for, and it is asserted as the
    /// concrete case rather than as "the seats are spread out" — the
    /// property version is satisfied by the arrangement this exists to
    /// avoid.
    #[test]
    fn two_machines_sit_beside_the_middle_and_not_in_a_column() {
        let seats = ring(2);
        assert_eq!(seats.len(), 2);
        for seat in &seats {
            assert!(
                (seat.top - CENTRE.top).abs() < 0.01,
                "{seat:?} is above or below the middle, so the three make a column"
            );
        }
        let mut sides: Vec<f32> = seats.iter().map(|s| s.left).collect();
        sides.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!(sides[0] < CENTRE.left && sides[1] > CENTRE.left, "one each side: {sides:?}");
    }

    /// The middle is where the ring averages out — which is what "evenly
    /// spaced around it" means, and the thing a painter can get wrong
    /// while every radius stays equal.
    #[test]
    fn the_middle_is_the_average_of_the_ring() {
        for n in 2..=12 {
            let seats = ring(n);
            let left: f32 = seats.iter().map(|s| s.left).sum::<f32>() / n as f32;
            let top: f32 = seats.iter().map(|s| s.top).sum::<f32>() / n as f32;
            assert!(
                (left - CENTRE.left).abs() < 0.01 && (top - CENTRE.top).abs() < 0.01,
                "n={n}: the ring averages to ({left}, {top}), not the middle"
            );
        }
    }

    /// Nobody but this machine means no ring at all — not one seat
    /// sitting somewhere.
    #[test]
    fn a_network_of_one_has_no_ring() {
        assert!(ring(0).is_empty());
    }

    /// Every seat is inside the square, with room for the face drawn on
    /// it. A radius of 50 would put the outermost ones on the edge.
    #[test]
    fn no_seat_reaches_the_edge_of_the_square() {
        for n in 1..=12 {
            for seat in ring(n) {
                assert!(
                    (5.0..=95.0).contains(&seat.left) && (5.0..=95.0).contains(&seat.top),
                    "n={n}: {seat:?} is off the square"
                );
            }
        }
    }
}
