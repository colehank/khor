//! Real-connection acceptance for self-reported faces: a machine's face
//! is **its own palette times its id**, and the palette half has to
//! survive the trip.
//!
//! Two nodes on one machine, real UDP, real pairing, real sync — and the
//! control group that makes the result mean something: beta must paint
//! alpha differently from how it paints itself, on the same screen, from
//! the same code path. Every await is under a timeout.

use std::path::PathBuf;
use std::time::Duration;

use khor_node::{avatar, preset, AvatarSeed, AvatarStyle, FaceShape, Node};
use tokio::time::timeout;

fn root(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("khor-face-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

async fn wait_for_endpoint_file(root: &PathBuf) {
    let path = root.join(".khor").join("endpoint.json");
    timeout(Duration::from_secs(10), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("serve should write endpoint.json within 10s");
}

/// A style that is **not** the factory default in all three of its
/// parts, so "the report arrived" cannot be confused with "it fell back
/// to the default and happened to match".
fn a_reported_style() -> AvatarStyle {
    let json = r##"{
        "palette": ["#e69f00", "#009e4f", "#f0e442", "#d55e00", "#cc79a7"],
        "variant": "bauhaus",
        "shape": "square"
    }"##;
    let s = AvatarStyle::from_json(json).expect("the fixture must parse");
    assert_ne!(s, AvatarStyle::default(), "the fixture has to differ from the default");
    s
}

#[tokio::test]
async fn a_machine_is_painted_in_the_palette_it_reported() {
    let ra = root("a");
    let rb = root("b");

    // The two doors onto one identity agree: the bytes iroh hands us and
    // the hex the device table stores seed the same face. Nothing else
    // checks that these two encodings match, and if they ever drifted
    // the symptom would be one machine with two faces.
    let a = Node::open_as(ra.clone(), "alpha").unwrap();
    assert_eq!(
        AvatarSeed::of(&a.device()),
        AvatarSeed::from_id_hex(a.device_str()).expect("the table's id is hex"),
        "the id in bytes and the id in hex must seed the same face"
    );

    // alpha chooses a style before anyone can see it.
    a.set_avatar_style(&a_reported_style()).unwrap();
    assert_eq!(a.avatar_style(), a_reported_style(), "the choice must persist locally");

    let server = Node::open_as(ra.clone(), "alpha").unwrap();
    let serve = tokio::spawn(async move { server.serve().await });
    wait_for_endpoint_file(&ra).await;

    let ticket = a.invite().unwrap();
    let b = Node::open_as(rb.clone(), "beta").unwrap();
    timeout(Duration::from_secs(15), b.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .unwrap();
    timeout(Duration::from_secs(20), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();

    // 1) beta's table carries what alpha said about itself.
    let devices = b.devices().unwrap();
    let alpha = devices.iter().find(|d| d.name == "alpha").expect("alpha should be in the table");
    let reported = alpha
        .style
        .as_deref()
        .and_then(AvatarStyle::from_json)
        .expect("alpha's reported style should have travelled and should parse");
    assert_eq!(reported, a_reported_style(), "the style must arrive unchanged");

    // 2) …and it reaches the paint. The control is the same face
    //    derived with the default style: equal here would mean the
    //    report travelled and was then ignored, which every assertion
    //    above would still pass.
    let face = b.face_of(alpha).expect("beta must be able to paint alpha");
    let seed = AvatarSeed::from_id_hex(&alpha.id).unwrap();
    assert_ne!(
        face,
        avatar(&seed, &AvatarStyle::default()),
        "alpha is being painted with the default palette, not the one it reported"
    );
    assert_eq!(face, avatar(&seed, &reported), "the painted face is the reported style's");

    // 3) The point of self-reporting: on one screen, beta paints alpha
    //    in alpha's palette and itself in its own. Before this, style
    //    was stored per viewer and the same machine was two colors on
    //    two screens with neither side wrong.
    let beta = devices.iter().find(|d| d.name == "beta").expect("beta should be in the table");
    assert_eq!(
        beta.style.as_deref().and_then(AvatarStyle::from_json),
        Some(AvatarStyle::default()),
        "beta reports its own default, and reports it explicitly"
    );
    let beta_face = b.face_of(beta).expect("beta must be able to paint itself");
    assert_ne!(
        face.background, beta_face.background,
        "two machines with different palettes must not share a ground color"
    );

    // 4) The seed half: same style, different id, different face. A
    //    palette that decided the whole face would make every machine in
    //    one network look alike, which is what an avatar exists to stop.
    let beta_seed = AvatarSeed::from_id_hex(&beta.id).unwrap();
    assert_ne!(
        avatar(&seed, &reported),
        avatar(&beta_seed, &reported),
        "two machines under one style must still differ"
    );

    // 5) Both ends of the wire agree. alpha derives its own face from
    //    its own table, beta derives it from the copy it synced —
    //    byte-identical output is the whole promise.
    let alpha_here = a
        .devices()
        .unwrap()
        .into_iter()
        .find(|d| d.name == "alpha")
        .expect("alpha knows itself");
    assert_eq!(
        a.face_of(&alpha_here).unwrap(),
        face,
        "the same machine must look the same on both devices"
    );

    serve.abort();
    for r in [&ra, &rb] {
        let _ = std::fs::remove_dir_all(r);
    }
}

/// A device that never reported a style is painted with the factory
/// default — and **not** with the viewer's own choice, which would be
/// the same "each screen paints its own way" bug in a smaller box.
/// **The newcomer's own claim survives pairing, seen from the inviter.**
///
/// This is the direction that used to lose. The other test in this file
/// watches the *inviter's* style travel outward, and that never broke:
/// alpha's row is created by alpha alone. Pairing creates one row twice
/// — the newcomer registers itself at open while the inviter creates a
/// row for it answering `Request::Pair` — so it is **beta's** row, read
/// on **alpha's** side, that the old nested shape blanked. `style` is
/// the only field with exactly one legitimate writer, which is why it is
/// the one that showed the loss; everything else had two writers saying
/// the same thing and looked untouched.
///
/// It is asserted here without anything restating anything, because
/// there is nothing left to restate: `Node::reassert_self` was retired
/// with the flat table (`khor_sync::devices` module head). If this goes
/// red, a row is being lost on creation again and the re-assertion is
/// not coming back to hide it.
#[tokio::test]
async fn a_newcomers_own_style_survives_pairing_on_the_inviters_side() {
    let ra = root("inv");
    let rb = root("new");

    let server = Node::open_as(ra.clone(), "alpha").unwrap();
    let serve = tokio::spawn(async move { server.serve().await });
    wait_for_endpoint_file(&ra).await;

    let a = Node::open_as(ra.clone(), "alpha").unwrap();
    let ticket = a.invite().unwrap();

    // The newcomer picks its face before it has ever been seen, so the
    // claim is already in its table when the inviter creates a row for
    // the same id.
    let b = Node::open_as(rb.clone(), "beta").unwrap();
    b.set_avatar_style(&a_reported_style()).unwrap();

    timeout(Duration::from_secs(15), b.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .unwrap();
    timeout(Duration::from_secs(20), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();

    let devices = a.devices().unwrap();
    let beta = devices
        .iter()
        .find(|d| d.name == "beta")
        .expect("the newcomer must be in the inviter's table at all");
    assert_eq!(
        beta.style.as_deref().and_then(AvatarStyle::from_json),
        Some(a_reported_style()),
        "the newcomer's own claim must survive the row being created twice"
    );

    // …and it reaches the paint, not just the field. Equal to the
    // default-styled face would mean the report arrived and was ignored.
    let seed = AvatarSeed::from_id_hex(&beta.id).unwrap();
    let face = a.face_of(beta).expect("the inviter must be able to paint the newcomer");
    assert_eq!(face, avatar(&seed, &a_reported_style()));
    assert_ne!(
        face,
        avatar(&seed, &AvatarStyle::default()),
        "the newcomer is being painted in the factory palette, not its own"
    );

    serve.abort();
    for r in [&ra, &rb] {
        let _ = std::fs::remove_dir_all(r);
    }
}

/// **One axis moves and the other two stay, and the write reaches the
/// table — not only the file beside it.**
///
/// The two halves fail differently and both look fine on the screen that
/// made the change. A `restyle` that rebuilt the style from the defaults
/// plus the named axis would quietly undo the last two choices. A
/// `restyle` that wrote only the local preference would leave this
/// machine wearing one face here and its old one on every other device
/// — the split `Node::set_avatar_style` exists to prevent, and the one
/// nobody looking at this screen can see.
#[test]
fn restyling_one_axis_leaves_the_others_and_reaches_the_table() {
    let r = root("restyle");
    let n = Node::open_as(r.clone(), "solo").unwrap();
    n.set_avatar_style(&a_reported_style()).unwrap();

    let before = a_reported_style();
    let after = n.restyle(None, None, Some("rounded")).expect("rounded is a shape");
    assert_eq!(after.shape, FaceShape::Rounded);
    assert_eq!(after.variant, before.variant, "the variant was not named and must not move");
    assert_eq!(after.palette, before.palette, "the palette was not named and must not move");

    // The table, read through the node that made the change — this
    // instance was opened *before* the restyle, so its own registration
    // wrote the old style and cannot be what satisfies this.
    let devices = n.devices().unwrap();
    let me = devices
        .iter()
        .find(|d| d.id == n.device_str())
        .expect("this machine is in its own table");
    assert_eq!(
        me.style.as_deref().and_then(AvatarStyle::from_json),
        Some(after.clone()),
        "a restyle that never reaches the table is a face only this machine can see"
    );

    // …and it outlives the process, which is the other half of "chosen".
    assert_eq!(Node::open_as(r.clone(), "solo").unwrap().avatar_style(), after);

    let _ = std::fs::remove_dir_all(&r);
}

/// **An unknown option is refused by name and nothing is written.**
///
/// The second half is the one worth a test. Validation all happens
/// before the write, so a good palette named alongside a bad variant
/// leaves the palette alone; validating as it goes would land the
/// palette and then report that the change failed — a face nobody chose,
/// on a screen that just said nothing happened.
#[test]
fn an_unknown_option_is_refused_and_changes_nothing() {
    let r = root("refuse");
    let n = Node::open_as(r.clone(), "solo").unwrap();
    n.set_avatar_style(&a_reported_style()).unwrap();

    assert!(n.restyle(None, Some("holodeck"), None).is_err(), "an unknown variant is a variant");
    assert!(n.restyle(None, None, Some("holodeck")).is_err(), "an unknown shape is a shape");
    let short = vec!["#f8f8d6".to_owned()];
    assert!(n.restyle(Some(&short), None, None).is_err(), "one slot is a palette");

    // The half-written case.
    let good: Vec<String> = preset("liquid").unwrap().colors.iter().map(|c| c.to_string()).collect();
    assert!(n.restyle(Some(&good), Some("holodeck"), None).is_err());
    assert_eq!(
        n.avatar_style(),
        a_reported_style(),
        "a refused restyle wrote its palette on the way out"
    );

    // The control: the same call with a real key does take, so the
    // refusals above are not "restyle never writes anything".
    n.restyle(Some(&good), Some("beam"), None).expect("beam is a variant");
    assert_ne!(n.avatar_style(), a_reported_style());

    let _ = std::fs::remove_dir_all(&r);
}

/// **A preview is the picture the row will show.**
///
/// `Node::face_under` exists so a chooser can paint what a style *would*
/// look like, and the one thing that has to be true of it is that it is
/// not a second painter. The preview is the only evidence anybody has
/// before pressing, so a preview derived any other way is a choice made
/// on a picture that never appears.
#[test]
fn a_preview_is_the_same_picture_the_row_will_show() {
    let r = root("preview");
    let n = Node::open_as(r.clone(), "solo").unwrap();
    let style = a_reported_style();

    // Painted before it is chosen…
    let previewed = n.face_under(&style);
    // …then chosen, and read back the way a list reads it.
    n.restyle(
        Some(&style.palette.colors().to_vec()),
        Some(style.variant.key()),
        Some(style.shape.key()),
    )
    .expect("the fixture's own keys must be pickable");
    let devices = n.devices().unwrap();
    let me = devices.iter().find(|d| d.id == n.device_str()).unwrap();
    assert_eq!(
        n.face_of(me).expect("this machine has a face"),
        previewed,
        "the preview and the face it turned into are two different pictures"
    );

    // The control: a preview of a different style is a different
    // picture, or the line above holds for a painter that ignores what
    // it is handed.
    assert_ne!(n.face_under(&AvatarStyle::default()), previewed);

    let _ = std::fs::remove_dir_all(&r);
}

#[tokio::test]
async fn an_unreported_style_falls_back_to_the_factory_default() {
    let r = root("fallback");
    let n = Node::open_as(r.clone(), "solo").unwrap();
    // This viewer has a non-default style of its own, so "default" and
    // "whatever the viewer likes" are distinguishable answers.
    n.set_avatar_style(&a_reported_style()).unwrap();

    // A peer known by id and name only, exactly as one arrives through
    // someone else's table.
    let quiet = "cd".repeat(32);
    {
        use khor_sync::devices::{devices_dir, DeviceDoc};
        use khor_sync::store::load;
        let loaded = load::<DeviceDoc>(&devices_dir(&r), 0x99).unwrap();
        loaded.doc.upsert(&quiet, "quiet", &[]).unwrap();
        let mut store = loaded.store;
        store.flush(&loaded.doc).unwrap();
    }

    let devices = n.devices().unwrap();
    let d = devices.iter().find(|d| d.name == "quiet").unwrap();
    assert_eq!(d.style, None, "the premise: this device reported nothing");

    let seed = AvatarSeed::from_id_hex(&quiet).unwrap();
    let face = n.face_of(d).expect("a silent device still gets a face");
    assert_eq!(face, avatar(&seed, &AvatarStyle::default()));
    assert_ne!(
        face,
        avatar(&seed, &a_reported_style()),
        "a silent device must not inherit the viewer's own style"
    );

    let _ = std::fs::remove_dir_all(&r);
}
