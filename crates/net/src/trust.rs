//! Device certificates and the roster, signed by the user's root key
//! (docs/CONNECT.md 身份：用户根钥匙, 分期 2).
//!
//! # The two records
//!
//! - A **certificate** says 「subject X is mine」: `{root, subject, kind,
//!   sponsor?, issued_ms, seq, expires_ms?}` plus the root's signature.
//!   `subject` is a device id, or a satellite's ssh host-key fingerprint
//!   (same 32-byte shape, docs/NET.md 卫星).
//! - The **roster** says 「these are all mine, and these are revoked」:
//!   `{root, seq, issued_ms, certs: [signed cert…], revoked: [id…]}` plus
//!   the root's signature. **It is the one source of trust**
//!   (CONNECT.md 名单是信任的唯一出处): a machine the device table holds
//!   but the roster does not list — or lists as revoked — does not count.
//!
//! Both verify with **nothing but the root's public key** — [`verify_cert`]
//! and [`verify_roster`] are the whole interface a relay, a billing
//! service or a team needs to check 「this device belongs to user U」
//! offline (CONNECT.md 网外的第三方可验).
//!
//! # The bytes
//!
//! ```text
//! magic(4) | body_len(u32, big-endian) | body (postcard) | signature(64)
//! ```
//!
//! The signature covers **everything before it**, magic and length
//! included, so flipping any byte anywhere is refused: before the
//! signature it breaks the signature, inside it the signature itself,
//! and a length that does not account for every byte is refused as
//! garbled before anything is verified. The magic differs between the
//! two records, so a certificate's signature can never be replayed as a
//! roster's. Stored everywhere as opaque bytes; nothing but this module
//! reads inside them.
//!
//! # Which roster wins
//!
//! The highest `seq` that verifies — [`newest`]. The device table's
//! last-writer-wins never enters the question: a stolen machine can
//! write an old roster into any document it likes, and every reader
//! still takes the newest signed one it has seen.
//!
//! # Reserved for sharing, not built
//!
//! CONNECT.md 为共享预留: a certificate may one day name a user or a
//! team, carry rights and an end date. [`Kind`] is an enum a later
//! variant extends, and `expires_ms` is already honoured. This phase
//! issues `Device` and `Satellite` only, with no end.

use serde::{Deserialize, Serialize};

use khor_catalog::msg;

/// A roster older than this is **stale** (CONNECT.md 撤销要新鲜): the
/// machine keeps trusting what the old roster already verified, accepts
/// no newly presented certificate, and `khor doctor` says how stale.
///
/// Thirty days, the number the lead set to start with; CONNECT.md 待定
/// keeps the value open. It only bites while the root holder is away
/// that long — the holder re-signs every [`REFRESH_DAYS`].
pub const FRESH_DAYS: u64 = 30;

/// How old the roster may get before the root holder signs the same
/// list again with a new date. Well inside [`FRESH_DAYS`], so a holder
/// that is online now and then keeps the whole network fresh without
/// anybody doing anything.
pub const REFRESH_DAYS: u64 = 7;

pub const DAY_MS: u64 = 86_400_000;

const CERT_MAGIC: &[u8; 4] = b"khc1";
const ROSTER_MAGIC: &[u8; 4] = b"khr1";
const SIG: usize = 64;
const HEAD: usize = 8;

/// A device id, a satellite's fingerprint, or a root public key: 32 bytes.
pub type Id = [u8; 32];

/// What a certificate's subject is.
///
/// An enum a later phase extends (a user, a team — CONNECT.md 为共享预留).
/// A newer variant is a record an older khor refuses as garbled, which
/// is the right failure: it cannot know what the new kind grants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    /// A machine running khor, which dials and is dialed with its key.
    Device,
    /// A machine only ssh reaches; `sponsor` speaks for it.
    Satellite,
}

impl Kind {
    /// The stable key a face keys off.
    pub fn key(self) -> &'static str {
        match self {
            Kind::Device => "device",
            Kind::Satellite => "satellite",
        }
    }
}

