//! Real-connection acceptance for faces the device table carries: a
//! machine's face is **the palette its row holds times its id**, and the
//! palette half has to survive the trip.
//!
//! **「self-reported」 until 2026-09-21.** Anyone may write anyone's
//! style now (`khor_sync::devices`, same rule as a rename), so a row
//! carries a choice rather than a claim — and a machine nobody has
//! styled carries nothing at all.
//!
//! Two nodes on one machine, real UDP, real pairing, real sync — and the
//! control group that makes the result mean something: beta must paint
//! alpha differently from how it paints itself, on the same screen, from
//! the same code path. Every await is under a timeout.

//!
//! # The clock here is issue #73's, not this file's subject
//!
//! Every deadline below is sized to swallow one full first-touch stall
//! (#73: 20-40s per process on this machine — the comment in
//! `wait_for_endpoint_file` has the mechanism). **Uniformly**, because
//! which call pays it is not deterministic: measured 2026-08-21, a bind
//! cost 21ms and a dial 1.1s while the first real sync cost 37s. The
//! runtime is multi-thread for the other half of #73: that stall is
//! *synchronous*, and on a current-thread runtime it freezes tokio's
//! clock, so every timeout in the file silently stretches with it and a
//! real hang stops looking like one.

mod util;

use std::path::Path;
use std::time::Duration;

use khor_node::{AvatarSeed, AvatarStyle, FaceShape, Node, avatar, preset};
use tokio::time::timeout;

