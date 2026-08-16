//! A device's face: a geometric figure derived from its identity.
//!
//! **The algorithm is written once.** Not to save code — because writing
//! it twice bends: the same machine ends up looking different on the
//! phone than on the desktop, which is worse than having no avatar at
//! all. A face is worth exactly one thing, "this face *is* that
//! machine", and one misrecognition voids the whole convention.
//!
//! The seed is the **device id** (the public key), never the machine
//! name: renaming keeps the face, the way a person who changes their
//! name keeps theirs. [`AvatarSeed`] makes that a type, not a habit.
//!
//! ## What comes out is geometry, not SVG
//!
//! [`Avatar`] is a ground color, a crop, and a set of painting
//! parameters ([`AvatarPaint`]). Deliberately **not an SVG string**:
//! the phone paints SwiftUI `Path`s, so a string would either make it
//! parse SVG at runtime or force this crate to emit two dialects — the
//! "written twice" problem again. Rust decides what a machine *looks
//! like*; each end only decides what to paint it *with*.
//!
//! ## The palette is an input; the theme is not
//!
//! [`AvatarStyle`] carries five color slots plus a variant and a crop.
//! Theme is absent on purpose: two sets of color values would mean the
//! same machine changes face when you flip light/dark, and the whole
//! point is that it never changes. "A theme switch keeps the face" is
//! the same rule as "a rename keeps the face".
//!
//! The cost is stated where it lands: one set of values across two
//! themes means on one of them some faces sit too close to the card.
//! **That is the edge's job, not the palette's** — see
//! [`Avatar::radius_ratio`] and the `avatar-edge` token in the GUI.
//!
//! ## Why five slots, and why the first stroke is no longer always ink
//!
//! An earlier design had **nine** color combinations in total: ground
//! one of three, second vein one of three, first vein always the ink.
//! With three papers spanning only 10° of hue and a σ=7 blur flattening
//! the geometry, 60 seeds shared a palette every 6–7 machines — **a
//! screen of faces looked the same**, which is the only problem an
//! avatar exists to solve.
//!
//! Three fixes together: 5 slots, hues pulled apart (see [`PRESETS`]),
//! and the first vein no longer fixed.
//!
//! Fixing the first vein to the ink was **deliberate** and the reason
//! held: it is the highest-contrast stroke in the picture, the only one
//! still legible at 16px. So this is not a loosening but a **rule that
//! encodes that reason**: the first vein is drawn from the two slots
//! with the highest contrast against the ground (see [`pick`]). On the
//! old four-color palette any paper as ground makes the ink the highest
//! contrast — **the rule degenerates to the old behavior there**, which
//! makes it a generalization rather than a replacement.
//!
//! ## Three variants
//!
//! [`Variant::Marble`], [`Variant::Bauhaus`], [`Variant::Beam`], all
//! ported from `boringdesigners/boring-avatars`. pixel / sunset / ring
//! are **not implemented**: every added variant is a cross-end
//! consistency exercise (marble alone cost three bugs — blur, overlay,
//! sRGB), so ship these and let someone look before adding more.
//!
//! The three variants do **not** share a parameter set (marble is two
//! blurred, blended veins; bauhaus is three hard-edged shapes; beam is
//! one wrapper plus a face), so on the wire they are a **tagged union**
//! ([`AvatarPaint`]) rather than "several arrays with some left empty".
//! An empty field forces the client into a "which side do I read this
//! time" decision, and that decision is where the two ends diverge.
//!
//! ## Two deliberate differences from boring-avatars
//!
//! The hash, digit extraction, unit extraction, the two paths and the
//! blur radius are **copied verbatim**. Changed:
//!
//! **One: how colors are assigned.** Upstream takes `colors[(hash + i)
//! % N]` — consecutive slots out of the palette. That assumes adjacent
//! slots differ a lot, and it collapses: rotation is `hash % 360` and
//! color is `hash % N`, and both 3 and 5 divide 360, so **color becomes
//! a function of rotation** and two dimensions merge into one. Here the
//! color chain runs on the quotient after dividing out 360 (see
//! [`pick`]), which shares no factor with the "mod 360" path.
//!
//! **Two: marble's four parameters read different segments of the hash
//! (this one was forced by the numbers).** Upstream:
//!
//! ```text
//! translateX = getUnit(n, SIZE/10, 1)   // n % 8
//! translateY = getUnit(n, SIZE/10, 2)   // n % 8   ← same number as X, only the sign differs
//! scale      = 1.2 + getUnit(n, SIZE/20) / 10  // n % 4  ← determined by n % 8
//! rotate     = getUnit(n, 360, 1)       // n % 360 ← 360 = 8×45, which determines n % 8
//! ```
//!
//! Four parameters are really one and a half: |X| and |Y| are always
//! equal (so the vein always runs along the 45° diagonal), scale shares
//! their source, and rotate determines both. Copied literally this
//! measures **4645/10000 distinct faces** against bauhaus's 9911/10000
//! — two orders of magnitude apart, and the "no collapse" test's 98%
//! floor goes red on the spot.
//!
//! This is not "marble is inherently worse": no range was lost, only
//! overlap caused by **those moduli dividing each other**. So the
//! composition and every range are kept and only the four parameters
//! are re-read down a `360 → 8 → 8 → 4` chain (see [`marble_props`]).
//!
//! The criterion is the same as the first difference: **a dimension
//! that is a function of another dimension does not exist.**
//!
//! ## The abs that blows up
//!
//! JS's `hashCode` relies on 32-bit signed overflow and a trailing
//! `Math.abs`. Copying that as `i32::abs` panics: `i32::MIN.abs()`
//! aborts in debug, and it aborts while painting an avatar.
//! **`wrapping_abs` is wrong too** — it hands `i32::MIN` back
//! unchanged, still negative, and `% range` in Rust takes the sign of
//! the dividend, so a negative index reaches the palette. In JS `hash`
//! is a double and `Math.abs(-2147483648)` is `2147483648`, **outside
//! i32**. The faithful port therefore widens to i64 first, see
//! [`hash_code`]. A test pins a device id that really does hit
//! `i32::MIN`.
//!
//! ## Three differences in the beam port
//!
//! **One: beam has its own canvas.** Upstream's `SIZE` for this variant
//! is **36**, not marble/bauhaus's 80. [`Avatar::canvas`] ships with
//! every face precisely so no client hardcodes 80, so giving beam its
//! own [`BEAM_CANVAS`] costs no crop code. Scaling beam's absolute
//! coordinates (the eyes' `14`, `1.5`, `2`; the mouth's `19` and
//! `M13,… a1,0.75…`) into an 80 canvas is the path that actually goes
//! wrong — those numbers are upstream's *in a 36 canvas*.
//!
//! **Two: colors go through [`pick`], not `colors[hash % N]`** — the
//! same collapse as marble's: `range` is always [`SLOTS`] (5), 5
//! divides 360, so fixing `wrapperRotate = hash % 360` fixes `hash % 5`
//! with it.
//!
//! **Three: `wrapperTranslateX`/`Y` and `wrapperScale`/`mouthSpread`
//! each share one `% range`** — another attack of the same collapse.
//! Upstream:
//!
//! ```text
//! preTranslateX = getUnit(n, 10, 1)   // n % 10
//! preTranslateY = getUnit(n, 10, 2)   // n % 10 ← same number as X, only the sign differs
//! wrapperScale  = 1 + getUnit(n, 3) / 10   // n % 3
//! mouthSpread   = getUnit(n, 3)            // n % 3 ← identical to scale's raw value
//! ```
//! and 10, 3 and 5 (eyeSpread's range) all divide 360, so once
//! `wrapperRotate` is fixed every "independent" quantity is fixed with
//! it. Same fix, same ranges, separate remainders (see [`beam_props`]).
//!
//! Not changed: the facial features' color ([`face_ink`], upstream
//! `getContrast` — a YIQ brightness threshold picking pure black or
//! white) is **not** a [`pick`] slot, on purpose. It guarantees the
//! features stay legible on *any* wrapper color, and no palette slot is
//! reserved for guaranteeing contrast. The difference from the
//! "every stroke comes from the palette" convention is documented on
//! [`AvatarPaint::Beam`]'s `face_ink`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::DeviceId;

/// Canvas side for [`Variant::Marble`] and [`Variant::Bauhaus`], equal
/// to boring-avatars' `SIZE` for those two; the geometric constants are
/// written against it. **Not one number shared by all three variants**
/// — beam's canvas is [`BEAM_CANVAS`], because upstream's component
/// already used a different `SIZE`. No client needs to know which
/// variant uses which: [`Avatar::canvas`] ships with every face.
///
/// The figure **overflows the canvas** (marble's paths already reach 86
/// before being scaled 1.2–1.5× and translated; bauhaus's bar spans the
/// canvas and then rotates). That is part of the composition, not a
/// bug: **the client must crop to the canvas bounds**, into the shape
/// [`Avatar::radius_ratio`] names — never a shape the client picks.
pub const CANVAS: f64 = 80.0;

/// Canvas side for [`Variant::Beam`], equal to `avatar-beam.tsx`'s
/// `SIZE`. **Not a typo for [`CANVAS`]** — the eye and mouth
/// coordinates on [`AvatarPaint::Beam`] are defined against 36, and
/// copying those numbers presumes copying the canvas too.
pub const BEAM_CANVAS: f64 = 36.0;

/// Gaussian blur `stdDeviation`, in **canvas units** (not pixels).
/// Upstream's value; marble only.
///
/// **This number must ship from here — never a constant on each end.**
/// Marble's entire texture is this one blur: too much and it is two
/// clouds, too little and it is two sheets of colored paper, and "both
/// ends look the same" is the only reason this code exists. Worse, the
/// two ends' blur APIs do not mean the same thing — SVG's
/// `feGaussianBlur stdDeviation` *is* the Gaussian σ, while SwiftUI's
/// `.blur(radius:)` only claims to be "a radius". Those two numbers may
/// not be assumed equal without measuring.
pub const BLUR: f64 = 7.0;

/// How many slots a palette has. **Five**, one per color picker in the
/// (later) settings screen.
///
/// Why this number: the previous design was "one ink + three papers" =
/// 4, of which one was pinned to the first vein, leaving 3 that varied.
/// Five is boring-avatars' control shape and the point where slots are
/// still distinguishable and still countable — one more and the user
/// has to remember which picker is which.
pub const SLOTS: usize = 5;

/// Which composition paints a face.
///
/// **Exhaustive, no wildcard**: clients select a brush off this tag, so
/// adding a variant here without following on the client renders a
/// blank (or fails to decode). Each end carries a gate test pinning
/// "an unseen tag must not decode", so adding a variant goes red on
/// both ends by construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Variant {
    /// Blurred color blotches (upstream `avatar-marble.tsx`).
    /// **Factory default** (all three defaults: [`AvatarStyle`]).
    #[default]
    Marble,
    /// A bar, a circle and a rule, hard-edged (upstream
    /// `avatar-bauhaus.tsx`).
    Bauhaus,
    /// A face: a rounded wrapper, two eyes and a mouth (upstream
    /// `avatar-beam.tsx`).
    Beam,
}

impl Variant {
    /// Every composition, in the order a chooser offers them.
    pub const ALL: [Variant; 3] = [Variant::Marble, Variant::Bauhaus, Variant::Beam];

    /// The stable key: what a chooser sends back, and what the catalog
    /// looks a word up by.
    ///
    /// **It is the serde tag, and a test pins that** — the same variant
    /// spelled two ways would let a face and the button that chose it
    /// disagree, and neither side would say so.
    pub const fn key(self) -> &'static str {
        match self {
            Variant::Marble => "marble",
            Variant::Bauhaus => "bauhaus",
            Variant::Beam => "beam",
        }
    }

    /// An unknown key is not a variant. Callers refuse by name rather
    /// than quietly painting the default — a typo that silently paints
    /// marble looks exactly like the setting not working.
    pub fn from_key(key: &str) -> Option<Variant> {
        Variant::ALL.into_iter().find(|v| v.key() == key)
    }
}

/// What shape the face is cropped to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FaceShape {
    /// A circle. **Factory default.**
    #[default]
    Circle,
    /// A rounded square (0.3).
    Rounded,
    /// A square.
    Square,
}

impl FaceShape {
    /// Every crop, in the order a chooser offers them.
    pub const ALL: [FaceShape; 3] = [FaceShape::Circle, FaceShape::Rounded, FaceShape::Square];

    /// The stable key, and the serde tag with it — same rule and same
    /// test as [`Variant::key`].
    pub const fn key(self) -> &'static str {
        match self {
            FaceShape::Circle => "circle",
            FaceShape::Rounded => "rounded",
            FaceShape::Square => "square",
        }
    }

    /// An unknown key is not a shape; refused by name, never defaulted.
    pub fn from_key(key: &str) -> Option<FaceShape> {
        FaceShape::ALL.into_iter().find(|s| s.key() == key)
    }

    /// Corner radius as a fraction of the side. **This table exists
    /// once.**
    ///
    /// It shipped as a comment ("change here, change there") while it
    /// had a single value; with three rows and a user-visible choice,
    /// **the same machine cropped differently on two ends** is exactly
    /// what this whole system exists to prevent, so it crosses the wire
    /// with the face and both ends read it.
    ///
    /// A ratio, not pixels: the same face is the same shape at 18px and
    /// 48px. A fixed pixel radius would look like a ball when small and
    /// a brick when large.
    pub fn radius_ratio(self) -> f64 {
        match self {
            Self::Circle => 0.5,
            Self::Rounded => 0.3,
            Self::Square => 0.0,
        }
    }
}

/// Five color slots. **One set, not one per theme** (reason in the
/// module header).
///
/// Every construction path goes through [`Palette::parse`], including
/// deserialization — so every slot downstream is guaranteed to be a
/// lowercase `#rrggbb` and nothing below has to re-check the format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Palette([String; SLOTS]);