/// One signed statement: 「`subject` is mine」.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cert {
    /// The root that signed it — repeated inside so a certificate read on
    /// its own says whose it is.
    pub root: Id,
    pub subject: Id,
    pub kind: Kind,
    /// The device that speaks for a satellite; `None` for a device.
    pub sponsor: Option<Id>,
    pub issued_ms: u64,
    /// The roster `seq` it was issued with.
    pub seq: u64,
    /// When it stops counting; `None` is no end (the only value issued in
    /// this phase).
    pub expires_ms: Option<u64>,
}

impl Cert {
    /// Whether it still counts at `now_ms`.
    pub fn live_at(&self, now_ms: u64) -> bool {
        self.expires_ms.is_none_or(|end| now_ms < end)
    }
}

#[derive(Serialize, Deserialize)]
struct RosterBody {
    root: Id,
    seq: u64,
    issued_ms: u64,
    certs: Vec<Vec<u8>>,
    revoked: Vec<Id>,
}

/// A roster whose signature and every certificate inside it verified.
///
/// Only [`verify_roster`] makes one, so holding one is the proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Roster {
    pub root: Id,
    pub seq: u64,
    pub issued_ms: u64,
    pub certs: Vec<Cert>,
    /// The signed certificates, in `certs`' order — handed out as they
    /// are, so a device can present its own to anyone.
    signed: Vec<Vec<u8>>,
    pub revoked: Vec<Id>,
    bytes: Vec<u8>,
}

/// Why a record was refused. Each one says which kind of wrong it is,
/// because each has a different next step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrustError {
    /// Not a record at all: wrong magic, a length that does not add up,
    /// a body that does not decode.
    Garbled,
    /// A record whose signature does not verify under the root.
    BadSignature,
    /// A well-formed record that says it belongs to a different root.
    OtherRoot,
    /// A roster that carries a certificate which does not verify, or
    /// that claims a later `seq` than the roster itself.
    BadCertInside,
}

impl std::fmt::Display for TrustError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            TrustError::Garbled => msg::trust_garbled(),
            TrustError::BadSignature => msg::trust_bad_signature(),
            TrustError::OtherRoot => msg::trust_other_root(),
            TrustError::BadCertInside => msg::trust_bad_cert_inside(),
        })
    }
}

impl std::error::Error for TrustError {}

/// 64 hex digits for an id — the spelling device ids already travel in.
pub fn id_hex(id: &Id) -> String {
    id.iter().map(|b| format!("{b:02x}")).collect()
}

/// An id back from 64 hex digits; `None` for anything else.
pub fn id_from_hex(s: &str) -> Option<Id> {
    if s.len() != 64 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
        let hi = (chunk[0] as char).to_digit(16)?;
        let lo = (chunk[1] as char).to_digit(16)?;
        out[i] = (hi * 16 + lo) as u8;
    }
    Some(out)
}

fn seal(magic: &[u8; 4], body: &[u8], key: &iroh::SecretKey) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEAD + body.len() + SIG);
    out.extend_from_slice(magic);
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    out.extend_from_slice(body);
    let sig = key.sign(&out);
    out.extend_from_slice(&sig.to_bytes());
    out
}

/// A record split into its body, the prefix the signature covers, and
/// the signature.
type Framed<'a> = (&'a [u8], &'a [u8], [u8; SIG]);

/// The body and the signed prefix, **unverified** — the framing checks
/// only. Every byte must be accounted for.
fn frame<'a>(magic: &[u8; 4], bytes: &'a [u8]) -> Result<Framed<'a>, TrustError> {
    if bytes.len() < HEAD + SIG || &bytes[..4] != magic {
        return Err(TrustError::Garbled);
    }
    let len = u32::from_be_bytes(bytes[4..HEAD].try_into().expect("four bytes")) as usize;
    if bytes.len() != HEAD + len + SIG {
        return Err(TrustError::Garbled);
    }
    let signed = &bytes[..HEAD + len];
    let sig: [u8; SIG] = bytes[HEAD + len..].try_into().expect("sixty-four bytes");
    Ok((&bytes[HEAD..HEAD + len], signed, sig))
}

