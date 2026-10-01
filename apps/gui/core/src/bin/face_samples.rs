//! Prints real faces as JSON, for `check-palette`'s 「a mark on a face」
//! section: every factory palette × every way of drawing, a handful of
//! machines each.
//!
//! **The faces are `khor_core::avatar`'s own output, never hand-shaped.**
//! What that section measures is whether a kind mark drawn *on* a face
//! (类型标记「浮现」, the owner 2026-10-02) stays readable against
//! whatever the face is wearing — and a fixture face somebody typed would
//! answer that for a face no machine has. Asked at run time (`cargo run
//! -q -p khor-gui-core --bin face_samples`) rather than written into
//! `src/gen`, so a palette that changes in Rust is measured the next time
//! the gate runs, with nothing to regenerate by hand.
//!
//! Each face carries what a mark on it wears (`face.mark`: the ink, and
//! the veil under it, derived per face — 「按脸换墨，不够才压」). Beside the
//! samples go the core's two inks as hex, so the gate can hold the
//! GUI's `--mark-ink-light` / `--mark-ink-dark` to the colours the rule
//! was derived with.

use khor_core::avatar::{
    AvatarSeed, AvatarStyle, FaceShape, MARK_INK_DARK, MARK_INK_LIGHT, MarkInk, PRESETS, Variant,
    avatar,
};

/// How many machines per palette × variant. Each face picks its ground
/// and strokes from the palette by its id, so more seeds is more of the
/// palette's slots tried as the ground under the mark. Sixteen is where
/// every slot of every factory palette has been the ground at least
/// twice (counted 2026-10-02: six reached only three of the five); the
/// gate refuses a run that saw fewer than five grounds per palette.
const SEEDS: u64 = 16;

/// A fixed, spread-out 64-hex id per index: a seed is a device id, and
/// nothing about these ids means anything except that they differ.
fn seed(i: u64) -> AvatarSeed {
    let mut state = 0x9e37_79b9_7f4a_7c15u64.wrapping_mul(i + 1);
    let mut hex = String::with_capacity(64);
    while hex.len() < 64 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        hex.push_str(&format!("{state:016x}"));
    }
    AvatarSeed::from_id_hex(&hex[..64]).expect("64 hex digits")
}

fn main() {
    let mut out = Vec::new();
    for p in PRESETS.iter() {
        for v in Variant::ALL {
            for i in 0..SEEDS {
                let style = AvatarStyle {
                    palette: p.palette(),
                    variant: v,
                    shape: FaceShape::Circle,
                };
                out.push(serde_json::json!({
                    "palette": p.id,
                    "variant": v.key(),
                    "seed": i,
                    "face": avatar(&seed(i), &style),
                }));
            }
        }
    }
    println!(
        "{}",
        serde_json::json!({
            "inks": { "light": MARK_INK_LIGHT, "dark": MARK_INK_DARK },
            "veil_under": { "light": MarkInk::Light.veil_hex(), "dark": MarkInk::Dark.veil_hex() },
            "samples": out,
        })
    );
}