impl Default for Palette {
    /// The [`DEFAULT_PRESET`] set.
    ///
    /// **Not `PRESETS[0]`.** [`PRESETS`] is ordered by *provenance*
    /// (ours, then borrowed ones); using that order to also mean "which
    /// one is the default" would force a reshuffle of a user-visible
    /// row of buttons every time the default changes.
    fn default() -> Self {
        // The table must contain it: `the_default_preset_exists` pins it
        Self::from_preset(preset(DEFAULT_PRESET).unwrap_or(&PRESETS[0]))
    }
}

impl Palette {
    /// Validates each slot as `#rrggbb`, normalizing to lowercase.
    ///
    /// **Not a lenient parser**: a slot too few or too many, `#rgb`,
    /// `rgb(…)` — all refused. Accepting them means something unknown
    /// gets painted, and painting a *wrong* color is worse than
    /// painting one stroke fewer: a missing stroke reads as broken, a
    /// wrong color reads as a different machine. Callers that can't
    /// parse fall back to [`Palette::default`] — that is **style, not
    /// identity**, so the fallback can't make anyone misread a machine.
    pub fn parse(colors: &[String]) -> Option<Self> {
        let colors: [String; SLOTS] = <[String; SLOTS]>::try_from(
            colors.iter().map(|c| norm_hex(c)).collect::<Option<Vec<_>>>()?,
        )
        .ok()?;
        Some(Self(colors))
    }

    fn from_preset(p: &Preset) -> Self {
        Self(p.colors.map(|c| c.to_string()))
    }

    pub fn colors(&self) -> &[String; SLOTS] {
        &self.0
    }

    /// Which factory palette these five slots are, when they are one.
    ///
    /// `None` is the honest answer after a slot has been changed by
    /// hand, and a chooser must then mark **no** factory palette rather
    /// than the nearest one: a highlight on "nord" over colors that are
    /// not nord's says the user picked something they did not, and the
    /// next press of that button would look like a no-op.
    pub fn preset_id(&self) -> Option<&'static str> {
        PRESETS
            .iter()
            .find(|p| self.0.iter().zip(p.colors).all(|(a, b)| a == b))
            .map(|p| p.id)
    }
}

/// Deserialization goes through [`Palette::parse`] rather than deriving
/// straight into the array. A style arrives here from another machine's
/// device-table entry, i.e. from a writer that may be a newer version
/// or simply wrong; **one gate, not a second validator that drifts from
/// the first.** A bad slot fails the whole [`AvatarStyle`], which is
/// the intent — half a chosen style and half a default is a face nobody
/// picked, and harder to explain than falling back wholesale.
impl<'de> Deserialize<'de> for Palette {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = Vec::<String>::deserialize(d)?;
        Palette::parse(&raw).ok_or_else(|| {
            serde::de::Error::custom(format!(
                "a palette is exactly {SLOTS} slots of #rrggbb"
            ))
        })
    }
}

/// `#rrggbb` → the same color, lowercase; anything else `None`.
fn norm_hex(s: &str) -> Option<String> {
    let body = s.strip_prefix('#')?;
    if body.len() != 6 || !body.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    Some(format!("#{}", body.to_ascii_lowercase()))
}

/// The factory default palette's `id`. **"Which one is the default" is
/// written exactly here.**
pub const DEFAULT_PRESET: &str = "mandala";

/// One factory palette.
///
/// `id` is what gets stored in a device's self-reported style (changing
/// it discards what users picked). **There is no display name here**:
/// user-visible words live in the catalog (docs/UX.md 文案), and the
/// settings screen that needs those names is a later batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preset {
    pub id: &'static str,
    pub colors: [&'static str; SLOTS],
}

/// The factory palettes. **Ordered by provenance, not by importance** —
/// ours first, then the borrowed ones. **Which one is the default is
/// [`DEFAULT_PRESET`]**, not this order (reason on [`Palette::default`]).
///
/// ## How a palette is chosen (in order)
///
/// 1. **Hues spread apart.** The previous set spanned 10°, which is
///    half of why a screen of faces looked the same. These span
///    84°–208°.
/// 2. **Must not collide with the cold state color** (`--state-blocked`,
///    `#01aac6`, hue 189°). docs/UX.md: it is the app's only cold color
///    and belongs to "waiting for you" alone; a face sits *beside* that
///    mark in the same row, and a same-hued slab next to it kills the
///    contrast the mark relies on. Monokai's blue `#66D9EF` (191°),
///    nord's blues and catppuccin's blue-purples were swapped out for
///    this. `presets_keep_off_the_blocked_hue` checks by **hue**, not
///    by literal value — a different hex at the same hue still goes red.
///    **Whatever a user picks is their own business**; this rule covers
///    only what ships.
/// 3. **In-palette contrast**: the first vein comes from the two slots
///    with the highest contrast against the ground (see [`pick`]), so
///    what decides legibility at small sizes is **pairwise contrast
///    inside the palette**, not this palette against the app's ground.
///
/// ## The numbers behind the third criterion, and one thing to be clear about
///
/// | palette | hue span | vein-on-ground median / min |
/// |---|---|---|
/// | liquid | 200° | **7.35** / 5.17 |
/// | Monokai | 179° | 2.00 / 1.53 |
/// | Catppuccin Mocha | 208° | 1.56 / 1.31 |
/// | Nord Aurora | 141° | 1.82 / 1.40 |
/// | Okabe-Ito | 183° | 2.31 / 1.36 |
/// | warm | 84° | 1.77 / 1.28 |
/// | mandala (default) | 195° | 3.86 / 1.99 |
/// | (the previous set) | 10° | 12.77 / 1.33 |
///
/// **All five borrowed palettes were tuned for code editors**: every
/// slot has to read against *one dark ground*, so their lightnesses sit
/// on one band — used as each other's ground and vein, in-palette
/// contrast is only 1.3–2.3. The old set was the opposite: one near-
/// black ink against three papers gave 12.77, but only 10° of hue.
///
/// So on these palettes **the two properties trade against each other**,
/// and picking a palette is the user's call. What we can do is make the
/// default good at both (mandala: 195° span and 3.86 median, second
/// best in this table) and hand "a tile you can't separate from the
/// card" to the edge (`avatar-edge` in the GUI) rather than to color
/// choice.
#[rustfmt::skip]
pub const PRESETS: [Preset; 7] = [
    // Ours. **The three liquid greens are kept verbatim** (`--lq1/2/3`
    // in the GUI theme), plus two "inks" of *different* hues — the old
    // structure was already "one ink + three papers", this only splits
    // that one ink in two. Span goes 10° → 200° with the "a dark stroke
    // pressed onto paper" composition untouched.
    Preset { id: "liquid", colors: [
        "#f2f6cb", "#dce9a6", "#b7cf75", "#7d2540", "#3f4482",
    ]},
    Preset { id: "monokai", colors: [
        "#f92672", "#a6e22e", "#fd971f", "#e6db74", "#ae81ff",
    ]},
    Preset { id: "mocha", colors: [
        "#f38ba8", "#a6e3a1", "#fab387", "#f9e2af", "#cba6f7",
    ]},
    Preset { id: "nord", colors: [
        "#bf616a", "#d08770", "#ebcb8b", "#a3be8c", "#b48ead",
    ]},
    // The standard palette for scientific figures, designed so the 8%
    // of men with color vision deficiency can still tell slots apart —
    // and telling things apart is the entire value of an avatar.
    //
    // **Its green moved from `#009E73` (164°) to `#009e4f` (150°)**: the
    // original sat 25° from the cold state color while every other slot
    // in these palettes is 73°+ away. After the 14° move it is 39° away
    // and **the worst pairwise distance under color deficiency did not
    // drop** (ΔE 17.2 — the limiting pair was orange/yellow all along),
    // so this change did not spend the palette's whole point. Moving
    // further to 144° would buy 45° of separation but drop the worst ΔE
    // to 14.5; not worth it.
    Preset { id: "okabe", colors: [
        "#e69f00", "#009e4f", "#f0e442", "#d55e00", "#cc79a7",
    ]},
    Preset { id: "warm", colors: [
        "#eb9d8d", "#93865a", "#a8bb9a", "#c5cba6", "#efd8a9",
    ]},
    // The user's own set, values taken verbatim, and the **factory
    // default** (see `DEFAULT_PRESET`).
    //
    // **Its alarm red is the highest of the eight (13.7% by render
    // measurement, 21 of 60 faces carrying it, worst face 69%), and
    // that was a decision made after seeing the number** — `#fa3e3e` is
    // more saturated than the failed-state red (Lab C* 82.6 vs 42.3),
    // so it does **not** match that specific color (ΔE00 11.1) while it
    // does match the *category* "red = something went wrong". See
    // `an_alarm_red_scoreboard_not_a_gate`. **Do not nudge it toward
    // orange or desaturate it**: it is the color he gave.
    //
    // Structurally it is the second best in the table (vein-on-ground
    // median 3.86, behind liquid's 7.35): cream paper, moss, slate and
    // one cinnabar — "paper and ink", not an editor palette.
    Preset { id: "mandala", colors: [
        "#f8f8d6", "#b3c67f", "#5d7e62", "#50595c", "#fa3e3e",
    ]},
];

impl Preset {
    /// These five slots as a [`Palette`]. The table's literals are
    /// already normalized, so this cannot fail the way [`Palette::parse`]
    /// can — which is why picking a factory palette is the one path to a
    /// style that needs no error word.
    pub fn palette(&self) -> Palette {
        Palette::from_preset(self)
    }
}

/// Looks a factory palette up by id.
pub fn preset(id: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|p| p.id == id)
}

/// Everything about a face that is **style** — the half that is not
/// identity.
///
/// The identity half is the [`AvatarSeed`]. They are separate for a
/// reason: **a changed style is the same machine in different clothes;
/// a changed seed is a different machine.** So style is a preference
/// that replicates with the device table, and the seed is derived from
/// the device id and assembled nowhere else.
///
/// ## The three factory defaults
///
/// | | value | written where |
/// |---|---|---|
/// | palette | **mandala** | [`DEFAULT_PRESET`] → [`Palette::default`] |
/// | variant | **marble** | `#[default]` on [`Variant`] |
/// | shape | **circle** | `#[default]` on [`FaceShape`] |
///
/// A device that has never reported a style is painted with these, so
/// they are what a fresh install looks like everywhere at once.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AvatarStyle {
    pub palette: Palette,
    pub variant: Variant,
    pub shape: FaceShape,
}

impl AvatarStyle {
    /// Reads a self-reported style. **Anything that doesn't parse is
    /// `None`, never a half-built style** — see the note on
    /// [`Palette`]'s `Deserialize`.
    pub fn from_json(text: &str) -> Option<AvatarStyle> {
        serde_json::from_str(text).ok()
    }

    /// The JSON a device stores in its own device-table entry.
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|e| e.to_string())
    }
}

/// The seed of a face: **a device's public key in hex, and nothing
/// else.**
///
/// This is a type rather than a `&str` parameter so that a machine
/// *name* cannot reach [`avatar`] at all. It is not a hypothetical
/// mistake — a name is the string that is nearest to hand at every call
/// site, and using it would make every rename a new face, silently,
/// with nothing red anywhere.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvatarSeed(String);

impl AvatarSeed {
    /// The face of this device.
    pub fn of(device: &DeviceId) -> AvatarSeed {
        AvatarSeed(device.hex())
    }

    /// The same identity arriving as text — the device table stores ids
    /// as hex (docs/NET.md). **Refuses anything that is not 64 hex
    /// digits**, which is what keeps a name out: the constructor, not a
    /// convention, is the gate.
    pub fn from_id_hex(hex: &str) -> Option<AvatarSeed> {
        let hex = hex.trim();
        if hex.len() != 64 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        Some(AvatarSeed(hex.to_ascii_lowercase()))
    }
}

/// Which of marble's two paths a vein is.
///
/// **The path data is not in this struct**: it is *how to paint*, not a
/// parameter. Each end writes the two `d` strings into its own brush
/// (the GUI's `VEIN_PATH`), and this enum is the number they agree on.
/// Those strings must be **character-for-character identical** across
/// ends, and each end's comment must point at the other.
///
/// Why not just take them in order (first is A, second is B): then the
/// order is a convention in a comment, and a client that gets it wrong
/// has no signal. As a tag, the client is an exhaustive match and
/// missing an arm is a compile-time or lint failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum Vein {
    /// Upstream's narrow shard (`M32.414 59.35…`).
    Shard,
    /// Upstream's sweeping block (`M22.216 24…`).
    Sweep,
}

/// What one bauhaus primitive is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum Shape {
    /// Axis-aligned rectangle given by `x`/`y`/`w`/`h` (before rotation).
    Rect,
    /// Ellipse inscribed in `x`/`y`/`w`/`h` (always a circle here).
    Circle,
}

/// One of marble's veins.
///
/// **How to paint it (both ends must match; one step out of order is a
/// different face):**
///
/// 1. take [`Vein`]'s path, placed as-is in canvas coordinates;
/// 2. scale by `scale` anchored at the **canvas origin** (0, 0) —
///    **not the canvas center**;
/// 3. rotate `rotate` degrees clockwise about the **canvas center**
///    (`CANVAS/2`, `CANVAS/2`);
/// 4. translate by `(dx, dy)`;
/// 5. Gaussian-blur by [`BLUR`] (**each vein blurred on its own**, not
///    the two of them together);
/// 6. composite with overlay blending when `overlay`, else normally.
///
/// y points down, matching both SVG and SwiftUI.
///
/// - SVG: `transform="translate(dx dy) rotate(rotate 40 40) scale(scale)"`
///   (a transform list applies right-to-left to coordinates, so written
///   out it reads backwards)
/// - SwiftUI: `.scaleEffect(scale, anchor: .topLeading)` — **the anchor
///   must be given explicitly**, because SwiftUI scales about `.center`
///   by default while SVG's `scale()` is about the origin; omitting it
///   is two different faces. Then `.rotationEffect(.degrees(rotate))`
///   (default anchor is the frame center) and `.offset(x: dx, y: dy)`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct AvatarVein {
    pub vein: Vein,
    /// Scale about the canvas origin, 1.2–1.5.
    pub scale: f64,
    /// Degrees clockwise about the canvas center.
    pub rotate: f64,
    pub dx: f64,
    pub dy: f64,
    /// Overlay blending or plain compositing.
    ///
    /// It currently happens to be a function of [`AvatarVein::vein`]
    /// (only [`Vein::Sweep`] uses overlay), but **that is this crate's
    /// business**: the client paints what this boolean says and must
    /// not look the mapping up itself — that table copied onto both
    /// ends is where they start to diverge.
    pub overlay: bool,
    /// `#rrggbb`, from [`AvatarStyle::palette`].
    pub color: String,
}