/// A postcard body that must be consumed exactly.
fn decode<'a, T: Deserialize<'a>>(body: &'a [u8]) -> Result<T, TrustError> {
    match postcard::take_from_bytes::<T>(body) {
        Ok((v, [])) => Ok(v),
        _ => Err(TrustError::Garbled),
    }
}

fn check_sig(root: &iroh::PublicKey, signed: &[u8], sig: &[u8; SIG]) -> Result<(), TrustError> {
    root.verify(signed, &iroh::Signature::from_bytes(sig))
        .map_err(|_| TrustError::BadSignature)
}

/// Signs one certificate.
pub fn sign_cert(key: &iroh::SecretKey, cert: &Cert) -> Vec<u8> {
    let body = postcard::to_allocvec(cert).expect("a certificate always encodes");
    seal(CERT_MAGIC, &body, key)
}

/// A certificate, **verified with nothing but the root's public key**.
///
/// The interface a relay admits devices by (CONNECT.md `khor relay`
/// 按证书放行): no device table, no network, no clock. Whether it has
/// been revoked is the roster's to say, and so is whether it has
/// expired at a given moment ([`Cert::live_at`]).
pub fn verify_cert(bytes: &[u8], root: &iroh::PublicKey) -> Result<Cert, TrustError> {
    let (body, signed, sig) = frame(CERT_MAGIC, bytes)?;
    let cert: Cert = decode(body)?;
    if &cert.root != root.as_bytes() {
        return Err(TrustError::OtherRoot);
    }
    check_sig(root, signed, &sig)?;
    Ok(cert)
}

/// A roster, **verified with nothing but the root's public key** — its
/// own signature and every certificate inside it.
pub fn verify_roster(bytes: &[u8], root: &iroh::PublicKey) -> Result<Roster, TrustError> {
    let (body, signed, sig) = frame(ROSTER_MAGIC, bytes)?;
    let b: RosterBody = decode(body)?;
    if &b.root != root.as_bytes() {
        return Err(TrustError::OtherRoot);
    }
    check_sig(root, signed, &sig)?;
    let mut certs = Vec::with_capacity(b.certs.len());
    for c in &b.certs {
        let cert = verify_cert(c, root).map_err(|_| TrustError::BadCertInside)?;
        if cert.seq > b.seq {
            return Err(TrustError::BadCertInside);
        }
        certs.push(cert);
    }
    Ok(Roster {
        root: b.root,
        seq: b.seq,
        issued_ms: b.issued_ms,
        certs,
        signed: b.certs,
        revoked: b.revoked,
        bytes: bytes.to_vec(),
    })
}

/// The root a roster **claims**, read without verifying anything.
///
/// For the one moment a machine has no root to check against yet: the
/// other side of a pairing hands over its roster, and the question is
/// which key to verify it with. The answer is then used *to verify*, so
/// a lie here only produces a roster that fails ([`verify_roster`]).
pub fn claimed_root(bytes: &[u8]) -> Option<iroh::PublicKey> {
    let (body, _, _) = frame(ROSTER_MAGIC, bytes).ok()?;
    let b: RosterBody = decode(body).ok()?;
    iroh::PublicKey::from_bytes(&b.root).ok()
}

/// The newest roster among `candidates` that verifies under `root`.
///
/// **Highest `seq` wins**, never the order they arrived in; a tie (two
/// holders of one root signing at once) is broken by the later
/// `issued_ms`, then by the bytes, so every machine picks the same one.
pub fn newest<'a>(
    candidates: impl IntoIterator<Item = &'a [u8]>,
    root: &iroh::PublicKey,
) -> Option<Roster> {
    candidates
        .into_iter()
        .filter_map(|b| verify_roster(b, root).ok())
        .max_by(|a, b| {
            (a.seq, a.issued_ms, &a.bytes).cmp(&(b.seq, b.issued_ms, &b.bytes))
        })
}

