//! **A certificate the system does not trust, passed for one host for this
//! run** (UX-42 §14, as Safari's 「访问此网站」). khor's machines on a LAN
//! often wear self-signed certificates, and a page there failed with
//! 「安全连接失败」 and no way through.
//!
//! When a load fails on the certificate, the certificate the server showed
//! is kept as the tab's offer (`offered`). 「仍然访问」 (`pass`) moves it to
//! the passed list, keyed by the exit machine and the host, and asks the
//! address again; WebKit's server-trust challenge (`answer`) is then
//! answered with that certificate's trust — **only that certificate**: a
//! different one on the same host is asked about again, as a first visit.
//!
//! **Never stored and never global.** Both lists live in this process and
//! go with it; nothing here is written anywhere, and every other challenge
//! (another host, another machine, a trusted certificate, a password) is
//! left to WebKit exactly as it was with no method at all.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_foundation::NSString;

#[link(name = "Security", kind = "framework")]
unsafe extern "C" {
    fn SecTrustGetCertificateCount(trust: *mut AnyObject) -> isize;
    fn SecTrustGetCertificateAtIndex(trust: *mut AnyObject, index: isize) -> *mut AnyObject;
    fn SecCertificateCopyData(cert: *mut AnyObject) -> *mut AnyObject;
}

/// One offer: which machine, which host, the address to go back to, and
/// the leaf certificate's bytes (`None` where the error carried none).
struct Offer {
    exit: String,
    host: String,
    url: String,
    leaf: Option<Vec<u8>>,
}

struct Book {
    /// By tab: the certificate its last failed load was shown.
    offered: HashMap<String, Offer>,
    /// By (exit, host): the certificate passed for this run.
    passed: HashMap<(String, String), Option<Vec<u8>>>,
}

fn book() -> &'static Mutex<Book> {
    static B: OnceLock<Mutex<Book>> = OnceLock::new();
    B.get_or_init(|| {
        Mutex::new(Book {
            offered: HashMap::new(),
            passed: HashMap::new(),
        })
    })
}

/// The host of an address, lower-cased.
fn host_of(url: &str) -> Option<String> {
    let rest = url.split_once("://")?.1;
    let auth = rest.split(['/', '?', '#']).next()?;
    let auth = auth.rsplit_once('@').map_or(auth, |(_, h)| h);
    let host = if let Some(v6) = auth.strip_prefix('[') {
        v6.split(']').next()?
    } else {
        auth.rsplit_once(':').map_or(auth, |(h, _)| h)
    };
    (!host.is_empty()).then(|| host.to_ascii_lowercase())
}

/// The DER bytes of a certificate.
///
/// # Safety
/// `cert` is a live `SecCertificateRef`.
unsafe fn der(cert: *mut AnyObject) -> Option<Vec<u8>> {
    if cert.is_null() {
        return None;
    }
    let data = unsafe { SecCertificateCopyData(cert) };
    // SAFETY: a CFData we own (Copy rule), toll-free an NSData.
    let data: Retained<AnyObject> = unsafe { Retained::from_raw(data) }?;
    let len: usize = unsafe { msg_send![&*data, length] };
    let at: *const u8 = unsafe { msg_send![&*data, bytes] };
    if at.is_null() {
        return None;
    }
    Some(unsafe { std::slice::from_raw_parts(at, len) }.to_vec())
}

/// The leaf of a `SecTrustRef`.
///
/// # Safety
/// `trust` is a live `SecTrustRef`.
unsafe fn leaf_of(trust: *mut AnyObject) -> Option<Vec<u8>> {
    if trust.is_null() || unsafe { SecTrustGetCertificateCount(trust) } < 1 {
        return None;
    }
    unsafe { der(SecTrustGetCertificateAtIndex(trust, 0)) }
}

/// Whether a failure is one a person may pass: the certificate is not
/// trusted, expired or not yet valid (NSURLErrorServerCertificate…).
/// Kept in step with the face's `browse/page-fail` `PASSABLE`.
pub fn passable(domain: &str, code: isize) -> bool {
    domain == "NSURLErrorDomain" && matches!(code, -1201 | -1202 | -1203 | -1204)
}

/// Keeps what a failed load was shown as `tab`'s offer, if the failure is
/// one a person may pass; any other failure takes the offer back.
///
/// # Safety
/// `error` is a live `NSError`.
pub unsafe fn offered(tab: &str, exit: &str, error: *mut AnyObject) {
    let Ok(mut b) = book().lock() else { return };
    b.offered.remove(tab);
    if error.is_null() {
        return;
    }
    let code: isize = unsafe { msg_send![error, code] };
    let domain: Option<Retained<NSString>> = unsafe { msg_send![error, domain] };
    if !passable(&domain.map(|d| d.to_string()).unwrap_or_default(), code) {
        return;
    }
    let info: *mut AnyObject = unsafe { msg_send![error, userInfo] };
    if info.is_null() {
        return;
    }
    let get = |key: &str| -> *mut AnyObject {
        let key = NSString::from_str(key);
        unsafe { msg_send![info, objectForKey: &*key] }
    };
    let mut url = get("NSErrorFailingURLStringKey");
    if url.is_null() {
        url = get("NSErrorFailingURLKey");
    }
    let url = if url.is_null() {
        String::new()
    } else {
        let s: Option<Retained<NSString>> = unsafe { msg_send![url, description] };
        s.map(|s| s.to_string()).unwrap_or_default()
    };
    let Some(host) = host_of(&url) else { return };
    // The trust the server presented; else the chain it sent.
    let trust = get("NSURLErrorFailingURLPeerTrustErrorKey");
    let leaf = if trust.is_null() {
        let chain = get("NSErrorPeerCertificateChainKey");
        if chain.is_null() {
            None
        } else {
            let first: *mut AnyObject = unsafe { msg_send![chain, firstObject] };
            unsafe { der(first) }
        }
    } else {
        unsafe { leaf_of(trust) }
    };
    b.offered.insert(
        tab.to_owned(),
        Offer {
            exit: exit.to_owned(),
            host,
            url,
            leaf,
        },
    );
}