/// One bauhaus primitive.
///
/// **How to paint it**: place by `x`/`y`/`w`/`h` → rotate `rotate`
/// degrees clockwise about the **canvas center** → translate by
/// `(dx, dy)`. No blur, no blending — bauhaus is hard edges stacked.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct AvatarShape {
    pub shape: Shape,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    /// Degrees clockwise about the canvas center.
    pub rotate: f64,
    pub dx: f64,
    pub dy: f64,
    /// `#rrggbb`, from [`AvatarStyle::palette`].
    pub color: String,
}

/// How to paint this face. **A tagged union, not "two arrays with one
/// left empty".**
///
/// The tag rides the `variant` key and its values are identical to
/// [`Variant`]'s, so a client has exactly one thing to read:
/// `paint.variant`.
///
/// Why `blur` is not hoisted onto [`Avatar`] for both variants to
/// share: bauhaus does not blur. A field that is always 0 makes the
/// client hesitate over whether to check it, and every hesitation is a
/// judgment written twice. **Which variant carries which parameters is
/// settled by this enum.**
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "variant", rename_all = "snake_case")]
pub enum AvatarPaint {
    Marble {
        /// Gaussian σ in canvas units. Always [`BLUR`]; reason there.
        blur: f64,
        /// Painted bottom-up in the given order. Always two here.
        veins: Vec<AvatarVein>,
    },
    Bauhaus {
        /// Painted bottom-up in the given order. Always three here.
        shapes: Vec<AvatarShape>,
    },
    /// A rounded wrapper over the ground color, then a face on top.
    ///
    /// **The canvas is [`BEAM_CANVAS`] (36), not [`CANVAS`].** The
    /// coordinates below (`14`, `19`, the `M13,… a1,0.75…` path) are
    /// defined against it; copying them presumes copying the canvas,
    /// with no rescaling in between.
    ///
    /// **How to paint it, in order:**
    ///
    /// 1. the ground fills [`BEAM_CANVAS`] × [`BEAM_CANVAS`] (the
    ///    ground is [`Avatar::background`], not repeated here);
    /// 2. the wrapper: a rounded rect at `x=0 y=0 w=h=BEAM_CANVAS`,
    ///    corner `wrapper_rx`, filled `wrapper_color` — scaled by
    ///    `wrapper_scale` about the **canvas origin** (the same anchor
    ///    as marble's vein, not the center) → rotated `wrapper_rotate`
    ///    degrees clockwise about the **canvas center** → translated by
    ///    `(wrapper_dx, wrapper_dy)`. No blur, no blending;
    ///    `translate(wrapper_dx wrapper_dy) rotate(wrapper_rotate c c) scale(wrapper_scale)`
    ///    with `c = BEAM_CANVAS / 2` — the same template as
    ///    [`AvatarVein`]'s;
    /// 3. the features as one group, translated by `(face_dx, face_dy)`
    ///    then rotated `face_rotate` degrees clockwise about the
    ///    **canvas center**, containing:
    ///    - when `mouth_open`, the open mouth
    ///      `M15 {19+mouth_spread}c2 1 4 1 6 0` (stroked only:
    ///      `stroke=face_ink`, `fill=none`, round caps); otherwise the
    ///      closed one `M13,{19+mouth_spread} a1,0.75 0 0,0 10,0`
    ///      (filled `face_ink`);
    ///    - two `1.5 × 2` rects with corner `1` as eyes at `y=14`, the
    ///      left at `x = 14 - eye_spread`, the right at
    ///      `x = 20 + eye_spread`, both filled `face_ink`.
    ///
    /// y points down. `14`/`19`/`1.5`/`2`/`1` and the path strings are
    /// **how to paint, not parameters** (verbatim from upstream, same
    /// treatment as [`Vein`]'s two paths): each end writes its own copy
    /// and they must match character for character.
    Beam {
        /// The wrapper's corner radius in canvas units. **Already a
        /// final value**, not a ratio: the client uses it directly and
        /// must not run it through [`FaceShape::radius_ratio`] — that
        /// table crops the whole face, this field rounds the block
        /// *inside* it. Always `BEAM_CANVAS / 2` (circle) or
        /// `BEAM_CANVAS / 6` (rounded square).
        wrapper_rx: f64,
        /// Scale about the canvas origin: 1.0 / 1.1 / 1.2.
        wrapper_scale: f64,
        /// Degrees clockwise about the canvas center.
        wrapper_rotate: f64,
        wrapper_dx: f64,
        wrapper_dy: f64,
        /// The wrapper's color, from [`AvatarStyle::palette`] —
        /// [`pick`]'s second slot, the highest contrast against the
        /// ground.
        wrapper_color: String,
        /// The mouth and eyes: `#000000` or `#ffffff`.
        ///
        /// **Deliberately not from the palette** (unlike marble's and
        /// bauhaus's every-stroke-from-the-palette convention):
        /// `wrapper_color` can be any of the five, the features must
        /// stay legible on **any** of them, and no slot is reserved for
        /// guaranteeing that. [`face_ink`] picks black or white off a
        /// YIQ threshold, independent of what the palette looks like.
        face_ink: String,
        /// Open or closed — two different paths (see above).
        mouth_open: bool,
        /// The `19 + mouth_spread` in the mouth path, canvas units,
        /// always non-negative (0..2).
        mouth_spread: f64,
        /// Eye offset, canvas units, always non-negative (0..4).
        eye_spread: f64,
        /// Degrees the feature group rotates clockwise about the canvas
        /// center.
        face_rotate: f64,
        face_dx: f64,
        face_dy: f64,
    },
}

/// One machine's face.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct Avatar {
    /// Canvas side: [`CANVAS`] for marble/bauhaus, [`BEAM_CANVAS`] for
    /// beam. It ships so no client hardcodes a constant.
    pub canvas: f64,
    /// The ground color filling the canvas, `#rrggbb`.
    pub background: String,
    /// The crop: corner radius ÷ side. See [`FaceShape::radius_ratio`].
    ///
    /// **Cropping identically on both ends went from a comment to a
    /// field.** The figure overflows the canvas by design, so the
    /// client must crop; and a client that crops to a different shape
    /// is the same machine looking different again.
    ///
    /// One more thing this crate cannot do and the client must:
    /// **draw a hairline edge around the face.** These palettes are
    /// tuned for dark editors and contrast only 1.12–1.56 against a
    /// *light* card — without an edge, half the faces dissolve into the
    /// paper in light mode and the row looks like it failed to paint
    /// one. The edge depends on what the face is sitting on, and only
    /// the client knows that (the GUI's `--avatar-edge`).
    pub radius_ratio: f64,
    /// Which composition, with which parameters.
    pub paint: AvatarPaint,
}

/// The port of JS's `hashCode`.
///
/// ```js
/// hash = ((hash<<5)-hash)+character;  // hash*31 + c
/// hash = hash & hash;                 // force back to 32 bits
/// return Math.abs(hash);
/// ```
///
/// Three details that must be copied exactly:
///
/// 1. **Overflow wraps.** JS's `& hash` truncates to int32; that is
///    `wrapping_*` here.
/// 2. **`charCodeAt` yields UTF-16 code units**, not code points. A
///    device id is all ASCII so this can never be reached, but what is
///    copied is the semantics, not the coincidence — `chars()` would
///    diverge from JS on any non-BMP character.
/// 3. **The final absolute value must be taken on i64.** Reason in the
///    module header.
fn hash_code(seed: &str) -> i64 {
    let mut hash: i32 = 0;
    for unit in seed.encode_utf16() {
        hash = hash
            .wrapping_shl(5)
            .wrapping_sub(hash)
            .wrapping_add(i32::from(unit));
    }
    (hash as i64).abs()
}

/// JS `getDigit`: `Math.floor(number / 10**ntn) % 10`, the `ntn`-th
/// decimal digit.
///
/// `number` is always non-negative ([`hash_code`] took the absolute
/// value, then only positive multiplications and divisions follow), so
/// integer division agrees with `Math.floor`.
fn get_digit(number: i64, ntn: u32) -> i64 {
    (number / 10_i64.pow(ntn)) % 10
}

/// JS `getBoolean`: `!(getDigit(number, ntn) % 2)` — is that digit even.
fn get_boolean(number: i64, ntn: u32) -> bool {
    get_digit(number, ntn) % 2 == 0
}

/// JS `getUnit`: `number % range`, signed by the parity of the `index`-th
/// digit.
///
/// `index` is an `Option`, not a `u32`, because the JS line reads
/// `if (index && ...)` — **index 0 short-circuits to false** and is the
/// same branch as "not passed". Neither variant uses 0 in practice, but
/// a port hardcoding `index: u32` would diverge there silently.
fn get_unit(number: i64, range: i64, index: Option<u32>) -> i64 {
    let value = number % range;
    match index {
        Some(i) if i != 0 && get_digit(number, i) % 2 == 0 => -value,
        _ => value,
    }
}

/// sRGB relative luminance (the WCAG 2.x formula). The input is
/// guaranteed to be a [`norm_hex`]'d `#rrggbb`.
fn rel_luminance(hex: &str) -> f64 {
    let v = u32::from_str_radix(&hex[1..], 16).unwrap_or(0);
    let ch = |shift: u32| {
        let c = ((v >> shift) & 0xff) as f64 / 255.0;
        if c <= 0.040_45 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * ch(16) + 0.7152 * ch(8) + 0.0722 * ch(0)
}

/// WCAG contrast ratio, `1.0..=21.0`.
///
/// Contrast rather than "which is darker" picks the first vein, because
/// **the rule has to hold on any palette** — including one whose ground
/// is the darkest slot in the set, where the highest contrast is the
/// lightest slot and the face comes out inverted. That is correct, not
/// a bug.
fn contrast(a: &str, b: &str) -> f64 {
    let (x, y) = (rel_luminance(a), rel_luminance(b));
    let (hi, lo) = if x > y { (x, y) } else { (y, x) };
    (hi + 0.05) / (lo + 0.05)
}

/// Whether beam's features are black or white — the port of upstream's
/// `getContrast`: a **YIQ** threshold,
/// `yiq = (r·299 + g·587 + b·114) / 1000`, `>= 128` means black.
///
/// **Not the WCAG formula in [`rel_luminance`]** — the two numbers come
/// from different places and are deliberately not merged: what is
/// wanted here is upstream's exact black/white verdict, not whichever
/// formula is "more correct". Input is a [`norm_hex`]'d `#rrggbb`.
fn face_ink(hex: &str) -> &'static str {
    let v = u32::from_str_radix(&hex[1..], 16).unwrap_or(0);
    let (r, g, b) = ((v >> 16) & 0xff, (v >> 8) & 0xff, v & 0xff);
    let yiq = (r * 299 + g * 587 + b * 114) / 1000;
    if yiq >= 128 { "#000000" } else { "#ffffff" }
}

/// Which slots a face uses, **in painting order** (index 0 is the
/// ground).
///
/// ## Three criteria
///
/// **One: the ground must not be a function of the rotation.** The
/// split deliberately avoids `h % 5`: rotation is `h % 360` and 5
/// divides 360, which would make the ground a function of rotation and
/// merge two dimensions into one. The high part left after dividing out
/// 360 shares no factor with that path — the same trick as
/// [`marble_props`].
///
/// **Two: the first stroke (marble's shard / bauhaus's bar) comes from
/// the two slots with the highest contrast against the ground.** It is
/// the highest-contrast stroke in the picture and the only one still
/// legible at 16–20px. The previous design guaranteed that by **always
/// taking the ink**, which threw a dimension away. Taking "one of the
/// top two" keeps the same reason and buys a dimension back — and on
/// the old palette, where any paper as ground makes the ink the highest
/// contrast, **the rule degenerates to the old behavior**.
///
/// Why two and not three: measured over six palettes, taking the top
/// two gives a minimum vein-on-ground contrast of 1.28–1.54 and a
/// median of 1.56–7.35; widening to three drops the minimum to
/// 1.14–1.53 and the median a notch. Combinations go 30 → 45 in
/// exchange for every face being slightly muddier. Not worth it.
///
/// **Three: later strokes differ from each other and from the ground.**
/// Two identical slots make a stroke vanish entirely, and that is not a
/// rare event — it is 1 in 5.
///
/// Combinations: marble's three strokes = 5 × 2 × 3 = **30**; bauhaus's
/// four = 5 × 2 × 3 × 2 = **60**. The previous design had 9. **This is
/// near the ceiling for this palette** — five colors in three distinct
/// positions is only 5×4×3 = 60, and criterion two halves the middle
/// position.
fn pick(h: i64, palette: &Palette, count: usize) -> Vec<usize> {
    let colors = palette.colors();
    // The mixed-radix chain on the quotient after 360: each position
    // consumes a segment that does not overlap the others
    let mut q = h / 360;
    let mut take = |radix: i64| {
        let v = (q % radix) as usize;
        q /= radix;
        v
    };

    let ground = take(SLOTS as i64);
    let mut out = vec![ground];

    // Criterion two: the other four sorted by contrast against the
    // ground, take one of the top two. The sort key carries the index
    // so ties order identically on every platform — this output has to
    // be byte-identical across ends.
    let mut rest: Vec<usize> = (0..SLOTS).filter(|&i| i != ground).collect();
    rest.sort_by(|&a, &b| {
        let (ca, cb) = (
            contrast(&colors[a], &colors[ground]),
            contrast(&colors[b], &colors[ground]),
        );
        cb.partial_cmp(&ca)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.cmp(&b))
    });
    let first = rest.remove(take(2));
    out.push(first);

    // Criterion three: the rest are drawn from what is left, one fewer
    // each time
    while out.len() < count {
        let i = take(rest.len() as i64);
        out.push(rest.remove(i));
    }
    out
}