fn sign_roster(
    key: &iroh::SecretKey,
    seq: u64,
    issued_ms: u64,
    certs: Vec<Vec<u8>>,
    revoked: Vec<Id>,
) -> Vec<u8> {
    let body = RosterBody {
        root: *key.public().as_bytes(),
        seq,
        issued_ms,
        certs,
        revoked,
    };
    seal(
        ROSTER_MAGIC,
        &postcard::to_allocvec(&body).expect("a roster always encodes"),
        key,
    )
}

/// One member to certify: who, what it is, and who speaks for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Member {
    pub subject: Id,
    pub kind: Kind,
    pub sponsor: Option<Id>,
}

impl Member {
    pub fn device(id: Id) -> Member {
        Member {
            subject: id,
            kind: Kind::Device,
            sponsor: None,
        }
    }
}

fn cert_for(key: &iroh::SecretKey, m: &Member, seq: u64, now_ms: u64) -> Vec<u8> {
    sign_cert(
        key,
        &Cert {
            root: *key.public().as_bytes(),
            subject: m.subject,
            kind: m.kind,
            sponsor: m.sponsor,
            issued_ms: now_ms,
            seq,
            expires_ms: None,
        },
    )
}

/// The first roster of a root: `seq` 1, certifying `members`.
pub fn genesis(key: &iroh::SecretKey, now_ms: u64, members: &[Member]) -> Vec<u8> {
    let certs = members.iter().map(|m| cert_for(key, m, 1, now_ms)).collect();
    sign_roster(key, 1, now_ms, certs, Vec::new())
}

impl Roster {
    /// The signed roster, as it travels and is stored.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The root this roster is under, as a key.
    pub fn root_key(&self) -> iroh::PublicKey {
        iroh::PublicKey::from_bytes(&self.root).expect("verified under this key")
    }

    /// The certificate this roster holds for `id`, if any — revoked or
    /// not ([`Roster::admits`] is the trust question).
    pub fn cert(&self, id: &Id) -> Option<&Cert> {
        self.certs.iter().find(|c| &c.subject == id)
    }

    /// The same certificate, signed — what `id` presents to others.
    pub fn signed_cert(&self, id: &Id) -> Option<&[u8]> {
        let i = self.certs.iter().position(|c| &c.subject == id)?;
        Some(&self.signed[i])
    }

    pub fn is_revoked(&self, id: &Id) -> bool {
        self.revoked.contains(id)
    }

    /// **The trust question**: listed, not revoked, not expired.
    pub fn admits(&self, id: &Id, now_ms: u64) -> bool {
        !self.is_revoked(id) && self.cert(id).is_some_and(|c| c.live_at(now_ms))
    }

    /// Whether this roster certifies nobody but `owner` — a network of
    /// one, which is what a machine is between its first start and its
    /// first pairing. [`settle`] lets such a root give way.
    pub fn lone(&self, owner: &Id) -> bool {
        self.certs.iter().all(|c| &c.subject == owner)
    }

    /// Milliseconds since it was signed; a date in the future (a clock
    /// running behind the signer's) reads as zero, not as negative.
    pub fn age_ms(&self, now_ms: u64) -> u64 {
        now_ms.saturating_sub(self.issued_ms)
    }

    /// Whole days since it was signed.
    pub fn age_days(&self, now_ms: u64) -> u64 {
        self.age_ms(now_ms) / DAY_MS
    }

    /// Past [`FRESH_DAYS`] (CONNECT.md 撤销要新鲜).
    pub fn is_stale(&self, now_ms: u64) -> bool {
        self.age_ms(now_ms) > FRESH_DAYS * DAY_MS
    }

    /// Whole days past [`FRESH_DAYS`] when stale — what 「名单已过期 N
    /// 天」 says — and `None` while fresh.
    pub fn stale_days(&self, now_ms: u64) -> Option<u64> {
        self.is_stale(now_ms)
            .then(|| (self.age_ms(now_ms) - FRESH_DAYS * DAY_MS) / DAY_MS)
    }

    /// Past [`REFRESH_DAYS`]: time for the holder to sign it again.
    pub fn needs_refresh(&self, now_ms: u64) -> bool {
        self.age_ms(now_ms) > REFRESH_DAYS * DAY_MS
    }

