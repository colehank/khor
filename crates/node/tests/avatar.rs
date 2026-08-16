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

use khor_node::{avatar, AvatarSeed, AvatarStyle, Node};
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