/// Marble's four parameters for one element: `(dx, dy, scale, rotate)`.
///
/// `i` is the element index and `n = h * (i + 1)` follows upstream. The
/// four run down a `360 → 8 → 8 → 4` chain, each consuming a
/// non-overlapping segment: every range and the composition are
/// unchanged, they just stop being functions of one another. Numbers
/// and reasoning in the module header (4645 → 9989).
///
/// The signs still follow [`get_unit`]'s old rule, only reading **their
/// own segment's** decimal digit — upstream shares `digit(n, 1)` for
/// all of them, so dx and rotate always agree in sign, another wasted
/// dimension.
fn marble_props(h: i64, i: i64) -> (f64, f64, f64, f64) {
    let n = h * (i + 1);
    // Rotation eats the lowest segment; the quotient feeds translation
    // and scale in turn
    let rotate = get_unit(n, 360, Some(1));
    let q = n / 360;
    let span = CANVAS as i64 / 10; // 8, upstream's SIZE/10
    let dx = get_unit(q, span, Some(1));
    let q = q / span;
    let dy = get_unit(q, span, Some(2));
    let q = q / span;
    // Scale is always positive: upstream passes no index here. Four
    // steps, 1.2/1.3/1.4/1.5
    let scale = 1.2 + get_unit(q, CANVAS as i64 / 20, None) as f64 / 10.0;
    (dx as f64, dy as f64, scale, rotate as f64)
}

/// Bauhaus's three parameters for one element: `(dx, dy, rotate)`.
/// **Copied verbatim.**
///
/// Element 0 only contributes a color (it is the ground), but the
/// formula still runs for it, because `SIZE/2 - (i + 17)` narrows with
/// `i`: 23 / 22 / 21 / 20.
fn bauhaus_props(h: i64, i: i64) -> (f64, f64, f64) {
    let n = h * (i + 1);
    let range = CANVAS as i64 / 2 - (i + 17);
    (
        get_unit(n, range, Some(1)) as f64,
        get_unit(n, range, Some(2)) as f64,
        get_unit(n, 360, None) as f64,
    )
}

/// Beam's nine numbers and two booleans.
struct BeamProps {
    wrapper_dx: f64,
    wrapper_dy: f64,
    wrapper_rotate: f64,
    wrapper_scale: f64,
    eye_spread: f64,
    mouth_spread: f64,
    mouth_open: bool,
    is_circle: bool,
    face_rotate: f64,
    face_dx: f64,
    face_dy: f64,
}

/// How [`BeamProps`] is drawn: three **individually short** chains,
/// separated by `h × 1/2/3` (the same trick as [`marble_props`]'s
/// `h × (i+1)`, for the same reason).
///
/// **Why not one chain all the way down.** Upstream's `generateData`
/// looks like a dozen independent `hash % range`, and turning them into
/// non-overlapping chain reads is the fix — but writing that as one
/// chain peeling off 12 segments (360→10→2→10→2→3→5→3→2→2→10→2) gets
/// the arithmetic wrong in the place that matters. The chain's total
/// span (432,000 × 1200 ≈ 5×10^8) looks smaller than `h`'s range
/// (`hash_code` yields `0..=2^31`, about 2.1×10^9), but **every step
/// divides the quotient**, so by the twelfth segment the quotient is
/// down to single digits and the last few parameters (notably the
/// features' independent jitter) degenerate to one value for nearly
/// every `h` — the same "a dimension becomes a function of another"
/// failure, just at the tail instead of through a shared range.
///
/// Three short chains each keep headroom: chain A (translation,
/// rotation, scale — span 432,000), chain B (feature shape and
/// orientation — span 1,200), chain C (the features' independent
/// jitter — span 224). Each finishes with thousands to millions of
/// quotient left, never collapsing to single digits.
fn beam_props(h: i64) -> BeamProps {
    // Chain A: the wrapper's translation / rotation / scale. Magnitude
    // and sign take separate segments instead of sharing one `% 10` —
    // upstream's `preTranslateX`/`preTranslateY` are both
    // `getUnit(n, 10, …)` differing only in sign, identical in magnitude
    // (module header, beam difference three).
    let mut qa = h;
    let mut take_a = |radix: i64| {
        let v = qa % radix;
        qa /= radix;
        v
    };
    let wrapper_rotate = take_a(360) as f64;
    let tx_mag = take_a(10);
    let pre_tx = if take_a(2) == 0 { -tx_mag } else { tx_mag };
    let ty_mag = take_a(10);
    let pre_ty = if take_a(2) == 0 { -ty_mag } else { ty_mag };
    let wrapper_scale = 1.0 + take_a(3) as f64 / 10.0;

    // Upstream: magnitudes below 5 get pushed outward by `SIZE/9` (= 4),
    // otherwise the translation piles up around the center and the
    // wrapper's offset is invisible at small sizes
    let bump = BEAM_CANVAS as i64 / 9;
    let wrapper_dx = (if pre_tx < 5 { pre_tx + bump } else { pre_tx }) as f64;
    let wrapper_dy = (if pre_ty < 5 { pre_ty + bump } else { pre_ty }) as f64;

    // Chain B: eye spread, mouth open/spread, round or square wrapper,
    // the features' orientation. Times 2 to clear chain A
    let mut qb = h * 2;
    let mut take_b = |radix: i64| {
        let v = qb % radix;
        qb /= radix;
        v
    };
    let eye_spread = take_b(5) as f64;
    let mouth_spread = take_b(3) as f64;
    let mouth_open = take_b(2) == 0;
    let is_circle = take_b(2) == 0;
    let face_rotate_mag = take_b(10);
    let face_rotate = if take_b(2) == 0 {
        -face_rotate_mag as f64
    } else {
        face_rotate_mag as f64
    };

    // Chain C: the small independent offset the features take when the
    // wrapper has not moved far (upstream's ternary else branch, ranges
    // 8 and 7). Times 3 to clear both others
    let mut qc = h * 3;
    let mut take_c = |radix: i64| {
        let v = qc % radix;
        qc /= radix;
        v
    };
    let jitter_x_mag = take_c(8);
    let jitter_x = if take_c(2) == 0 { -jitter_x_mag } else { jitter_x_mag };
    let jitter_y_mag = take_c(7);
    let jitter_y = if take_c(2) == 0 { -jitter_y_mag } else { jitter_y_mag };

    // Upstream: when the wrapper moved far (> `SIZE/6`) the features
    // follow it halfway, otherwise they jitter on their own (chain C)
    let sixth = BEAM_CANVAS / 6.0;
    let face_dx = if wrapper_dx > sixth { wrapper_dx / 2.0 } else { jitter_x as f64 };
    let face_dy = if wrapper_dy > sixth { wrapper_dy / 2.0 } else { jitter_y as f64 };

    BeamProps {
        wrapper_dx,
        wrapper_dy,
        wrapper_rotate,
        wrapper_scale,
        eye_spread,
        mouth_spread,
        mouth_open,
        is_circle,
        face_rotate,
        face_dx,
        face_dy,
    }
}