    fn next(&self, key: &iroh::SecretKey, now_ms: u64, certs: Vec<Vec<u8>>, revoked: Vec<Id>) -> Vec<u8> {
        sign_roster(key, self.seq + 1, now_ms, certs, revoked)
    }

    /// The next roster with `m` certified (replacing any earlier
    /// certificate of the same subject, and taking it off the revoked
    /// list — admitting a machine again is what re-pairing it *is*).
    pub fn admit(&self, key: &iroh::SecretKey, now_ms: u64, m: &Member) -> Vec<u8> {
        let seq = self.seq + 1;
        let mut certs: Vec<Vec<u8>> = self
            .certs
            .iter()
            .zip(&self.signed)
            .filter(|(c, _)| c.subject != m.subject)
            .map(|(_, s)| s.clone())
            .collect();
        certs.push(cert_for(key, m, seq, now_ms));
        let revoked = self.revoked.iter().copied().filter(|r| r != &m.subject).collect();
        self.next(key, now_ms, certs, revoked)
    }

    /// The next roster with `id` revoked: its certificate dropped and its
    /// id listed, so a presented copy of the old certificate is refused
    /// too.
    pub fn revoke(&self, key: &iroh::SecretKey, now_ms: u64, id: &Id) -> Vec<u8> {
        let certs = self
            .certs
            .iter()
            .zip(&self.signed)
            .filter(|(c, _)| &c.subject != id)
            .map(|(_, s)| s.clone())
            .collect();
        let mut revoked = self.revoked.clone();
        if !revoked.contains(id) {
            revoked.push(*id);
        }
        self.next(key, now_ms, certs, revoked)
    }

    /// The same list, signed again with today's date — what keeps a
    /// network fresh while its holder is around ([`REFRESH_DAYS`]).
    pub fn refresh(&self, key: &iroh::SecretKey, now_ms: u64) -> Vec<u8> {
        self.next(key, now_ms, self.signed.clone(), self.revoked.clone())
    }
}

/// What a pairing does with the two sides' roots ([`settle`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Settle {
    /// Keep this side's root; the other side's roster is kept too when it
    /// is under the same root (the newer one then wins by `seq`).
    Keep,
    /// Take the other side's root: this side's own is a network of one
    /// (or there is none), so it gives way.
    Adopt,
    /// Two networks, each with more than one machine, under two roots.
    /// Merging them is not something a pairing may decide.
    Clash,
}