/// 「仍然访问」: `tab`'s offer passed for this run; answers the address to
/// ask again, or `None` where nothing was offered.
pub fn pass(tab: &str) -> Option<String> {
    let mut b = book().lock().ok()?;
    let o = b.offered.remove(tab)?;
    b.passed.insert((o.exit, o.host), o.leaf);
    Some(o.url)
}

/// The tab's page is gone: its offer with it.
pub fn forget(tab: &str) {
    if let Ok(mut b) = book().lock() {
        b.offered.remove(tab);
    }
}

/// WebKit's challenge for a page on `exit`: a server trust for a host
/// passed this run, showing the certificate that was passed, is answered
/// with it (0: NSURLSessionAuthChallengeUseCredential); everything else is
/// WebKit's own handling (1: PerformDefaultHandling).
///
/// # Safety
/// `challenge` is a live `NSURLAuthenticationChallenge` and `done` its
/// completion handler; called once.
pub unsafe fn answer(
    exit: Option<&str>,
    challenge: *mut AnyObject,
    done: &block2::Block<dyn Fn(isize, *mut AnyObject)>,
) {
    let cred = exit.and_then(|exit| unsafe { credential(exit, challenge) });
    match cred {
        Some(c) => done.call((0, Retained::as_ptr(&c) as *mut AnyObject)),
        None => done.call((1, std::ptr::null_mut())),
    }
}

/// The credential for a passed certificate, if this challenge is one.
///
/// # Safety
/// As [`answer`].
unsafe fn credential(exit: &str, challenge: *mut AnyObject) -> Option<Retained<AnyObject>> {
    let space: *mut AnyObject = unsafe { msg_send![challenge, protectionSpace] };
    if space.is_null() {
        return None;
    }
    let method: Option<Retained<NSString>> = unsafe { msg_send![space, authenticationMethod] };
    if method?.to_string() != "NSURLAuthenticationMethodServerTrust" {
        return None;
    }
    let host: Option<Retained<NSString>> = unsafe { msg_send![space, host] };
    let host = host?.to_string().to_ascii_lowercase();
    let passed = book()
        .lock()
        .ok()?
        .passed
        .get(&(exit.to_owned(), host))
        .cloned()?;
    let trust: *mut AnyObject = unsafe { msg_send![space, serverTrust] };
    if trust.is_null() {
        return None;
    }
    // The certificate that was passed, not whatever the host shows now;
    // an offer that carried none was passed for the host.
    if let Some(want) = passed
        && unsafe { leaf_of(trust) }.as_deref() != Some(want.as_slice())
    {
        return None;
    }
    unsafe { msg_send![objc2::class!(NSURLCredential), credentialForTrust: trust] }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_host_is_read_from_any_address_form() {
        assert_eq!(
            host_of("https://Box.local:8443/a?b#c").as_deref(),
            Some("box.local")
        );
        assert_eq!(
            host_of("https://user:pw@10.0.0.2/").as_deref(),
            Some("10.0.0.2")
        );
        assert_eq!(
            host_of("https://[fe80::1]:443/").as_deref(),
            Some("fe80::1")
        );
        assert_eq!(host_of("about:blank"), None);
    }

    #[test]
    fn only_certificate_failures_are_passable() {
        assert!(passable("NSURLErrorDomain", -1202));
        assert!(passable("NSURLErrorDomain", -1201));
        assert!(
            !passable("NSURLErrorDomain", -1200),
            "a failed handshake is not a certificate to pass"
        );
        assert!(
            !passable("NSURLErrorDomain", -1206),
            "a client certificate is not passed"
        );
        assert!(!passable("WebKitErrorDomain", -1202));
    }

    #[test]
    fn a_pass_is_one_offer_once() {
        book().lock().unwrap().offered.insert(
            "t1".into(),
            Offer {
                exit: "m".into(),
                host: "box.local".into(),
                url: "https://box.local/".into(),
                leaf: Some(vec![1, 2]),
            },
        );
        assert_eq!(pass("t1").as_deref(), Some("https://box.local/"));
        assert_eq!(pass("t1"), None, "the offer is taken");
        assert_eq!(
            book()
                .lock()
                .unwrap()
                .passed
                .get(&("m".into(), "box.local".into())),
            Some(&Some(vec![1, 2]))
        );
        assert!(
            !book()
                .lock()
                .unwrap()
                .passed
                .contains_key(&("other".into(), "box.local".into())),
            "another machine's host is not passed"
        );
    }
}