/// Derives a machine's face.
///
/// Pure: the same `seed` and `style` always give the same output, with
/// no randomness, clock or process state involved. **That is the
/// mechanism behind "it looks the same on the phone and the desktop"**
/// — it does not depend on anyone remembering.
///
/// **The geometry is independent of the palette and of the variant**: a
/// different palette recolors without reshaping, and a different
/// variant recomposes while translation and rotation still come from
/// the same seed. Tests pin this.
pub fn avatar(seed: &AvatarSeed, style: &AvatarStyle) -> Avatar {
    let h = hash_code(&seed.0);
    let colors = style.palette.colors();
    let radius_ratio = style.shape.radius_ratio();

    match style.variant {
        Variant::Marble => {
            // ── geometry, line by line against avatar-marble.tsx ──
            // Upstream takes only element 0's color (it is the ground)
            // and discards its geometry; so it is not computed here.
            let (shard_dx, shard_dy, shard_scale, shard_rot) = marble_props(h, 1);
            let (sweep_dx, sweep_dy, sweep_scale, sweep_rot) = marble_props(h, 2);
            let c = pick(h, &style.palette, 3);
            Avatar {
                canvas: CANVAS,
                background: colors[c[0]].clone(),
                radius_ratio,
                paint: AvatarPaint::Marble {
                    blur: BLUR,
                    veins: vec![
                        // The shard: one of the two highest-contrast
                        // slots, composited normally. It is the
                        // highest-contrast stroke in the picture and
                        // the only one still legible small.
                        AvatarVein {
                            vein: Vein::Shard,
                            scale: shard_scale,
                            rotate: shard_rot,
                            dx: shard_dx,
                            dy: shard_dy,
                            overlay: false,
                            color: colors[c[1]].clone(),
                        },
                        // The sweep: the third slot, overlay-blended.
                        // Over the dark half it barely moves (overlay
                        // multiplies there), over the light half it
                        // lifts — that is where marble's "two
                        // brightnesses in one stone" comes from. **It
                        // is second-tier information, not the main
                        // stroke**: it should only resolve above 48px.
                        AvatarVein {
                            vein: Vein::Sweep,
                            scale: sweep_scale,
                            rotate: sweep_rot,
                            dx: sweep_dx,
                            dy: sweep_dy,
                            overlay: true,
                            color: colors[c[2]].clone(),
                        },
                    ],
                },
            }
        }
        Variant::Bauhaus => {
            // ── geometry, line by line against avatar-bauhaus.tsx ──
            let (bar_dx, bar_dy, bar_rot) = bauhaus_props(h, 1);
            let (dot_dx, dot_dy, _) = bauhaus_props(h, 2);
            let (rule_dx, rule_dy, rule_rot) = bauhaus_props(h, 3);
            // Whether the bar is a thin rule or a whole block.
            // **Upstream really does use `h`, not `h * (i+1)`**, so all
            // four elements share the verdict and only the bar uses it.
            let is_square = get_boolean(h, 2);
            let c = pick(h, &style.palette, 4);
            Avatar {
                canvas: CANVAS,
                background: colors[c[0]].clone(),
                radius_ratio,
                paint: AvatarPaint::Bauhaus {
                    shapes: vec![
                        // The bar: a rect spanning the canvas, a whole
                        // block when `is_square`. The highest-contrast
                        // stroke — at 16px roughly all that survives is
                        // its angle.
                        AvatarShape {
                            shape: Shape::Rect,
                            x: (CANVAS - 60.0) / 2.0,
                            y: (CANVAS - 20.0) / 2.0,
                            w: CANVAS,
                            h: if is_square { CANVAS } else { CANVAS / 8.0 },
                            rotate: bar_rot,
                            dx: bar_dx,
                            dy: bar_dy,
                            color: colors[c[1]].clone(),
                        },
                        // The circle: the third slot. Upstream does not
                        // rotate it; neither do we.
                        AvatarShape {
                            shape: Shape::Circle,
                            x: CANVAS / 2.0 - CANVAS / 5.0,
                            y: CANVAS / 2.0 - CANVAS / 5.0,
                            w: CANVAS / 2.5,
                            h: CANVAS / 2.5,
                            rotate: 0.0,
                            dx: dot_dx,
                            dy: dot_dy,
                            color: colors[c[2]].clone(),
                        },
                        // The rule: upstream's `<line stroke-width=2>`,
                        // expressed as a 2-tall rect. At 16px it is
                        // 0.4px and effectively invisible; it is for
                        // the 48px-and-up sizes.
                        AvatarShape {
                            shape: Shape::Rect,
                            x: 0.0,
                            y: CANVAS / 2.0 - 1.0,
                            w: CANVAS,
                            h: 2.0,
                            rotate: rule_rot,
                            dx: rule_dx,
                            dy: rule_dy,
                            color: colors[c[3]].clone(),
                        },
                    ],
                },
            }
        }
        Variant::Beam => {
            // ── geometry, line by line against avatar-beam.tsx ──
            let p = beam_props(h);
            // Criterion two again, only two slots here: the ground and
            // the highest contrast against it (module header, beam
            // difference two)
            let c = pick(h, &style.palette, 2);
            let wrapper_color = colors[c[1]].clone();
            let face_ink = face_ink(&wrapper_color).to_string();
            Avatar {
                canvas: BEAM_CANVAS,
                background: colors[c[0]].clone(),
                radius_ratio,
                paint: AvatarPaint::Beam {
                    wrapper_rx: if p.is_circle {
                        BEAM_CANVAS / 2.0
                    } else {
                        BEAM_CANVAS / 6.0
                    },
                    wrapper_scale: p.wrapper_scale,
                    wrapper_rotate: p.wrapper_rotate,
                    wrapper_dx: p.wrapper_dx,
                    wrapper_dy: p.wrapper_dy,
                    wrapper_color,
                    face_ink,
                    mouth_open: p.mouth_open,
                    mouth_spread: p.mouth_spread,
                    eye_spread: p.eye_spread,
                    face_rotate: p.face_rotate,
                    face_dx: p.face_dx,
                    face_dy: p.face_dy,
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

    /// **Must not appear in a factory palette**, with its reason. Not a
    /// comment — a test checks it (`presets_keep_off_the_blocked_hue`).
    ///
    /// The cold state color (`--state-blocked`, `#01aac6` dark /
    /// `#016f82` light): docs/UX.md makes it the app's only cold color,
    /// belonging to "waiting for you" alone. Using it in a face is not
    /// just semantic dilution — a face and the waiting mark appear
    /// **side by side in one row**, and a same-hued slab beside the mark
    /// removes its contrast.
    ///
    /// **Checked by hue, not by literal value.** The literal-value
    /// version only stops copy-paste, not collision.
    const BLOCKED_CYAN: &str = "#01aac6";
    /// How far a factory slot must stay from that cyan. Measured
    /// minimum over six palettes is 39° (Okabe-Ito's green), the rest
    /// are 47°+. The floor sits at 35°: it stops "someone put another
    /// cyan in", not jitter.
    const BLOCKED_KEEPOUT: f64 = 35.0;

    /// **The chroma floor for hue judgments.** Slots below this C* do
    /// not take part in the hue check above.
    ///
    /// This exists because of a **false positive**: the user's palette
    /// carries a slate `#50595c` whose HSL hue computes to 195°, only
    /// 6° from the cyan, and the original assertion went red on it.
    /// **It cannot possibly be mistaken for cyan**: its Lab chroma C*
    /// is 4.1 (the real cyan `#01aac6` is 36.1) and its ΔE00 to that
    /// cyan is 32.4 — **further than `#5d7e62` (28.1), which passes**.
    ///
    /// The criterion: **hue is meaningless on a near-neutral.** A
    /// gray's hue is decided by its residual cast and can jump from
    /// 195° to 15° on one quantization step; judging "does it look like
    /// a saturated cyan" from that is measuring a quantity that isn't
    /// there.
    ///
    /// The number 15: the least chromatic slot in service is warm's
    /// `#a8bb9a` at 19.3, so this floor **excludes no slot in service**
    /// — `at_least_judged` below pins that, so nobody can raise the
    /// floor until the whole test is hollow.
    const CHROMA_FLOOR: f64 = 15.0;

    /// A device id that really does drive `hashCode` to `i32::MIN`.
    ///
    /// Not invented: with the first 55 characters fixed and the last 9
    /// free, `hash = hash*31 + c (mod 2^32)` was solved by meet-in-the-
    /// middle. 16^9 combinations hitting one residue class mod 2^32
    /// expects 16 solutions; this prefix has 13. "Happens to collide"
    /// is not a zero-probability event, and where it panics is while
    /// painting an avatar.
    const SEED_I32_MIN: &str =
        "9f2c7a41be80d35e6417c0a9fd2b6e83c15470ab29de6f01b4837c50e5af670f";

    /// A seed straight from a string. **Test-only on purpose**: the
    /// public constructors take a device identity ([`AvatarSeed::of`])
    /// or 64 hex digits, and the golden values below were computed by
    /// hand against short upstream-style seeds. Production code must
    /// not have this door — see `a_machine_name_cannot_seed_a_face`.
    fn seed_of(s: &str) -> AvatarSeed {
        AvatarSeed(s.to_owned())
    }

    /// The style the golden tests use: **liquid / marble / rounded**,
    /// hardcoded, **not following the defaults**.
    ///
    /// The golden numbers pin the **algorithm** (hash → digits → color
    /// split, a chain that has not changed); the defaults answer "what
    /// does a fresh install look like". Tying them together means every
    /// default change rewrites a set of hand-computed numbers, and
    /// **replacing golden values also replaces the cross-version and
    /// cross-end reference point** — the one whose failure mode is
    /// "the two ends look different", which is the thing this system
    /// exists to prevent.
    ///
    /// The defaults have their own test
    /// ([`the_three_defaults_are_the_ones_that_ship`]).
    fn golden_style() -> AvatarStyle {
        AvatarStyle {
            palette: Palette::from_preset(preset("liquid").unwrap()),
            variant: Variant::Marble,
            shape: FaceShape::Rounded,
        }
    }

    /// Every palette × variant × shape, for tests that must sweep.
    fn all_styles() -> Vec<AvatarStyle> {
        let mut out = vec![];
        for p in PRESETS.iter() {
            for variant in [Variant::Marble, Variant::Bauhaus, Variant::Beam] {
                for shape in [FaceShape::Circle, FaceShape::Rounded, FaceShape::Square] {
                    out.push(AvatarStyle {
                        palette: Palette::from_preset(p),
                        variant,
                        shape,
                    });
                }
            }
        }
        out
    }

    /// Which slots a face used, in painting order.
    ///
    /// **Beam counts only `wrapper_color`, never `face_ink`**: the
    /// latter deliberately does not come from the palette, and folding
    /// it into "which palette slots were used" would make
    /// `palette_combinations` compute a fake number.
    fn used_colors(a: &Avatar) -> Vec<String> {
        let mut v = vec![a.background.clone()];
        match &a.paint {
            AvatarPaint::Marble { veins, .. } => {
                v.extend(veins.iter().map(|x| x.color.clone()))
            }
            AvatarPaint::Bauhaus { shapes } => {
                v.extend(shapes.iter().map(|x| x.color.clone()))
            }
            AvatarPaint::Beam { wrapper_color, .. } => v.push(wrapper_color.clone()),
        }
        v
    }

    fn seeds(n: usize, mut state: u64) -> Vec<AvatarSeed> {
        (0..n)
            .map(|_| {
                seed_of(
                    &(0..4)
                        .map(|_| {
                            state ^= state << 13;
                            state ^= state >> 7;
                            state ^= state << 17;
                            format!("{state:016x}")
                        })
                        .collect::<String>(),
                )
            })
            .collect()
    }

    fn hue(hex: &str) -> f64 {
        let v = u32::from_str_radix(&hex[1..], 16).unwrap();
        let (r, g, b) = (
            ((v >> 16) & 0xff) as f64 / 255.0,
            ((v >> 8) & 0xff) as f64 / 255.0,
            (v & 0xff) as f64 / 255.0,
        );
        let (mx, mn) = (r.max(g).max(b), r.min(g).min(b));
        let d = mx - mn;
        if d == 0.0 {
            return f64::NAN;
        }
        let deg = if mx == r {
            60.0 * (((g - b) / d) % 6.0)
        } else if mx == g {
            60.0 * ((b - r) / d + 2.0)
        } else {
            60.0 * ((r - g) / d + 4.0)
        };
        (deg + 360.0) % 360.0
    }

    /// Lab chroma C*. **Use this to decide whether a color has a hue at
    /// all**, not HSL saturation — HSL's s runs high on dark colors
    /// (`#50595c` is 7% while the equally gray `#101820` is 33%).
    fn chroma(hex: &str) -> f64 {
        let (a, b) = lab_ab(hex);
        a.hypot(b)
    }

    /// Lab `a*` / `b*`. Chroma is their magnitude, hue their argument;
    /// two tests use one half each.
    fn lab_ab(hex: &str) -> (f64, f64) {
        let v = u32::from_str_radix(&hex[1..], 16).unwrap();
        let f = |shift: u32| {
            let c = ((v >> shift) & 0xff) as f64 / 255.0;
            if c <= 0.040_45 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        let (r, g, b) = (f(16), f(8), f(0));
        // XYZ → Lab under the D65 white point
        let x = (0.4124 * r + 0.3576 * g + 0.1805 * b) / 0.950_47;
        let y = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        let z = (0.0193 * r + 0.1192 * g + 0.9505 * b) / 1.088_83;
        let k = |t: f64| {
            if t > 216.0 / 24389.0 {
                t.cbrt()
            } else {
                (841.0 / 108.0) * t + 4.0 / 29.0
            }
        };
        let (fx, fy, fz) = (k(x), k(y), k(z));
        (500.0 * (fx - fy), 200.0 * (fy - fz))
    }

    fn hue_gap(a: f64, b: f64) -> f64 {
        let d = (a - b).abs() % 360.0;
        d.min(360.0 - d)
    }

    /// **A machine name cannot seed a face.**
    ///
    /// The rename-keeps-the-face rule, enforced by the constructor
    /// rather than by discipline: names in this network are pathable
    /// channel names, and none of them is 64 hex digits. What this
    /// leaves is the compile-time half — [`avatar`] takes an
    /// [`AvatarSeed`], so no `&str` reaches it at all, and the only
    /// public ways to build one both take a device identity.
    #[test]
    fn a_machine_name_cannot_seed_a_face() {
        for name in ["turing", "alpha", "mac-mini", "", "beta-2"] {
            assert!(
                AvatarSeed::from_id_hex(name).is_none(),
                "{name:?} is a machine name, it must not be a seed"
            );
        }
        // The two doors agree: the bytes and the hex the device table
        // stores are the same identity, so a face derived from a
        // `DeviceInfo` equals one derived from a `DeviceId`.
        let id = DeviceId([0xab; 32]);
        let hex = "ab".repeat(32);
        assert_eq!(AvatarSeed::of(&id), AvatarSeed::from_id_hex(&hex).unwrap());
        // Uppercase hex is the same machine, not a second one
        assert_eq!(
            AvatarSeed::from_id_hex(&hex.to_uppercase()).unwrap(),
            AvatarSeed::from_id_hex(&hex).unwrap()
        );
        // Length is the gate: 63 and 65 hex digits are not ids
        assert!(AvatarSeed::from_id_hex(&"a".repeat(63)).is_none());
        assert!(AvatarSeed::from_id_hex(&"a".repeat(65)).is_none());
    }

    /// Criterion one: that `abs` does not blow up, and gives the value
    /// JS's double gives.
    ///
    /// Only meaningful in debug (overflow checks on), which is what
    /// `cargo test` runs. Copied as `i32::abs` this panics; written as
    /// `wrapping_abs` the asserted value is a negative `i32::MIN`, and
    /// the `% range` below it turns negative and indexes the palette.
    #[test]
    fn i32_min_seed_does_not_panic() {
        assert_eq!(hash_code(SEED_I32_MIN), 2_147_483_648);

        // Actually paint one: a negative leaking through lands on a
        // palette index and a few `%`s — the first panics, the second
        // quietly yields negative translations. **Every variant**, since
        // `pick`'s chain is shared and bauhaus takes one slot more.
        let seed = AvatarSeed::from_id_hex(SEED_I32_MIN).expect("that is a real device id");
        for style in all_styles() {
            let a = avatar(&seed, &style);
            assert!(a.background.starts_with('#'));
            assert!(!used_colors(&a).is_empty());
        }
        // And pin the sign of `% range`: JS's `%` and Rust's both follow
        // the dividend, and the dividend here is always non-negative —
        // the moment the hash leaks a negative, this goes red.
        assert!(hash_code(SEED_I32_MIN) >= 0);
    }

    /// Criterion two: determinism, and **a style change is not a face
    /// change**.
    ///
    /// This is the mechanism behind "the same machine looks the same on
    /// both ends": a different palette recolors, and the geometry
    /// matches field for field.
    #[test]
    fn same_seed_same_face() {
        let seed = seed_of("b4f1c2d3e4a5968778695a4b3c2d1e0fb4f1c2d3e4a5968778695a4b3c2d1e0f");
        for style in all_styles() {
            assert_eq!(avatar(&seed, &style), avatar(&seed, &style));
        }

        // A different palette: colors change, not one geometry field
        let liquid = golden_style();
        let mono = AvatarStyle {
            palette: Palette::from_preset(preset("monokai").unwrap()),
            ..golden_style()
        };
        let (a, b) = (avatar(&seed, &liquid), avatar(&seed, &mono));
        assert_ne!(a.background, b.background);
        assert_eq!(a.canvas, b.canvas);
        assert_eq!(a.radius_ratio, b.radius_ratio);
        let (
            AvatarPaint::Marble { veins: va, blur: ba },
            AvatarPaint::Marble { veins: vb, blur: bb },
        ) = (&a.paint, &b.paint)
        else {
            panic!("the golden style's variant is marble");
        };
        assert_eq!(ba, bb);
        for (x, y) in va.iter().zip(vb.iter()) {
            assert_eq!(
                (x.vein, x.scale, x.rotate, x.dx, x.dy, x.overlay),
                (y.vein, y.scale, y.rotate, y.dx, y.dy, y.overlay)
            );
            assert_ne!(x.color, y.color);
        }

        // A different shape: only radius_ratio moves, not even a color
        for shape in [FaceShape::Circle, FaceShape::Square] {
            let s = AvatarStyle { shape, ..golden_style() };
            let c = avatar(&seed, &s);
            assert_eq!(c.radius_ratio, shape.radius_ratio());
            assert_eq!(c.background, a.background);
            assert_eq!(c.paint, a.paint);
        }
    }

    /// Criterion three: different seeds give different faces, and **how
    /// often faces collide**.
    ///
    /// Ten thousand random device ids, counting pairs sharing an
    /// identical geometry-plus-palette. The ids come from a fixed-seed
    /// xorshift, so the number is the same on every run — a statistical
    /// test that goes red occasionally is no test.
    #[test]
    fn different_seeds_different_faces() {
        let style = golden_style();
        assert_ne!(avatar(&seed_of("aaaa"), &style), avatar(&seed_of("bbbb"), &style));

        const N: usize = 10_000;
        for variant in [Variant::Marble, Variant::Bauhaus, Variant::Beam] {
            let style = AvatarStyle { variant, ..golden_style() };
            let mut seen: HashMap<String, usize> = HashMap::new();
            for seed in seeds(N, 0x9e37_79b9_7f4a_7c15) {
                *seen.entry(format!("{:?}", avatar(&seed, &style))).or_default() += 1;
            }
            let pairs: usize = seen.values().map(|&k| k * (k - 1) / 2).sum();
            let distinct = seen.len();

            // Measured: marble 9989 faces / 11 colliding pairs (the
            // birthday formula puts the effective space near 4.5×10^6),
            // bauhaus 9911 / 89 (about 5.6×10^5).
            //
            // **A literal copy of upstream's marble goes red here**:
            // 4645 faces / 10110 pairs, an effective space near 5×10^3
            // — its four parameters are functions of each other (module
            // header, difference two). The 98% floor is set for exactly
            // that **collapse**: when one dimension becomes a function
            // of another this number loses orders of magnitude, not a
            // few dozen. The number itself is deterministic.
            //
            // Beam sits at the same floor: 9928 faces / 73 pairs —
            // **the literal "one chain all the way down" also goes red**
            // (see `beam_props`), passing once split into three.
            assert!(
                distinct >= N * 98 / 100,
                "{variant:?} collides too much: {distinct} faces / {N} ids, {pairs} pairs"
            );
            println!("{variant:?}: {N} ids → {distinct} faces, {pairs} colliding pairs");
        }
    }

    /// **Palette combinations**: the headline reason this design exists.
    ///
    /// The previous one had 9 in total (ground one of three × second
    /// vein one of three, first vein fixed), so 60 seeds shared a
    /// palette every 6–7 machines and a screen of faces looked alike.
    ///
    /// Now: marble 5 × 2 × 3 = 30, bauhaus 5 × 2 × 3 × 2 = 60. **That
    /// is against the ceiling**: five colors in three distinct
    /// positions is 60 in total, and criterion two halves the middle
    /// position.
    ///
    /// Beam uses two (ground + wrapper; the features' black/white is
    /// not a palette slot), so 5 × 2 = **10** — fewer than the others,
    /// which is not a collapse, just one stroke fewer.
    #[test]
    fn palette_combinations() {
        for (variant, want) in
            [(Variant::Marble, 30), (Variant::Bauhaus, 60), (Variant::Beam, 10)]
        {
            for p in PRESETS.iter() {
                let style = AvatarStyle {
                    palette: Palette::from_preset(p),
                    variant,
                    shape: FaceShape::Rounded,
                };
                let combos: HashSet<Vec<String>> = seeds(20_000, 0x2545_f491_4f6c_dd1d)
                    .iter()
                    .map(|s| used_colors(&avatar(s, &style)))
                    .collect();
                assert_eq!(
                    combos.len(),
                    want,
                    "{} / {variant:?} has {} palette combinations, not {want}",
                    p.id,
                    combos.len()
                );
            }
        }
    }

    /// **How many palettes repeat when 60 machines sit on one screen.**
    ///
    /// The previous design measured 9 (60 seeds crammed into 9
    /// palettes). The floor here is 20 — it does not measure "is it
    /// pretty", it measures "has it collapsed back".
    #[test]
    fn sixty_machines_on_one_screen() {
        for variant in [Variant::Marble, Variant::Bauhaus] {
            for p in PRESETS.iter() {
                let style = AvatarStyle {
                    palette: Palette::from_preset(p),
                    variant,
                    shape: FaceShape::Rounded,
                };
                let combos: HashSet<Vec<String>> = seeds(60, 0x1234_5678_9abc_def0)
                    .iter()
                    .map(|s| used_colors(&avatar(s, &style)))
                    .collect();
                println!("60 seeds / {} / {variant:?} → {} palettes", p.id, combos.len());
                assert!(
                    combos.len() >= 20,
                    "{} / {variant:?} only has {} palettes",
                    p.id,
                    combos.len()
                );
            }
        }
    }

    /// The same thing for beam, **with a different floor**.
    ///
    /// Beam has 10 palette combinations in total
    /// (`palette_combinations` pins it), fewer than the others — not a
    /// collapse, it just paints two strokes. Sixty random seeds falling
    /// into 10 buckets will almost surely hit all ten, so the floor is
    /// 9 to leave a little slack; what it still catches is a collapse
    /// to single digits.
    #[test]
    fn sixty_machines_on_one_screen_beam() {
        for p in PRESETS.iter() {
            let style = AvatarStyle {
                palette: Palette::from_preset(p),
                variant: Variant::Beam,
                shape: FaceShape::Rounded,
            };
            let combos: HashSet<Vec<String>> = seeds(60, 0x1234_5678_9abc_def0)
                .iter()
                .map(|s| used_colors(&avatar(s, &style)))
                .collect();
            println!("60 seeds / {} / Beam → {} palettes (of 10)", p.id, combos.len());
            assert!(
                combos.len() >= 9,
                "{} / Beam only has {} palettes (of 10)",
                p.id,
                combos.len()
            );
        }
    }

    /// **The first stroke is always one of the two highest-contrast
    /// slots** ([`pick`]'s criterion two).
    ///
    /// It checks a **rank**, not "is it dark enough" — the rank is what
    /// the rule says, and it holds on any palette, including one whose
    /// ground is its own darkest slot.
    #[test]
    fn the_first_stroke_is_the_highest_contrast_one() {
        for style in all_styles() {
            let colors = style.palette.colors();
            for seed in seeds(300, 0x0bad_c0de_dead_beef) {
                let a = avatar(&seed, &style);
                let used = used_colors(&a);
                let mut rank: Vec<&String> =
                    colors.iter().filter(|c| **c != a.background).collect();
                rank.sort_by(|x, y| {
                    contrast(y, &a.background)
                        .partial_cmp(&contrast(x, &a.background))
                        .unwrap()
                });
                assert!(
                    used[1] == *rank[0] || used[1] == *rank[1],
                    "the first stroke {} is not among the two highest-contrast {:?} (ground {})",
                    used[1],
                    &rank[..2],
                    a.background
                );
                // Criterion three: the strokes all differ
                let uniq: HashSet<&String> = used.iter().collect();
                assert_eq!(uniq.len(), used.len(), "two strokes share a color: {used:?}");
            }
        }
    }

    /// On the old palette the new rule **degenerates to the old
    /// behavior**: the first stroke is always the ink.
    ///
    /// This is the evidence that changing the rule did not change the
    /// output. It going red means [`pick`]'s criterion two was altered,
    /// which means every machine's face from the previous design moved.
    #[test]
    fn on_the_old_palette_the_rule_degenerates_to_ink() {
        const INK: &str = "#1d2107";
        let palette = Palette::parse(&[
            "#f2f6cb".into(),
            "#dce9a6".into(),
            "#b7cf75".into(),
            INK.into(),
            // The old set had four slots; a second copy of the ink fills
            // the fifth. It does not affect this criterion, which asks
            // whether the ink is among the two highest-contrast slots
            INK.into(),
        ])
        .unwrap();
        for variant in [Variant::Marble, Variant::Bauhaus, Variant::Beam] {
            let style = AvatarStyle {
                palette: palette.clone(),
                variant,
                shape: FaceShape::Rounded,
            };
            for seed in seeds(500, 0xfeed_face_cafe_0001) {
                let a = avatar(&seed, &style);
                if a.background == INK {
                    continue; // ink as ground makes paper the highest contrast, correctly
                }
                assert_eq!(
                    used_colors(&a)[1],
                    INK,
                    "the first stroke on the old palette is no longer ink: {seed:?}"
                );
            }
        }
    }

    /// **Factory palettes must not collide with the cold state color**
    /// (docs/UX.md; reasoning on [`BLOCKED_CYAN`]).
    ///
    /// Checked by **hue**, not by literal value: a different hex that
    /// looks just as cyan still goes red.
    ///
    /// **Near-neutrals are excluded from the hue check** (see
    /// [`CHROMA_FLOOR`]: a gray's hue is its residual cast, and
    /// measuring it measures a quantity that is not there).
    #[test]
    fn presets_keep_off_the_blocked_hue() {
        let cyan = hue(BLOCKED_CYAN);
        for p in PRESETS.iter() {
            for c in p.colors {
                assert_eq!(
                    norm_hex(c).as_deref(),
                    Some(c),
                    "{}'s {c} is not lowercase #rrggbb",
                    p.id
                );
                if chroma(c) < CHROMA_FLOOR {
                    continue;
                }
                let gap = hue_gap(hue(c), cyan);
                assert!(
                    gap >= BLOCKED_KEEPOUT,
                    "{}'s {c} is only {gap:.0}° from the cold state color (floor {BLOCKED_KEEPOUT}°)",
                    p.id
                );
            }
        }
        // **Positive control: this test can still catch something.** A
        // real cyan (high chroma, hue against the state color) has to be
        // judged — otherwise all-green above only proves it looked at
        // nothing.
        //
        // These two lines also close off "raise the floor until the test
        // is hollow": a floor above a real cyan's chroma reddens the
        // first one. **It does not assume anything about what ships** —
        // an earlier version counted "judged slots == total slots",
        // which assumed no factory palette ever holds a near-neutral,
        // and that is exactly the false positive being fixed.
        for cyanish in ["#01aac6", "#12b8d0", "#2aa9bd"] {
            assert!(
                chroma(cyanish) >= CHROMA_FLOOR,
                "{cyanish} slipped past the chroma floor and it is a real cyan"
            );
            assert!(
                hue_gap(hue(cyanish), cyan) < BLOCKED_KEEPOUT,
                "{cyanish} was not caught by the hue check"
            );
        }
        // And the other way: the slate must be held out of the check by
        // the floor (its hue sits against the cyan while its ΔE00 to it
        // is further than `#5d7e62`, which passes)
        assert!(chroma("#50595c") < CHROMA_FLOOR, "the slate was not read as near-neutral");
        assert!(
            hue_gap(hue("#50595c"), cyan) < BLOCKED_KEEPOUT,
            "this control lost its premise: the slate's hue is supposed to sit against the cyan"
        );
    }

    /// The chroma floor lets every slot in service be judged — nobody
    /// may raise it until the hue check covers nothing.
    #[test]
    fn at_least_judged() {
        for p in PRESETS.iter() {
            for c in p.colors {
                if c == "#50595c" {
                    continue; // the slate, held out on purpose
                }
                assert!(
                    chroma(c) >= CHROMA_FLOOR,
                    "{}'s {c} (C* {:.1}) fell under the chroma floor — either it is \
                     genuinely near-neutral, or the floor has been raised too far",
                    p.id,
                    chroma(c)
                );
            }
        }
    }

    /// **The alarm-red scoreboard: a ruler, not a gate.**
    ///
    /// ## What it measures
    ///
    /// Whether a palette holds a slot that reads at a glance as
    /// "something went wrong". The criterion: **Lab hue 20–50° and
    /// C* > 45** — the failed-state red's own C* is 42.3, so the
    /// threshold sits at "more saturated than that".
    ///
    /// ## Why it exists: a criterion recovered from a screenshot
    ///
    /// Originally two things were measured: **misreading** (how close
    /// rendered pixels land to a state color) and **masking** (how many
    /// times the face's chroma exceeds the state mark's). By the
    /// misreading measure mandala's `#fa3e3e` is nearly innocent — ΔE00
    /// 11.1 from the failed red, outside the threshold, and only 3.59%
    /// of rendered area "looks like the failed color".
    ///
    /// **That measure was about to conclude "the red does not compete,
    /// use it". Then a render of one screen had a row that was wrong**:
    /// a machine whose face was a slab of bright red, on a row whose
    /// state was unknown (gray).
    ///
    /// The cause is that it is **more saturated than the failed red**
    /// (C* 82.6 vs 42.3). Too saturated to match *that color*, but a
    /// perfect match for the *category* "red = trouble". **The earlier
    /// measure measured the former and missed the latter.**
    ///
    /// This repo's discipline is usually "don't trust your eyes, get
    /// numbers"; **this time it went the other way — the numbers missed
    /// a whole category and the screenshot was the only thing that
    /// found it**. It does not overturn "get numbers"; it says the
    /// numbers have to measure the right thing.
    ///
    /// ## Why it is not an assertion
    ///
    /// mandala's set has the highest alarm red of the eight (13.7% by
    /// render, 21 of 60 faces carrying it, worst face 69%), **and it
    /// was chosen as the default after seeing that number**. Writing
    /// "red if above X" would be a test overruling that decision.
    ///
    /// So this test **only prints**. What it owes the next person who
    /// adds a palette is one thing: **let them see the number.**
    ///
    /// ## One thing it cannot reach; don't treat it as the whole story
    ///
    /// This is **palette-level** (Rust does not render), while the real
    /// number is **render-level** — blur and overlay mix colors the
    /// palette does not contain. Measured:
    ///
    /// | palette | palette-level hit | rendered alarm red % |
    /// |---|---|---|
    /// | mandala | `#fa3e3e` | **13.7** |
    /// | Monokai | **none** | **8.5** |
    /// | Nord | none | 2.3 |
    /// | liquid | none | 1.7 |
    /// | Okabe-Ito | none | 0.5 |
    /// | Catppuccin / warm | none | 0.0 |
    ///
    /// **Monokai hits nothing at palette level and renders 8.5%**: its
    /// pink (Lab hue 9°) and orange (67°) sit on either side of the band
    /// and blur into it. Palette-level hits are therefore **necessary,
    /// not sufficient** — a new palette needs the browser measurement
    /// re-run.
    #[test]
    fn an_alarm_red_scoreboard_not_a_gate() {
        /// More saturated than the failed red (C* 42.3) to count
        const ALARM_CHROMA: f64 = 45.0;
        const ALARM_HUE: std::ops::RangeInclusive<f64> = 20.0..=50.0;

        let lab_hue = |hex: &str| {
            let (a, b) = lab_ab(hex);
            (b.atan2(a).to_degrees() + 360.0) % 360.0
        };
        let is_alarm =
            |hex: &str| ALARM_HUE.contains(&lab_hue(hex)) && chroma(hex) > ALARM_CHROMA;

        println!("\nalarm-red scoreboard (palette level; render level needs the browser rig)");
        for p in PRESETS.iter() {
            let hits: Vec<&str> = p.colors.iter().copied().filter(|c| is_alarm(c)).collect();
            println!(
                "  {:10} {}",
                p.id,
                if hits.is_empty() {
                    "—".to_string()
                } else {
                    hits.iter()
                        .map(|c| format!("{c}(h{:.0} C*{:.0})", lab_hue(c), chroma(c)))
                        .collect::<Vec<_>>()
                        .join(" ")
                }
            );
        }

        // **Positive and negative controls: the ruler still measures.**
        // It gates nothing, but it may not silently break — a broken
        // ruler prints a column of "—", and that reads like good news
        assert!(is_alarm("#fa3e3e"), "it cannot see mandala's cinnabar; the ruler is broken");
        assert!(!is_alarm("#f92672"), "pink (Lab hue 9°) is outside the band");
        assert!(!is_alarm("#a6e22e"), "acid green is not an alarm red");
        assert!(!is_alarm("#eb9d8d"), "warm's terracotta is only C* 34, not saturated enough");
        // The failed red itself: inside the band by hue, just under the
        // chroma threshold — which is where the threshold came from
        assert!(ALARM_HUE.contains(&lab_hue("#d4756b")));
        assert!(chroma("#d4756b") < ALARM_CHROMA);
    }

    /// Factory palettes may not collapse their **hue span** again.
    ///
    /// The previous set spanned 10°, which is half of why a screen of
    /// faces looked alike (the other half is the 9 combinations, see
    /// `palette_combinations`). The floor is 60°: measured spans are
    /// 84°–208° and the old one was 10° — it guards against collapse,
    /// not jitter.
    #[test]
    fn presets_spread_their_hues() {
        for p in PRESETS.iter() {
            let mut hs: Vec<f64> = p.colors.iter().map(|c| hue(c)).collect();
            hs.sort_by(|a, b| a.partial_cmp(b).unwrap());
            // The complement of the largest gap after sorting is the arc
            // this palette is crammed into
            let mut biggest = 360.0 - (hs[hs.len() - 1] - hs[0]);
            for w in hs.windows(2) {
                biggest = biggest.max(w[1] - w[0]);
            }
            let span = 360.0 - biggest;
            println!("{} hue span {span:.0}°", p.id);
            assert!(span >= 60.0, "{}'s hue span is only {span:.0}°", p.id);
        }
    }

    /// Palette parsing: **no leniency**, on every path including serde.
    #[test]
    fn palette_parsing_is_strict() {
        let ok = ["#f2f6cb", "#DCE9A6", "#b7cf75", "#7d2540", "#3f4482"].map(String::from);
        let p = Palette::parse(&ok).unwrap();
        // Uppercase normalizes to lowercase: both ends receive the same
        // string, and TS/Swift compare colors literally
        assert_eq!(p.colors()[1], "#dce9a6");

        let four: Vec<String> = ok[..4].to_vec();
        assert!(Palette::parse(&four).is_none(), "four slots were accepted");
        let six: Vec<String> = ok.iter().cloned().chain(["#000000".into()]).collect();
        assert!(Palette::parse(&six).is_none(), "six slots were accepted");
        for bad in ["#fff", "f2f6cb", "#gggggg", "rgb(1,2,3)", ""] {
            let mut v = ok.to_vec();
            v[2] = bad.into();
            assert!(Palette::parse(&v).is_none(), "{bad} was accepted");
        }

        // **The serde door is the same gate.** A style arriving from
        // another machine goes through `Deserialize`, and a second
        // validator there would be a second thing to drift.
        assert!(
            AvatarStyle::from_json(
                r##"{"palette":["#fff","#dce9a6","#b7cf75","#7d2540","#3f4482"],
                     "variant":"marble","shape":"circle"}"##
            )
            .is_none(),
            "a short hex slipped in through serde"
        );
        assert!(
            AvatarStyle::from_json(
                r##"{"palette":["#f2f6cb","#dce9a6","#b7cf75","#7d2540","#3f4482"],
                     "variant":"holodeck","shape":"circle"}"##
            )
            .is_none(),
            "an unknown variant slipped in through serde"
        );
        // …and a good one round-trips, so the refusals above are not
        // just "nothing parses"
        let good = AvatarStyle {
            palette: Palette::from_preset(preset("okabe").unwrap()),
            variant: Variant::Bauhaus,
            shape: FaceShape::Square,
        };
        let back = AvatarStyle::from_json(&good.to_json().unwrap()).expect("a good style parses");
        assert_eq!(back, good);
    }

    /// **The three factory defaults**: mandala / marble / circle.
    ///
    /// What this guards is not "the defaults may not change" — it is
    /// **changing only half of them**. Whoever paints a device that has
    /// never reported a style uses these, so a half-change means the
    /// same fresh install looks different in two places, with no
    /// compile-time signal and no runtime error.
    ///
    /// The value is not written as "equals PRESETS[0]": that would tie
    /// "the default" to "the first row", and then **reordering the
    /// table silently changes every new user's palette while the test
    /// stays green**.
    #[test]
    fn the_three_defaults_are_the_ones_that_ship() {
        let d = AvatarStyle::default();
        assert_eq!(DEFAULT_PRESET, "mandala");
        assert_eq!(d.variant, Variant::Marble, "the default variant is marble");
        assert_eq!(d.shape, FaceShape::Circle, "the default shape is a circle");
        assert_eq!(d.shape.radius_ratio(), 0.5);

        // Compare slot by slot against the table row; **do not copy five
        // hex values here** — a copy is one more thing that drifts, and
        // after drifting this test would still be green
        let p = preset(DEFAULT_PRESET).expect("the default row must exist in the table");
        assert_eq!(d.palette.colors(), &p.colors.map(String::from));

        // **It really is not the first row** — this line exists so that
        // if someone moves mandala to the top and reverts
        // `Palette::default` to index 0, the assertions above stay green
        // while the "written in one place" criterion is gone. When it
        // goes red, the reader lands on `Palette::default`'s note
        // instead of just changing the index back
        assert_ne!(
            PRESETS[0].id, DEFAULT_PRESET,
            "the default moved to the head of the table. That order means provenance; \
             don't make it moonlight as the default"
        );
        assert_eq!(PRESETS[0].id, "liquid");
    }

    /// The default row exists in the table (what [`Palette::default`]'s
    /// fallback covers).
    #[test]
    fn the_default_preset_exists() {
        assert!(preset(DEFAULT_PRESET).is_some());
        // Ids are unique, or `preset()` silently answers with whichever
        // came first
        let ids: HashSet<&str> = PRESETS.iter().map(|p| p.id).collect();
        assert_eq!(ids.len(), PRESETS.len(), "two palettes share an id");
    }

    /// **A key and its wire tag are the same string.**
    ///
    /// The two are written apart — `key()` is a match, the tag is a
    /// serde attribute — and a settings screen sends a key back over the
    /// very wire the tag names. If they drift, the button that chose a
    /// variant and the face that came back disagree about which one it
    /// was, and **nothing anywhere says so**: the style still parses,
    /// still paints, and simply is not what was pressed.
    ///
    /// Asserted against the *serialized* form rather than a literal
    /// table, so renaming the tag alone is what goes red.
    #[test]
    fn a_key_is_the_tag_it_travels_under() {
        for v in Variant::ALL {
            assert_eq!(serde_json::to_string(&v).unwrap(), format!("\"{}\"", v.key()));
            assert_eq!(Variant::from_key(v.key()), Some(v), "{} does not come back", v.key());
        }
        for s in FaceShape::ALL {
            assert_eq!(serde_json::to_string(&s).unwrap(), format!("\"{}\"", s.key()));
            assert_eq!(FaceShape::from_key(s.key()), Some(s), "{} does not come back", s.key());
        }
        // Refused by name, never defaulted: a typo must not paint marble
        // in a circle and look like the setting failed to take.
        assert_eq!(Variant::from_key("holodeck"), None);
        assert_eq!(FaceShape::from_key("holodeck"), None);
        // The keys are distinct within each axis, or `from_key` answers
        // with whichever came first.
        let vs: HashSet<&str> = Variant::ALL.iter().map(|v| v.key()).collect();
        assert_eq!(vs.len(), 3, "two variants share a key");
        let ss: HashSet<&str> = FaceShape::ALL.iter().map(|s| s.key()).collect();
        assert_eq!(ss.len(), 3, "two shapes share a key");
    }

    /// **"Which factory palette is this" has to answer `None` once a
    /// slot has been touched.**
    ///
    /// The chooser marks the palette this returns. A nearest-match
    /// answer would light a button the user did not press, and pressing
    /// that lit button would then look like a control that does nothing
    /// — the failure docs/UX.md names as 做了但没变化.
    ///
    /// The control is the second half: every factory row must recognize
    /// itself, or the whole thing could answer `None` always and the
    /// first assertion would pass.
    #[test]
    fn a_palette_names_its_factory_set_and_only_that() {
        for p in PRESETS {
            assert_eq!(p.palette().preset_id(), Some(p.id), "{} does not know itself", p.id);
        }
        // One slot off the default set, and it is nobody's set. The
        // replacement is a color no factory palette holds.
        let mut colors = preset(DEFAULT_PRESET).unwrap().colors.map(String::from);
        colors[2] = "#123456".to_owned();
        assert_eq!(Palette::parse(&colors).unwrap().preset_id(), None);
    }

    /// Three shapes give three different corner ratios, and **only
    /// three**.
    #[test]
    fn every_shape_has_its_own_ratio() {
        let rs: HashSet<String> = [FaceShape::Circle, FaceShape::Rounded, FaceShape::Square]
            .iter()
            .map(|s| format!("{}", s.radius_ratio()))
            .collect();
        assert_eq!(rs.len(), 3, "two shapes crop to the same radius: {rs:?}");
        assert_eq!(FaceShape::Circle.radius_ratio(), 0.5);
        // Rounded stays 0.3: changing it reshapes every existing user's face
        assert_eq!(FaceShape::Rounded.radius_ratio(), 0.3);
        assert_eq!(FaceShape::Square.radius_ratio(), 0.0);
    }

    /// **No axis is a function of another** — the gate for the module
    /// header's difference two.
    ///
    /// The collision-rate test only sees the total and cannot say which
    /// dimension collapsed, and collapse is exactly what copying
    /// upstream produces. This pins upstream's four overlaps:
    ///
    /// 1. |dx| ≡ |dy| (upstream takes `n % 8` for both)
    /// 2. scale determined by |dx| (upstream `n % 4`, and 4 divides 8)
    /// 3. dx and rotate always agree in sign (upstream shares `digit(n, 1)`)
    /// 4. the two veins always share a scale (upstream's
    ///    `properties[2].scale` typo)
    #[test]
    fn no_axis_is_a_function_of_another() {
        let (mut same_abs, mut same_sign, mut same_scale) = (0, 0, 0);
        let mut scale_by_dx: HashMap<i64, HashSet<String>> = HashMap::new();
        const N: usize = 4_000;
        let style = golden_style();
        for seed in seeds(N, 0x51ed_270b_5a1f_c33d) {
            let AvatarPaint::Marble { veins, .. } = avatar(&seed, &style).paint else {
                panic!("the golden style's variant is marble");
            };
            let (s, w) = (&veins[0], &veins[1]);
            if s.dx.abs() == s.dy.abs() {
                same_abs += 1;
            }
            if (s.dx >= 0.0) == (s.rotate >= 0.0) {
                same_sign += 1;
            }
            if s.scale == w.scale {
                same_scale += 1;
            }
            scale_by_dx
                .entry(s.dx.abs() as i64)
                .or_default()
                .insert(format!("{}", s.scale));
        }
        // Each floor is set against **its own** null hypothesis, not a
        // uniform half.
        //
        // An earlier version wrote `< N/2` for all three, but
        // `same_sign`'s null hypothesis **is exactly 50%** (dx's and
        // rotate's signs come from two unrelated decimal digits, and two
        // i.i.d. signs agree half the time). That assertion was betting
        // a fair coin lands below half — it would go red half the time
        // on a new set of seeds, with nothing broken.
        //
        // Nulls: |dx|≡|dy| is 1/8, the two scales matching is 1/4, signs
        // agreeing is 1/2. Upstream's formulas drive all three **to N**
        // (100%), so a floor between the null and 100% suffices; it
        // guards collapse, not jitter.
        assert!(same_abs < N * 2 / 5, "|dx| and |dy| are still one number: {same_abs}/{N}");
        assert!(same_scale < N / 2, "the two veins still share a scale: {same_scale}/{N}");
        assert!(same_sign < N * 3 / 4, "dx and rotate still share a sign digit: {same_sign}/{N}");
        // Every |dx| should have seen more than one scale — upstream's
        // version has exactly one here
        for (dx, scales) in &scale_by_dx {
            assert!(scales.len() > 1, "|dx|={dx} maps to a single scale: {scales:?}");
        }
    }

    /// The geometry's own invariants: each variant's primitive count,
    /// order and ranges.
    #[test]
    fn geometry_stays_in_range() {
        for style in all_styles() {
            for seed in seeds(200, 0x0123_4567_89ab_cdef) {
                let a = avatar(&seed, &style);
                // beam has its own canvas (see `BEAM_CANVAS`), it is not
                // always `CANVAS`
                assert_eq!(
                    a.canvas,
                    if style.variant == Variant::Beam { BEAM_CANVAS } else { CANVAS }
                );
                assert_eq!(a.radius_ratio, style.shape.radius_ratio());
                match &a.paint {
                    AvatarPaint::Marble { blur, veins } => {
                        assert_eq!(*blur, BLUR);
                        assert_eq!(veins.len(), 2);
                        assert_eq!(veins[0].vein, Vein::Shard);
                        assert_eq!(veins[1].vein, Vein::Sweep);
                        // Only the second overlays (the client paints
                        // what `overlay` says, it does not look it up)
                        assert!(!veins[0].overlay);
                        assert!(veins[1].overlay);
                        for v in veins {
                            // Upstream SIZE/10 = 8: translation in (-8, 8)
                            assert!(v.dx.abs() < CANVAS / 10.0, "dx {} out of range", v.dx);
                            assert!(v.dy.abs() < CANVAS / 10.0, "dy {} out of range", v.dy);
                            assert!(v.rotate.abs() < 360.0, "rotate {}", v.rotate);
                            // Upstream 1.2 + (0..4)/10. No float drift
                            // either: four steps, these four numbers
                            assert!(
                                [12, 13, 14, 15].contains(&((v.scale * 10.0).round() as i64)),
                                "scale {} is not one of the four steps",
                                v.scale
                            );
                        }
                    }
                    AvatarPaint::Bauhaus { shapes } => {
                        assert_eq!(shapes.len(), 3);
                        assert_eq!(shapes[0].shape, Shape::Rect);
                        assert_eq!(shapes[1].shape, Shape::Circle);
                        assert_eq!(shapes[2].shape, Shape::Rect);
                        // Upstream `SIZE/2 - (i+17)`: 22 / 21 / 20
                        for (s, range) in shapes.iter().zip([22.0, 21.0, 20.0]) {
                            assert!(s.dx.abs() < range, "dx {} out of range ({range})", s.dx);
                            assert!(s.dy.abs() < range, "dy {} out of range ({range})", s.dy);
                            assert!(s.rotate.abs() < 360.0, "rotate {}", s.rotate);
                        }
                        // The circle does not rotate (neither upstream)
                        assert_eq!(shapes[1].rotate, 0.0);
                    }
                    AvatarPaint::Beam {
                        wrapper_rx,
                        wrapper_scale,
                        wrapper_rotate,
                        wrapper_dx,
                        wrapper_dy,
                        wrapper_color,
                        face_ink,
                        mouth_open: _,
                        mouth_spread,
                        eye_spread,
                        face_rotate,
                        face_dx,
                        face_dy,
                    } => {
                        assert!(
                            (0.0..360.0).contains(wrapper_rotate),
                            "wrapper_rotate {wrapper_rotate} out of range"
                        );
                        // Upstream 1 + (0..2)/10: three steps
                        assert!(
                            [10, 11, 12].contains(&((wrapper_scale * 10.0).round() as i64)),
                            "wrapper_scale {wrapper_scale} is not one of the three steps"
                        );
                        assert!(
                            *wrapper_rx == BEAM_CANVAS / 2.0
                                || *wrapper_rx == BEAM_CANVAS / 6.0,
                            "wrapper_rx {wrapper_rx} is neither the circle nor the rounded step"
                        );
                        assert!(wrapper_color.starts_with('#'));
                        assert!(
                            face_ink == "#000000" || face_ink == "#ffffff",
                            "face_ink {face_ink} is neither pure black nor pure white"
                        );
                        // Upstream ranges 5 and 3, both unsigned
                        assert!((0.0..5.0).contains(eye_spread), "eye_spread {eye_spread}");
                        assert!((0.0..3.0).contains(mouth_spread), "mouth_spread {mouth_spread}");
                        // Upstream range 10, signed
                        assert!(face_rotate.abs() < 10.0, "face_rotate {face_rotate}");
                        // wrapper_dx/dy's tight bound is awkward because
                        // upstream's "if it barely moved, push it out"
                        // ternary is asymmetric (< 5 adds `SIZE/9`,
                        // otherwise unchanged); this is a loose but still
                        // effective envelope (derivation in `beam_props`)
                        assert!(wrapper_dx.abs() <= 9.0, "wrapper_dx {wrapper_dx}");
                        assert!(wrapper_dy.abs() <= 9.0, "wrapper_dy {wrapper_dy}");
                        // face_dx/dy is either half the wrapper's
                        // translation or the independent jitter (ranges
                        // 8 and 7); the envelope is their union
                        assert!(face_dx.abs() <= 7.0, "face_dx {face_dx}");
                        assert!(face_dy.abs() <= 7.0, "face_dy {face_dy}");
                    }
                }
            }
        }
    }

    /// Golden values: one seed's **whole** output pinned field by field.
    ///
    /// Two uses. Regression — the hash, digit extraction, geometric
    /// constants, palette or color rule moving turns this red
    /// immediately, instead of someone noticing a machine changed face
    /// months later. And **cross-repo**: these are the same numbers
    /// mandala's verified implementation produces, so the port is
    /// provably the same derivation, not a rewrite that looks similar.
    ///
    /// The style is [`golden_style`] (liquid / marble / rounded) rather
    /// than the current defaults; the reason is on that function.
    ///
    /// The seed is `"abcd"` (hash = 2987074), small enough to compute
    /// by hand:
    ///
    /// ```text
    /// marble shard n = 2×2987074 = 5974148
    ///   rotate: 5974148 % 360 = 308, tens digit 4 (even) → -308
    ///   dx:     5974148 / 360 = 16594, % 8 = 2, tens digit 9 (odd) → +2
    ///   dy:     16594 / 8 = 2074, % 8 = 2, hundreds digit 0 (even) → -2
    ///   scale:  2074 / 8 = 259, % 4 = 3 → 1.2 + 0.3 = 1.5
    /// color chain q = 2987074 / 360 = 8297
    ///   ground: 8297 % 5 = 2 → liquid[2] = #b7cf75; q = 1659
    ///   first:  the other four by contrast against #b7cf75, descending, are
    ///           [#7d2540, #3f4482, #f2f6cb, #dce9a6]; 1659 % 2 = 1 → #3f4482; q = 829
    ///   second: of what is left [#7d2540, #f2f6cb, #dce9a6]; 829 % 3 = 1 → #f2f6cb
    /// ```
    #[test]
    fn golden_face() {
        assert_eq!(hash_code("abcd"), 2_987_074);
        let a = avatar(&seed_of("abcd"), &golden_style());
        assert_eq!(a.canvas, 80.0);
        assert_eq!(a.radius_ratio, 0.3);
        assert_eq!(a.background, "#b7cf75");
        let AvatarPaint::Marble { blur, veins } = &a.paint else {
            panic!("the golden style's variant is marble");
        };
        assert_eq!(*blur, 7.0);
        let f = |v: &AvatarVein| {
            (v.vein, v.scale, v.rotate, v.dx, v.dy, v.overlay, v.color.clone())
        };
        assert_eq!(
            f(&veins[0]),
            (Vein::Shard, 1.5, -308.0, 2.0, -2.0, false, "#3f4482".into())
        );
        assert_eq!(
            f(&veins[1]),
            (Vein::Sweep, 1.2, -102.0, 4.0, 7.0, true, "#f2f6cb".into())
        );
    }

    /// Bauhaus's golden values. Same seed, same palette.
    #[test]
    fn golden_bauhaus_face() {
        let style = AvatarStyle { variant: Variant::Bauhaus, ..golden_style() };
        let a = avatar(&seed_of("abcd"), &style);
        assert_eq!(a.background, "#b7cf75");
        let AvatarPaint::Bauhaus { shapes } = &a.paint else {
            panic!("the variant should be bauhaus");
        };
        let f = |s: &AvatarShape| {
            (s.shape, s.x, s.y, s.w, s.h, s.rotate, s.dx, s.dy, s.color.clone())
        };
        // The bar's h is 80, not 10: `get_boolean(h, 2)` on hash=2987074
        // reads the hundreds digit (0, even) as true, so on this seed the
        // bar became a **whole block**. Having one of each across the
        // goldens is deliberate — pinning only the thin-rule step leaves
        // the `is_square` branch unwatched
        assert_eq!(
            f(&shapes[0]),
            (Shape::Rect, 10.0, 30.0, 80.0, 80.0, 308.0, -4.0, 4.0, "#3f4482".into())
        );
        assert_eq!(
            f(&shapes[1]),
            (Shape::Circle, 24.0, 24.0, 32.0, 32.0, 0.0, -18.0, -18.0, "#f2f6cb".into())
        );
        assert_eq!(
            f(&shapes[2]),
            (Shape::Rect, 0.0, 39.0, 80.0, 2.0, 256.0, 16.0, -16.0, "#7d2540".into())
        );
    }

    /// Beam's golden values. Same seed, same palette.
    #[test]
    fn golden_beam_face() {
        let style = AvatarStyle { variant: Variant::Beam, ..golden_style() };
        let a = avatar(&seed_of("abcd"), &style);
        assert_eq!(a.canvas, BEAM_CANVAS, "beam's canvas is 36, not 80");
        assert_eq!(a.background, "#b7cf75");
        let AvatarPaint::Beam {
            wrapper_rx,
            wrapper_scale,
            wrapper_rotate,
            wrapper_dx,
            wrapper_dy,
            wrapper_color,
            face_ink,
            mouth_open,
            mouth_spread,
            eye_spread,
            face_rotate,
            face_dx,
            face_dy,
        } = &a.paint
        else {
            panic!("the variant should be beam");
        };
        assert_eq!(
            (
                *wrapper_rx,
                *wrapper_scale,
                *wrapper_rotate,
                *wrapper_dx,
                *wrapper_dy,
                wrapper_color.as_str(),
            ),
            (18.0, 1.2, 154.0, 7.0, 8.0, "#3f4482")
        );
        assert_eq!(
            (
                face_ink.as_str(),
                *mouth_open,
                *mouth_spread,
                *eye_spread,
                *face_rotate,
                *face_dx,
                *face_dy,
            ),
            ("#ffffff", true, 1.0, 3.0, -9.0, 3.5, 4.0)
        );
    }

    /// **The shape of the JSON on the wire.** Both ends parse it by
    /// name, and these are the names.
    ///
    /// Especially `paint`'s internal tag: the client reads
    /// `paint.variant` in exactly one place, and reading it wrong means
    /// painting a whole face wrong.
    #[test]
    fn the_wire_shape_is_exactly_this() {
        let v: serde_json::Value =
            serde_json::to_value(avatar(&seed_of("abcd"), &golden_style())).unwrap();
        let keys: HashSet<&str> = v.as_object().unwrap().keys().map(|s| s.as_str()).collect();
        assert_eq!(
            keys,
            HashSet::from(["canvas", "background", "radius_ratio", "paint"]),
            "the avatar frame's key set changed"
        );
        assert_eq!(v["paint"]["variant"], "marble");
        let vein_keys: HashSet<&str> = v["paint"]["veins"][0]
            .as_object()
            .unwrap()
            .keys()
            .map(|s| s.as_str())
            .collect();
        assert_eq!(
            vein_keys,
            HashSet::from(["vein", "scale", "rotate", "dx", "dy", "overlay", "color"])
        );

        let style = AvatarStyle { variant: Variant::Bauhaus, ..golden_style() };
        let b: serde_json::Value =
            serde_json::to_value(avatar(&seed_of("abcd"), &style)).unwrap();
        assert_eq!(b["paint"]["variant"], "bauhaus");
        // **Bauhaus carries no blur**: the two variants do not share a
        // parameter set, which is why this is a tagged union rather than
        // "leave an array empty"
        assert!(b["paint"].get("blur").is_none(), "bauhaus should not carry blur");
        let shape_keys: HashSet<&str> = b["paint"]["shapes"][0]
            .as_object()
            .unwrap()
            .keys()
            .map(|s| s.as_str())
            .collect();
        assert_eq!(
            shape_keys,
            HashSet::from(["shape", "x", "y", "w", "h", "rotate", "dx", "dy", "color"])
        );

        let beam_style = AvatarStyle { variant: Variant::Beam, ..golden_style() };
        let m: serde_json::Value =
            serde_json::to_value(avatar(&seed_of("abcd"), &beam_style)).unwrap();
        assert_eq!(m["canvas"], 36.0, "beam's canvas is 36, not 80");
        assert_eq!(m["paint"]["variant"], "beam");
        // Beam has no veins/shapes, it is a flat set of fields (see
        // `AvatarPaint::Beam`: one wrapper and one face, not a sequence
        // of primitives)
        assert!(m["paint"].get("veins").is_none(), "beam should not carry veins");
        assert!(m["paint"].get("shapes").is_none(), "beam should not carry shapes");
        let beam_keys: HashSet<&str> =
            m["paint"].as_object().unwrap().keys().map(|s| s.as_str()).collect();
        assert_eq!(
            beam_keys,
            HashSet::from([
                "variant",
                "wrapper_rx",
                "wrapper_scale",
                "wrapper_rotate",
                "wrapper_dx",
                "wrapper_dy",
                "wrapper_color",
                "face_ink",
                "mouth_open",
                "mouth_spread",
                "eye_spread",
                "face_rotate",
                "face_dx",
                "face_dy",
            ])
        );
    }

    /// Port fidelity: a handful of hand-computed intermediates. Touching
    /// the hash or the digit helpers reddens here first.
    #[test]
    fn matches_reference_helpers() {
        // hash("a") = 97; hash("ab") = 97*31 + 98 = 3105
        assert_eq!(hash_code("a"), 97);
        assert_eq!(hash_code("ab"), 3105);
        assert_eq!(hash_code(""), 0);

        // getDigit(12345, 0..4) = 5,4,3,2,1
        assert_eq!(
            (0..5).map(|i| get_digit(12345, i)).collect::<Vec<_>>(),
            vec![5, 4, 3, 2, 1]
        );
        // getBoolean: digit 1 is 4, even → true; digit 0 is 5, odd → false
        assert!(get_boolean(12345, 1));
        assert!(!get_boolean(12345, 0));

        // getUnit(12345, 100, Some(1)): value = 45, digit 1 is 4, even → negate
        assert_eq!(get_unit(12345, 100, Some(1)), -45);
        // digit 0 is 5, odd → no negation
        assert_eq!(get_unit(12345, 100, Some(0)), 45); // index=0 short-circuits in JS
        assert_eq!(get_unit(12345, 100, None), 45);
        // digit 2 is 3, odd → no negation
        assert_eq!(get_unit(12345, 100, Some(2)), 45);

        // Contrast: white on black is 21, a color on itself is 1 (the two
        // WCAG endpoints, pinning the formula)
        assert!((contrast("#ffffff", "#000000") - 21.0).abs() < 1e-9);
        assert!((contrast("#7d2540", "#7d2540") - 1.0).abs() < 1e-12);
    }
}