/// How two machines' roots meet when they pair. **One rule, run on both
/// sides with the same inputs**, so both reach the same verdict without
/// a round trip to agree on it.
///
/// - No root on either side (a network from before roots): keep.
/// - A root on one side only: the side without takes it.
/// - The same root: keep.
/// - Two roots: a network of one gives way to a network of more
///   ([`Roster::lone`]). Two networks of one — two fresh machines, each
///   with the root it made at first start — resolve toward **the ticket
///   issuer**: the one who dialed (`i_dialed`) is the one who asked to
///   come in. Two networks of more clash.
pub fn settle(
    mine: Option<&Roster>,
    me: &Id,
    theirs: Option<&Roster>,
    them: &Id,
    i_dialed: bool,
) -> Settle {
    match (mine, theirs) {
        (_, None) => Settle::Keep,
        (None, Some(_)) => Settle::Adopt,
        (Some(m), Some(t)) if m.root == t.root => Settle::Keep,
        (Some(m), Some(t)) => match (m.lone(me), t.lone(them)) {
            (true, false) => Settle::Adopt,
            (false, true) => Settle::Keep,
            (true, true) if i_dialed => Settle::Adopt,
            (true, true) => Settle::Keep,
            (false, false) => Settle::Clash,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_790_000_000_000;

    fn id(n: u8) -> Id {
        [n; 32]
    }

    fn fresh(members: &[Member]) -> (iroh::SecretKey, Roster) {
        let key = iroh::SecretKey::generate();
        let bytes = genesis(&key, NOW, members);
        let r = verify_roster(&bytes, &key.public()).unwrap();
        (key, r)
    }

    #[test]
    fn a_certificate_round_trips_and_verifies_with_only_the_root_key() {
        let key = iroh::SecretKey::generate();
        let cert = Cert {
            root: *key.public().as_bytes(),
            subject: id(7),
            kind: Kind::Satellite,
            sponsor: Some(id(8)),
            issued_ms: NOW,
            seq: 3,
            expires_ms: None,
        };
        let bytes = sign_cert(&key, &cert);
        assert_eq!(verify_cert(&bytes, &key.public()).unwrap(), cert);
    }

    #[test]
    fn a_roster_round_trips() {
        let (key, r) = fresh(&[Member::device(id(1)), Member::device(id(2))]);
        assert_eq!(r.seq, 1);
        assert_eq!(r.root, *key.public().as_bytes());
        assert_eq!(r.certs.len(), 2);
        let again = verify_roster(r.bytes(), &key.public()).unwrap();
        assert_eq!(again, r);
        assert!(r.admits(&id(1), NOW) && r.admits(&id(2), NOW));
        assert!(!r.admits(&id(3), NOW), "a stranger is not admitted");
        // A certificate handed out of the roster verifies on its own.
        let c = verify_cert(r.signed_cert(&id(2)).unwrap(), &key.public()).unwrap();
        assert_eq!(c.subject, id(2));
    }

    #[test]
    fn a_record_signed_by_another_key_is_refused() {
        let (_, r) = fresh(&[Member::device(id(1))]);
        let other = iroh::SecretKey::generate();
        assert_eq!(
            verify_roster(r.bytes(), &other.public()).unwrap_err(),
            TrustError::OtherRoot
        );
        // A forger who puts the right root inside but signs with their
        // own key: the claim matches, the signature does not.
        let body = RosterBody {
            root: r.root,
            seq: 99,
            issued_ms: NOW,
            certs: Vec::new(),
            revoked: Vec::new(),
        };
        let forged = seal(ROSTER_MAGIC, &postcard::to_allocvec(&body).unwrap(), &other);
        assert_eq!(
            verify_roster(&forged, &r.root_key()).unwrap_err(),
            TrustError::BadSignature
        );
        let cert = Cert {
            root: r.root,
            subject: id(9),
            kind: Kind::Device,
            sponsor: None,
            issued_ms: NOW,
            seq: 1,
            expires_ms: None,
        };
        assert_eq!(
            verify_cert(&sign_cert(&other, &cert), &r.root_key()).unwrap_err(),
            TrustError::BadSignature
        );
    }

    /// **Every** byte, not a sample: a flip anywhere — magic, length,
    /// body, signature — must be refused, and never read back as a
    /// different valid record.
    #[test]
    fn any_single_byte_tampered_is_refused() {
        let (key, r) = fresh(&[Member::device(id(1)), Member::device(id(2))]);
        let good = r.bytes().to_vec();
        for i in 0..good.len() {
            for flip in [0x01u8, 0x80, 0xff] {
                let mut bad = good.clone();
                bad[i] ^= flip;
                assert!(
                    verify_roster(&bad, &key.public()).is_err(),
                    "roster byte {i} ^ {flip:#x} was accepted"
                );
            }
        }
        let cert = r.signed_cert(&id(1)).unwrap().to_vec();
        for i in 0..cert.len() {
            let mut bad = cert.clone();
            bad[i] ^= 0x01;
            assert!(
                verify_cert(&bad, &key.public()).is_err(),
                "cert byte {i} was accepted"
            );
        }
        // Truncated and extended are refused too.
        assert_eq!(
            verify_roster(&good[..good.len() - 1], &key.public()).unwrap_err(),
            TrustError::Garbled
        );
        let mut longer = good.clone();
        longer.push(0);
        assert_eq!(
            verify_roster(&longer, &key.public()).unwrap_err(),
            TrustError::Garbled
        );
    }

    /// A certificate's signature cannot stand in for a roster's: the
    /// magic is signed, and it differs.
    #[test]
    fn a_certificate_is_not_a_roster() {
        let (key, r) = fresh(&[Member::device(id(1))]);
        let cert = r.signed_cert(&id(1)).unwrap();
        assert!(verify_roster(cert, &key.public()).is_err());
        assert!(verify_cert(r.bytes(), &key.public()).is_err());
    }

    #[test]
    fn a_roster_carrying_a_foreign_certificate_is_refused() {
        let key = iroh::SecretKey::generate();
        let other = iroh::SecretKey::generate();
        let foreign = cert_for(&other, &Member::device(id(5)), 1, NOW);
        let bytes = sign_roster(&key, 1, NOW, vec![foreign], Vec::new());
        assert_eq!(
            verify_roster(&bytes, &key.public()).unwrap_err(),
            TrustError::BadCertInside
        );
    }

    /// The whole reason `seq` exists: an old roster never replaces a newer
    /// one, whatever order they are read in.
    #[test]
    fn an_older_seq_never_wins_over_a_newer_one() {
        let (key, r1) = fresh(&[Member::device(id(1))]);
        let b2 = r1.admit(&key, NOW + 1, &Member::device(id(2)));
        let r2 = verify_roster(&b2, &key.public()).unwrap();
        let b3 = r2.revoke(&key, NOW + 2, &id(2));
        let r3 = verify_roster(&b3, &key.public()).unwrap();
        assert_eq!((r1.seq, r2.seq, r3.seq), (1, 2, 3));
        let orders: [[&[u8]; 3]; 3] = [
            [r1.bytes(), r2.bytes(), r3.bytes()],
            [r3.bytes(), r1.bytes(), r2.bytes()],
            [r2.bytes(), r3.bytes(), r1.bytes()],
        ];
        for o in orders {
            let got = newest(o, &key.public()).unwrap();
            assert_eq!(got.seq, 3);
            assert!(!got.admits(&id(2), NOW), "the revocation stands");
            assert!(got.is_revoked(&id(2)));
        }
        // Garbage and foreign rosters among the candidates are skipped,
        // not fatal.
        let (_, foreign) = fresh(&[Member::device(id(9))]);
        let got = newest([&b"junk"[..], foreign.bytes(), r2.bytes()], &key.public()).unwrap();
        assert_eq!(got.seq, 2);
    }

    #[test]
    fn admitting_a_revoked_machine_takes_it_off_the_revoked_list() {
        let (key, r1) = fresh(&[Member::device(id(1)), Member::device(id(2))]);
        let r2 = verify_roster(&r1.revoke(&key, NOW, &id(2)), &key.public()).unwrap();
        assert!(!r2.admits(&id(2), NOW));
        let r3 =
            verify_roster(&r2.admit(&key, NOW, &Member::device(id(2))), &key.public()).unwrap();
        assert!(r3.admits(&id(2), NOW));
        assert!(!r3.is_revoked(&id(2)));
        assert_eq!(r3.certs.len(), 2, "one certificate per subject");
        assert!(r3.admits(&id(1), NOW), "the others are untouched");
    }

    #[test]
    fn a_refresh_keeps_the_list_and_moves_the_date() {
        let (key, r1) = fresh(&[Member::device(id(1))]);
        let r1 = verify_roster(&r1.revoke(&key, NOW, &id(4)), &key.public()).unwrap();
        let later = NOW + 8 * DAY_MS;
        assert!(r1.needs_refresh(later));
        let r2 = verify_roster(&r1.refresh(&key, later), &key.public()).unwrap();
        assert_eq!(r2.seq, r1.seq + 1);
        assert_eq!(r2.issued_ms, later);
        assert_eq!(r2.certs, r1.certs);
        assert_eq!(r2.revoked, r1.revoked);
        assert!(!r2.needs_refresh(later));
    }

    /// The clock comes in as an argument: fresh up to the day, stale the
    /// millisecond after, and the days counted past the limit.
    #[test]
    fn staleness_is_counted_on_the_clock_it_is_given() {
        let (_, r) = fresh(&[Member::device(id(1))]);
        let edge = NOW + FRESH_DAYS * DAY_MS;
        assert!(!r.is_stale(edge));
        assert_eq!(r.stale_days(edge), None);
        assert!(r.is_stale(edge + 1));
        assert_eq!(r.stale_days(edge + 1), Some(0));
        assert_eq!(r.stale_days(edge + 5 * DAY_MS + 1), Some(5));
        // A signer ahead of this clock is not "negative age".
        assert_eq!(r.age_ms(NOW - 1000), 0);
        assert!(!r.is_stale(NOW - 1000));
    }

    #[test]
    fn an_expired_certificate_is_not_admitted() {
        let key = iroh::SecretKey::generate();
        let c = Cert {
            root: *key.public().as_bytes(),
            subject: id(3),
            kind: Kind::Device,
            sponsor: None,
            issued_ms: NOW,
            seq: 1,
            expires_ms: Some(NOW + 10),
        };
        let bytes = sign_roster(&key, 1, NOW, vec![sign_cert(&key, &c)], Vec::new());
        let r = verify_roster(&bytes, &key.public()).unwrap();
        assert!(r.admits(&id(3), NOW + 9));
        assert!(!r.admits(&id(3), NOW + 10));
    }

    #[test]
    fn hex_ids_round_trip_and_refuse_anything_else() {
        let x = [0xab; 32];
        assert_eq!(id_from_hex(&id_hex(&x)), Some(x));
        assert_eq!(id_from_hex("ab"), None);
        assert_eq!(id_from_hex(&"zz".repeat(32)), None);
        let key = iroh::SecretKey::generate();
        assert_eq!(
            id_hex(key.public().as_bytes()),
            key.public().to_string(),
            "the spelling device ids already travel in"
        );
    }

    #[test]
    fn claimed_root_reads_the_label_and_verification_still_decides() {
        let (key, r) = fresh(&[Member::device(id(1))]);
        assert_eq!(claimed_root(r.bytes()), Some(key.public()));
        assert_eq!(claimed_root(b"nonsense"), None);
    }

    /// The pairing rule, both sides at once: whatever one side concludes
    /// the other must conclude the mirror of it.
    #[test]
    fn settle_resolves_every_pair_of_states() {
        let (_, lone_a) = fresh(&[Member::device(id(1))]);
        let (_, lone_b) = fresh(&[Member::device(id(2))]);
        let (_, big_a) = fresh(&[Member::device(id(1)), Member::device(id(3))]);
        let (_, big_b) = fresh(&[Member::device(id(2)), Member::device(id(4))]);
        let (a, b) = (id(1), id(2));

        // Nobody has a root.
        assert_eq!(settle(None, &a, None, &b, true), Settle::Keep);
        // One side only.
        assert_eq!(settle(None, &a, Some(&lone_b), &b, true), Settle::Adopt);
        assert_eq!(settle(Some(&lone_b), &b, None, &a, false), Settle::Keep);
        // Same root.
        assert_eq!(settle(Some(&big_a), &a, Some(&big_a), &a, true), Settle::Keep);
        // A network of one gives way to a network of more, either way round.
        assert_eq!(settle(Some(&lone_a), &a, Some(&big_b), &b, true), Settle::Adopt);
        assert_eq!(settle(Some(&big_b), &b, Some(&lone_a), &a, false), Settle::Keep);
        assert_eq!(settle(Some(&lone_a), &a, Some(&big_b), &b, false), Settle::Adopt);
        assert_eq!(settle(Some(&big_b), &b, Some(&lone_a), &a, true), Settle::Keep);
        // Two fresh machines: the one that dialed gives way.
        assert_eq!(settle(Some(&lone_a), &a, Some(&lone_b), &b, true), Settle::Adopt);
        assert_eq!(settle(Some(&lone_b), &b, Some(&lone_a), &a, false), Settle::Keep);
        // Two real networks: nobody decides that by pairing.
        assert_eq!(settle(Some(&big_a), &a, Some(&big_b), &b, true), Settle::Clash);
        assert_eq!(settle(Some(&big_b), &b, Some(&big_a), &a, false), Settle::Clash);
    }
}