/// A scratch tree that **takes itself away however the test leaves**
/// (`crates/node/src/tests.rs::Scratch` has the whole reasoning: a
/// cleanup written at the end of a test is skipped by the assertion
/// that fails, so a red run leaks a directory every time it is red).
/// A copy per file, which is this repo's convention for test helpers —
/// it costs no dev-dependency.
struct Scratch(std::path::PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

impl std::ops::Deref for Scratch {
    type Target = std::path::Path;

    fn deref(&self) -> &std::path::Path {
        &self.0
    }
}

fn root(tag: &str) -> Scratch {
    let p = std::env::temp_dir().join(format!("khor-face-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    util::hermetic(&p);
    Scratch(p)
}

async fn wait_for_endpoint_file(root: &Path) {
    // 90s, not 10 (issue #73, sampled 2026-08-20; every real-connection
    // test in this directory timed out for it on 2026-08-21). On a Mac
    // operated over ssh, a freshly compiled test binary is a new face to
    // macOS: its first network-stack touch (interface enumeration,
    // SystemConfiguration) hangs **20-40s per process**, waiting on an
    // authorization prompt nobody can ever click.
    //
    // What the budgets in this file are distinguishing themselves from:
    // a real hang, which is unbounded. 90s against a measured 40s
    // ceiling is more than double, deliberately — a timeout that never
    // fires costs nothing when the test passes, and a flaky red costs a
    // person a trip to find out it was the machine.
    let path = root.join(".khor").join("endpoint.json");
    timeout(Duration::from_secs(90), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("serve should write endpoint.json");
}

/// A style that is **not** the factory default in all three of its
/// parts, so "somebody's choice arrived" cannot be confused with "it
/// fell back to the default and happened to match".
///
/// **Named for a choice rather than a report since 2026-09-21**: a style
/// is not a thing a machine says about itself any more, it is a row in
/// the device table that anyone may write (`khor_sync::devices`).
fn a_chosen_style() -> AvatarStyle {
    let json = r##"{
        "palette": ["#e69f00", "#009e4f", "#f0e442", "#d55e00", "#cc79a7"],
        "variant": "bauhaus",
        "shape": "square"
    }"##;
    let s = AvatarStyle::from_json(json).expect("the fixture must parse");
    assert_ne!(
        s,
        AvatarStyle::default(),
        "the fixture has to differ from the default"
    );
    s
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_machine_is_painted_in_the_palette_the_table_carries_for_it() {
    let ra = root("a");
    let rb = root("b");

    // The two doors onto one identity agree: the bytes iroh hands us and
    // the hex the device table stores seed the same face. Nothing else
    // checks that these two encodings match, and if they ever drifted
    // the symptom would be one machine with two faces.
    let a = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    assert_eq!(
        AvatarSeed::of(&a.device()),
        AvatarSeed::from_id_hex(a.device_str()).expect("the table's id is hex"),
        "the id in bytes and the id in hex must seed the same face"
    );

    // alpha chooses a style before anyone can see it.
    a.set_avatar_style(&a_chosen_style()).unwrap();
    assert_eq!(
        a.avatar_style(),
        a_chosen_style(),
        "the choice must persist locally"
    );

    let server = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let serve = tokio::spawn(async move { server.serve().await });
    wait_for_endpoint_file(&ra).await;

    let ticket = a.invite().unwrap();
    let b = Node::open_as(rb.to_path_buf(), "beta").unwrap();
    timeout(Duration::from_secs(60), b.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .unwrap();
    timeout(Duration::from_secs(60), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();

    // 1) beta's table carries the style alpha's row was given.
    let devices = b.devices().unwrap();
    let alpha = devices
        .iter()
        .find(|d| d.name == "alpha")
        .expect("alpha should be in the table");
    let chosen = alpha
        .style
        .as_deref()
        .and_then(AvatarStyle::from_json)
        .expect("alpha's style should have travelled and should parse");
    assert_eq!(chosen, a_chosen_style(), "the style must arrive unchanged");

    // 2) …and it reaches the paint. The control is the same face
    //    derived with the default style: equal here would mean the
    //    choice travelled and was then ignored, which every assertion
    //    above would still pass.
    let face = b.face_of(alpha).expect("beta must be able to paint alpha");
    let seed = AvatarSeed::from_id_hex(&alpha.id).unwrap();
    assert_ne!(
        face,
        avatar(&seed, &AvatarStyle::default()),
        "alpha is being painted with the default palette, not the one its row carries"
    );
    assert_eq!(
        face,
        avatar(&seed, &chosen),
        "the painted face is the chosen style's"
    );

    // 3) On one screen, beta paints alpha in alpha's palette and itself
    //    in its own. Before the table carried style at all, it was
    //    stored per viewer and the same machine was two colors on two
    //    screens with neither side wrong.
    //
    //    **beta's row carries no style, and that is the assertion**
    //    (2026-09-21): `register_self` stopped pushing one, because a
    //    machine that pushed its own on every boot would take its face
    //    back from whoever last set it. An unwritten row is painted with
    //    the factory default — a fallback, not a report, and the
    //    difference is that a fallback overwrites nobody.
    let beta = devices
        .iter()
        .find(|d| d.name == "beta")
        .expect("beta should be in the table");
    assert_eq!(
        beta.style, None,
        "a machine nobody has styled carries no style of its own"
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
        avatar(&seed, &chosen),
        avatar(&beta_seed, &chosen),
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
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_newcomers_own_style_survives_pairing_on_the_inviters_side() {
    let ra = root("inv");
    let rb = root("new");

    let server = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let serve = tokio::spawn(async move { server.serve().await });
    wait_for_endpoint_file(&ra).await;

    let a = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let ticket = a.invite().unwrap();

    // The newcomer picks its face before it has ever been seen, so the
    // claim is already in its table when the inviter creates a row for
    // the same id.
    let b = Node::open_as(rb.to_path_buf(), "beta").unwrap();
    b.set_avatar_style(&a_chosen_style()).unwrap();

    timeout(Duration::from_secs(60), b.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .unwrap();
    timeout(Duration::from_secs(60), b.sync_now())
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
        Some(a_chosen_style()),
        "the newcomer's own claim must survive the row being created twice"
    );

    // …and it reaches the paint, not just the field. Equal to the
    // default-styled face would mean the report arrived and was ignored.
    let seed = AvatarSeed::from_id_hex(&beta.id).unwrap();
    let face = a
        .face_of(beta)
        .expect("the inviter must be able to paint the newcomer");
    assert_eq!(face, avatar(&seed, &a_chosen_style()));
    assert_ne!(
        face,
        avatar(&seed, &AvatarStyle::default()),
        "the newcomer is being painted in the factory palette, not its own"
    );

    serve.abort();
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
    let n = Node::open_as(r.to_path_buf(), "solo").unwrap();
    n.set_avatar_style(&a_chosen_style()).unwrap();

    let before = a_chosen_style();
    let after = n
        .restyle(None, None, Some("rounded"))
        .expect("rounded is a shape");
    assert_eq!(after.shape, FaceShape::Rounded);
    assert_eq!(
        after.variant, before.variant,
        "the variant was not named and must not move"
    );
    assert_eq!(
        after.palette, before.palette,
        "the palette was not named and must not move"
    );

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
    assert_eq!(
        Node::open_as(r.to_path_buf(), "solo")
            .unwrap()
            .avatar_style(),
        after
    );
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
    let n = Node::open_as(r.to_path_buf(), "solo").unwrap();
    n.set_avatar_style(&a_chosen_style()).unwrap();

    assert!(
        n.restyle(None, Some("holodeck"), None).is_err(),
        "an unknown variant is a variant"
    );
    assert!(
        n.restyle(None, None, Some("holodeck")).is_err(),
        "an unknown shape is a shape"
    );
    let short = vec!["#f8f8d6".to_owned()];
    assert!(
        n.restyle(Some(&short), None, None).is_err(),
        "one slot is a palette"
    );

    // The half-written case.
    let good: Vec<String> = preset("liquid")
        .unwrap()
        .colors
        .iter()
        .map(|c| c.to_string())
        .collect();
    assert!(n.restyle(Some(&good), Some("holodeck"), None).is_err());
    assert_eq!(
        n.avatar_style(),
        a_chosen_style(),
        "a refused restyle wrote its palette on the way out"
    );

    // The control: the same call with a real key does take, so the
    // refusals above are not "restyle never writes anything".
    n.restyle(Some(&good), Some("beam"), None)
        .expect("beam is a variant");
    assert_ne!(n.avatar_style(), a_chosen_style());
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
    let n = Node::open_as(r.to_path_buf(), "solo").unwrap();
    let style = a_chosen_style();

    // Painted before it is chosen…
    let previewed = n.face_under(&style);
    // …then chosen, and read back the way a list reads it.
    n.restyle(
        Some(style.palette.colors().as_ref()),
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
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_unreported_style_falls_back_to_the_factory_default() {
    let r = root("fallback");
    let n = Node::open_as(r.to_path_buf(), "solo").unwrap();
    // This viewer has a non-default style of its own, so "default" and
    // "whatever the viewer likes" are distinguishable answers.
    n.set_avatar_style(&a_chosen_style()).unwrap();

    // A peer known by id and name only, exactly as one arrives through
    // someone else's table.
    let quiet = "cd".repeat(32);
    {
        use khor_sync::devices::{DeviceDoc, devices_dir};
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
        avatar(&seed, &a_chosen_style()),
        "a silent device must not inherit the viewer's own style"
    );
}

/// **A face somebody else gave this machine survives its restart.**
///
/// This is the guard on the one behaviour that changed when `style`
/// stopped being self-reported (2026-09-21, 「头像设置这, 要支持对每台
/// 机器的修改」). `Node::register_self` used to push `.khor/avatar.json`
/// into the device table on *every* open — which, once anybody may
/// restyle anybody, is this machine quietly taking its own face back
/// once per start, against a write that was the network's.
///
/// **The arm is the leftover file**, and it is what makes this test
/// discriminate. Setting a style and reading it back would pass either
/// way, because the old code kept file and table in step. So the table
/// is given one style and the dead file a *different* one by hand —
/// exactly the state an older khor on the same home leaves behind. If
/// the table is the only source the file is ignored; if anything still
/// reads it, the restart reverts the face and this goes red naming both.
///
/// No network: the claim is about what one machine does to its own row
/// when it opens, and a real peer's write lands in that same register by
/// construction (one field, whole-value LWW).
#[test]
fn a_face_another_machine_gave_this_one_survives_a_restart() {
    let r = root("kept");

    let chosen = {
        let n = Node::open_as(r.to_path_buf(), "alpha").unwrap();
        // Stands for 「beta restyled alpha and it synced here」: the same
        // register a peer writes, written through the same call.
        n.set_avatar_style(&a_chosen_style()).unwrap();
        assert_eq!(n.avatar_style(), a_chosen_style(), "the table took it");
        n.avatar_style()
    };

    // The dead file, wearing a style that is neither the table's nor the
    // factory default — so a revert to *either* is visible.
    let leftover = AvatarStyle::from_json(
        r##"{
            "palette": ["#111111", "#222222", "#333333", "#444444", "#555555"],
            "variant": "beam",
            "shape": "circle"
        }"##,
    )
    .expect("the fixture must parse");
    assert_ne!(
        leftover, chosen,
        "control: the leftover has to differ from the table's"
    );
    std::fs::write(
        r.join(".khor").join("avatar.json"),
        leftover.to_json().unwrap(),
    )
    .unwrap();

    // The restart.
    let again = Node::open_as(r.to_path_buf(), "alpha").unwrap();
    assert_eq!(
        again.avatar_style(),
        chosen,
        "a restart must not take the face back from whoever set it; the table is \
         the only source and `.khor/avatar.json` is dead"
    );
}

/// A failed read is not an instruction to replace the other axes with
/// factory defaults. Preserve the exact stored value on refusal.
#[test]
fn avatar_partial_edit_refuses_invalid_saved_style() {
    let r = root("invalid-edit");
    let n = Node::open_as(r.to_path_buf(), "solo").unwrap();
    let loaded = khor_sync::store::load::<khor_sync::devices::DeviceDoc>(
        &r.join(".khor/devices"), 73,
    ).unwrap();
    let invalid = r##"{"variant":"future-style","palette":["#123456"]}"##;
    loaded.doc.set_style(n.device_str(), invalid).unwrap();
    let mut store = loaded.store;
    store.flush(&loaded.doc).unwrap();
    drop(store);
    assert!(n.restyle(None, None, Some("rounded")).is_err(),
        "an unreadable chosen face must not be replaced by defaults plus one edited axis");
    assert_eq!(n.devices().unwrap().into_iter().find(|d| d.id == n.device_str()).unwrap().style.as_deref(), Some(invalid));
}

#[test]
fn avatar_partial_edit_refuses_a_damaged_device_block() {
    let r = root("damaged-edit");
    let n = Node::open_as(r.to_path_buf(), "solo").unwrap();
    n.set_avatar_style(&a_chosen_style()).unwrap();
    std::fs::write(r.join(".khor/devices/u-broken.loro"), b"not a loro block").unwrap();
    assert!(n.restyle(None, None, Some("rounded")).is_err(),
        "a partial document must not be used as the base for an avatar edit");
    assert_eq!(n.avatar_style(), a_chosen_style());
}

#[test]
fn avatar_edit_refuses_a_missing_machine() {
    let r = root("missing-edit");
    let n = Node::open_as(r.to_path_buf(), "solo").unwrap();
    assert!(n.restyle_device(&"a".repeat(64), None, Some("beam"), None).is_err(),
        "an unknown device must not acquire a default style row through editing");
    assert_eq!(n.devices().unwrap().len(), 1);
}

#[test]
fn avatar_checked_read_distinguishes_unset_from_failure() {
    let r = root("checked-read");
    let n = Node::open_as(r.to_path_buf(), "solo").unwrap();
    assert!(n.checked_avatar_style_of(n.device_str()).unwrap().is_none());
    assert_eq!(n.restyle(None, None, None).unwrap(), AvatarStyle::default());
    assert!(n.checked_avatar_style_of(n.device_str()).unwrap().is_none(), "an edit with no axes must not invent a persisted choice");
    let chosen = n.restyle(None, Some("beam"), None).unwrap();
    assert_eq!(chosen.variant.key(), "beam");
    let path = r.join(".khor/devices");
    let held = r.join(".khor/devices-held");
    std::fs::rename(&path, &held).unwrap();
    std::fs::write(&path, b"not a directory").unwrap();
    assert!(n.checked_avatar_style_of(n.device_str()).is_err());
    assert!(n.restyle(None, None, Some("rounded")).is_err());
    std::fs::remove_file(&path).unwrap();
    std::fs::rename(&held, &path).unwrap();
    assert_eq!(n.checked_avatar_style_of(n.device_str()).unwrap(), Some(chosen));
}

#[test]
fn avatar_change_provenance_stays_with_the_choice_and_noop_keeps_it() {
    let r = root("provenance");
    let n = Node::open_as(r.to_path_buf(), "solo").unwrap();
    let before = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
    let chosen = n.restyle_device_from(n.device_str(), None, Some("beam"), None, khor_node::AvatarChangeOrigin::Gui).unwrap();
    let raw = || n.devices().unwrap().into_iter().find(|d| d.id == n.device_str()).unwrap().style.unwrap();
    let initial = raw();
    let value: serde_json::Value = serde_json::from_str(&initial).unwrap();
    assert_eq!(value["change"]["writer"], n.device_str());
    assert_eq!(value["change"]["origin"], "gui");
    assert!(value["change"]["at_ms"].as_u64().unwrap() >= before);
    // Old readers still consume exactly the same style fields.
    assert_eq!(AvatarStyle::from_json(&initial), Some(chosen.clone()));
    let directory_state = || {
        let mut files: Vec<_> = std::fs::read_dir(r.join(".khor/devices")).unwrap().flatten()
            .map(|e| e.path()).filter(|p| p.extension().is_some_and(|ext| ext == "loro")).collect();
        files.sort();
        files.into_iter().map(|p| (p.file_name().unwrap().to_owned(), std::fs::read(&p).unwrap())).collect::<Vec<_>>()
    };
    let blocks_before = directory_state();
    n.restyle_device_from(n.device_str(), None, Some("beam"), None, khor_node::AvatarChangeOrigin::Cli).unwrap();
    assert_eq!(raw(), initial, "a no-op must not pretend the CLI changed the GUI's choice");
    assert_eq!(directory_state(), blocks_before, "a no-op must not append sync blocks");
    let restarted = Node::open_as(r.to_path_buf(), "solo").unwrap();
    assert_eq!(restarted.checked_avatar_style_of(n.device_str()).unwrap(), Some(chosen));
    let after_restart = restarted.devices().unwrap().into_iter().find(|d| d.id == n.device_str()).unwrap().style.unwrap();
    assert_eq!(after_restart, initial, "restart must preserve both choice and provenance");
}
