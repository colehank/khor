//! A pairing ticket as a QR code, for the screen (2026-09-30, the owner:
//! 「邀请设备显示两个内容：现在的 token，和一个二维码，后者为手机连接奠定基础」).
//!
//! **One invite, two spellings.** What the code carries is the token
//! itself, byte for byte — not a URL around it, not a second value. A
//! phone that scans it one day gets exactly the string 加入网络 takes
//! today, so there is nothing to keep in step between the two.
//!
//! **Drawn by the face, encoded here.** The answer is SVG path data in
//! module units and the symbol's size, so the face paints it with
//! `currentColor` at whatever size it has room for and brings in no QR
//! library of its own; the one encoder in this tree is the `qrcode`
//! crate the terminal's `khor web` block already uses (MIT/Apache).

use serde::Serialize;
use ts_rs::TS;

/// The light margin around the symbol, in modules. **Part of the code,
/// not decoration**: ISO/IEC 18004 asks for four, and without it a
/// scanner cannot find the three finder squares against whatever is
/// around the picture. It is in the path's coordinates rather than left
/// to the face, so no face can draw the code without it.
pub const QUIET: u32 = 4;

/// A QR code, ready to be drawn as one `<path>`.
#[derive(Debug, Clone, Serialize, TS)]
pub struct Qr {
    /// Modules across the whole picture, quiet zone included — the
    /// `viewBox` is `0 0 span span`. The symbol itself is `span - 8`.
    pub span: u32,
    /// Every dark module, as SVG path data in module units, already
    /// offset by [`QUIET`]. One rectangle per horizontal run of dark
    /// modules (`M x y h n v 1 h -n z`), which keeps a ticket-sized code
    /// to a few kilobytes and leaves no hairline seams between
    /// neighbouring modules when it is scaled.
    pub path: String,
}

/// `text` as a QR code at the encoder's default error correction (M), or
/// `None` when it is too long for any version — then the face shows the
/// token alone, which is still the whole invite.
pub fn encode(text: &str) -> Option<Qr> {
    let code = qrcode::QrCode::new(text.as_bytes()).ok()?;
    let width = code.width();
    let modules = code.to_colors();
    let dark = |x: usize, y: usize| modules[y * width + x] == qrcode::Color::Dark;
    let mut path = String::new();
    for y in 0..width {
        let mut x = 0;
        while x < width {
            if !dark(x, y) {
                x += 1;
                continue;
            }
            let start = x;
            while x < width && dark(x, y) {
                x += 1;
            }
            let run = x - start;
            let (px, py) = (start + QUIET as usize, y + QUIET as usize);
            path.push_str(&format!("M{px} {py}h{run}v1h-{run}z"));
        }
    }
    Some(Qr {
        span: width as u32 + QUIET * 2,
        path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Paints `path` back onto a `span`×`span` grid. Knows only the five
    /// commands [`encode`] writes, and fails loud on anything else, so a
    /// change of spelling there cannot quietly read back as an empty
    /// picture.
    fn paint(qr: &Qr) -> Vec<Vec<bool>> {
        let span = qr.span as usize;
        let mut grid = vec![vec![false; span]; span];
        for rect in qr.path.split('z').filter(|s| !s.is_empty()) {
            let rest = rect.strip_prefix('M').expect("each run starts with M");
            let (xy, rest) = rest.split_once('h').expect("then h");
            let (x, y) = xy.split_once(' ').expect("M x y");
            let (run, rest) = rest.split_once('v').expect("then v");
            let (one, back) = rest.split_once('h').expect("then h back");
            assert_eq!(one, "1", "a run is one module tall");
            assert_eq!(back, format!("-{run}"), "and closes on itself");
            let (x, y, run): (usize, usize, usize) =
                (x.parse().unwrap(), y.parse().unwrap(), run.parse().unwrap());
            for cell in &mut grid[y][x..x + run] {
                assert!(!*cell, "two runs cover ({x},{y})");
                *cell = true;
            }
        }
        grid
    }

    /// **The picture is the code of the token, and of nothing else.**
    /// The run-length packing is the part written here rather than by
    /// the library, and every way it can be wrong — a run cut one short,
    /// rows off by one, the margin on three sides — still draws
    /// something that looks like a QR code and does not scan. So the
    /// path is read back and compared module for module against what
    /// the encoder makes of the very same bytes.
    #[test]
    fn the_drawn_code_is_the_token_itself() {
        let token = "khorAAQCAYDAMBQHAZDIMJRGIYTEMBTGAYDAMBQGAYDAMBQGAYDAMBQGA2DMOBRHE";
        let qr = encode(token).expect("a ticket-sized string encodes");
        let code = qrcode::QrCode::new(token.as_bytes()).unwrap();
        let width = code.width();
        let modules = code.to_colors();
        assert_eq!(qr.span as usize, width + 2 * QUIET as usize);
        let grid = paint(&qr);
        for y in 0..width {
            for x in 0..width {
                assert_eq!(
                    grid[y + QUIET as usize][x + QUIET as usize],
                    modules[y * width + x] == qrcode::Color::Dark,
                    "module ({x},{y}) came out wrong — the picture is not this token"
                );
            }
        }
    }

    /// Four light modules on every side, the standard's quiet zone.
    #[test]
    fn four_light_modules_surround_the_symbol() {
        let qr = encode("khor").unwrap();
        let grid = paint(&qr);
        let span = qr.span as usize;
        let q = QUIET as usize;
        for (y, row) in grid.iter().enumerate() {
            for (x, &on) in row.iter().enumerate() {
                if x < q || y < q || x >= span - q || y >= span - q {
                    assert!(!on, "({x},{y}) is dark inside the quiet zone");
                }
            }
        }
        // …and the symbol reaches it: a finder square sits in each of
        // three corners, so the margin is exactly four, not more.
        assert!(grid[q][q] && grid[q][span - q - 1] && grid[span - q - 1][q]);
    }

    #[test]
    fn too_long_to_encode_is_none_not_a_broken_picture() {
        assert!(encode(&"x".repeat(8_000)).is_none());
    }
}
