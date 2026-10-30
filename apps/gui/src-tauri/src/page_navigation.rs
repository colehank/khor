//! Navigation identity and snapshots shared by the native page adapters.
//!
//! WebKit's navigation object identifies a load; its URL does not (reloads
//! and redirects). A view generation also guards callbacks queued before a
//! page with the same tab label was replaced. Only `finished` reaches 1.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use khor_gui_core::web::{PageNavigationError, PageNavigationPhase, PageNavigationState};

const ERROR_STRING_BYTES: usize = 4 * 1024;
const ERROR_CHAIN_LENGTH: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ViewKey {
    address: usize,
    generation: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct Changed {
    pub face: String,
    pub state: PageNavigationState,
}

/// Native input keeps an exact retry target separately from its bounded
/// diagnostic projection. It is not a public/serializable wire type.
pub(crate) struct Failure {
    pub error: PageNavigationError,
    pub target: String,
}

/// Only the borrow producer's explicit failure document uses this reason;
/// the presence of a proxy or a generic connection-lost error does not.
pub(crate) fn route_error(url: String) -> Failure {
    Failure {
        target: url.clone(),
        error: PageNavigationError {
            domain: "khor".into(),
            code: "route".into(),
            description: String::new(),
            url,
            underlying: Vec::new(),
            truncated: false,
        },
    }
}

#[derive(Clone)]
struct Page {
    key: ViewKey,
    face: String,
    native_navigation: Option<usize>,
    failed_target: Option<String>,
    state: PageNavigationState,
}

/// Synchronous SDK control attempt only; no handle, observer, queue or
/// second registry. A nil SDK result restores the prior page facts.
pub(crate) struct PreparedRequest {
    key: ViewKey,
    revision: String,
    previous: Page,
}

/// The one page lifecycle registry. Platform adapters own native handles,
/// not another copy of the navigation/loading/error facts kept here.
#[derive(Default)]
pub(crate) struct Pages {
    serial: u64,
    views: HashMap<usize, Page>,
}

fn pages() -> &'static Mutex<Pages> {
    static PAGES: OnceLock<Mutex<Pages>> = OnceLock::new();
    PAGES.get_or_init(Default::default)
}

pub(crate) fn with_pages<T>(use_pages: impl FnOnce(&mut Pages) -> T) -> Option<T> {
    pages().lock().ok().map(|mut pages| use_pages(&mut pages))
}

pub(crate) fn snapshot(tab: &str) -> Option<PageNavigationState> {
    with_pages(|pages| pages.snapshot(tab)).flatten()
}

pub(crate) fn retry_target(tab: &str) -> Option<String> {
    with_pages(|pages| pages.retry_target(tab)).flatten()
}

fn estimate(value: f64) -> Option<f64> {
    // A WebKit estimate of 1 can precede didFinish. It is still loading
    // until that navigation's success callback, never a synthetic finish.
    (value.is_finite() && (0.0..=1.0).contains(&value)).then(|| value.min(0.99))
}

fn active(phase: &PageNavigationPhase) -> bool {
    matches!(
        phase,
        PageNavigationPhase::Started | PageNavigationPhase::Committed
    )
}

impl Pages {
    /// Wry platforms without a WK object use an issued builder-instance
    /// key. These tokens identify our event/request stream, not OS nav IDs.
    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), test))]
    pub(crate) fn attach_wry(
        &mut self,
        face: String,
        tab: String,
        target: String,
        exit: String,
        proxied: bool,
    ) -> Option<(ViewKey, Changed)> {
        let address = usize::try_from(self.serial.checked_add(1)?).ok()?;
        self.attach(address, face, tab, target, exit, proxied)
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), test))]
    pub(crate) fn wry_started(&mut self, key: ViewKey, target: String) -> Option<Changed> {
        let issued = usize::try_from(self.serial.checked_add(1)?).ok()?;
        self.started(key, issued, target)
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), test))]
    pub(crate) fn wry_finished(&mut self, key: ViewKey, url: String) -> Option<Changed> {
        let navigation = self
            .views
            .get(&key.address)
            .filter(|page| page.key == key)?
            .native_navigation?;
        self.update_navigation(key, navigation, |page| {
            page.state.phase = PageNavigationPhase::Settled;
            page.state.url = url;
            page.state.estimate = None;
            page.state.error = None;
            page.failed_target = None;
            true
        })
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), test))]
    pub(crate) fn wry_failed(&mut self, key: ViewKey, failure: Failure) -> Option<Changed> {
        let navigation = self
            .views
            .get(&key.address)
            .filter(|page| page.key == key)?
            .native_navigation?;
        self.failed(key, navigation, failure)
    }

    pub(crate) fn attach(
        &mut self,
        address: usize,
        face: String,
        tab: String,
        requested_url: String,
        exit: String,
        proxied: bool,
    ) -> Option<(ViewKey, Changed)> {
        let serial = self.serial.checked_add(1)?;
        let key = ViewKey {
            address,
            generation: serial,
        };
        let state = PageNavigationState {
            tab: tab.clone(),
            view: serial.to_string(),
            navigation: String::new(),
            revision: serial.to_string(),
            phase: PageNavigationPhase::Opening,
            estimate: None,
            requested_url,
            url: String::new(),
            exit,
            proxied,
            error: None,
        };
        self.views.retain(|_, page| page.state.tab != tab);
        self.views.insert(
            address,
            Page {
                key,
                face: face.clone(),
                native_navigation: None,
                failed_target: None,
                state: state.clone(),
            },
        );
        self.serial = serial;
        Some((key, Changed { face, state }))
    }

    /// Capture synchronously in a native callback, before deferring work.
    /// Passing only a raw address across the queue could hit a replacement.
    pub(crate) fn key(&self, address: usize) -> Option<ViewKey> {
        self.views.get(&address).map(|page| page.key)
    }

    pub(crate) fn key_for_tab(&self, tab: &str) -> Option<ViewKey> {
        self.views
            .values()
            .find(|page| page.state.tab == tab)
            .map(|page| page.key)
    }

    pub(crate) fn state(&self, key: ViewKey) -> Option<&PageNavigationState> {
        self.views
            .get(&key.address)
            .filter(|page| page.key == key)
            .map(|page| &page.state)
    }

    pub(crate) fn snapshot(&self, tab: &str) -> Option<PageNavigationState> {
        self.views
            .values()
            .find(|page| page.state.tab == tab)
            .map(|page| page.state.clone())
    }

    pub(crate) fn retry_target(&self, tab: &str) -> Option<String> {
        self.views
            .values()
            .find(|page| page.state.tab == tab)
            .and_then(|page| page.failed_target.clone())
    }

    pub(crate) fn detach(&mut self, key: ViewKey) {
        if self.key(key.address) == Some(key) {
            self.views.remove(&key.address);
        }
    }

    /// An explicit target replaces the old load immediately, before WebKit
    /// starts the new navigation. Old failure/finish callbacks then cannot
    /// complete the new opening state.
    pub(crate) fn request(&mut self, key: ViewKey, target: String) -> Option<Changed> {
        self.update(key, |page| {
            page.native_navigation = None;
            page.state.phase = PageNavigationPhase::Opening;
            page.state.requested_url = target;
            page.state.navigation.clear();
            page.state.estimate = None;
            page.state.error = None;
            page.failed_target = None;
            true
        })
    }

    /// Bind the navigation returned by an explicit loadRequest before the
    /// runloop can deliver callbacks for the replaced request. Internal
    /// links are instead issued synchronously by their didStart callback.
    pub(crate) fn expect(&mut self, key: ViewKey, request_revision: &str, navigation: usize) {
        if navigation == 0 {
            return;
        }
        if let Some(page) = self.views.get_mut(&key.address).filter(|page| {
            page.key == key
                && page.state.phase == PageNavigationPhase::Opening
                && page.state.revision == request_revision
        }) {
            page.native_navigation = Some(navigation);
        }
    }

    pub(crate) fn prepare_request(
        &mut self,
        key: ViewKey,
        target: String,
    ) -> Option<(PreparedRequest, Changed)> {
        let previous = self
            .views
            .get(&key.address)
            .filter(|page| page.key == key)?
            .clone();
        let changed = self.request(key, target)?;
        Some((
            PreparedRequest {
                key,
                revision: changed.state.revision.clone(),
                previous,
            },
            changed,
        ))
    }

    /// No SDK navigation (e.g. a same-document/history no-op) does not
    /// fabricate a load, success or cancellation. Newer accepted facts win.
    pub(crate) fn no_navigation(&mut self, request: PreparedRequest) -> bool {
        let Some(current) = self.views.get(&request.key.address) else {
            return false;
        };
        if current.key != request.key
            || current.state.revision != request.revision
            || current.state.phase != PageNavigationPhase::Opening
            || current.native_navigation.is_some()
        {
            return false;
        }
        let Some(serial) = self.serial.checked_add(1) else {
            return false;
        };
        let mut previous = request.previous;
        previous.state.revision = serial.to_string();
        self.views.insert(request.key.address, previous);
        self.serial = serial;
        true
    }

    /// Invoke synchronously in the native didStart callback. If an adapter
    /// needs to defer delivery, defer the returned immutable Changed only.
    /// Replaying a raw start later would incorrectly issue a fresh load.
    pub(crate) fn started(
        &mut self,
        key: ViewKey,
        navigation: usize,
        target: String,
    ) -> Option<Changed> {
        if navigation == 0 {
            return None;
        }
        let serial = self.serial.checked_add(1)?;
        self.update(key, |page| {
            if page.native_navigation == Some(navigation) && active(&page.state.phase) {
                return false;
            }
            let opening = page.state.phase == PageNavigationPhase::Opening;
            let expected = opening && page.native_navigation.is_some();
            if expected && page.native_navigation != Some(navigation) {
                return false;
            }
            page.native_navigation = Some(navigation);
            page.state.navigation = serial.to_string();
            page.state.phase = PageNavigationPhase::Started;
            if page.state.requested_url.is_empty() || !opening {
                page.state.requested_url = target;
            }
            page.state.estimate = None;
            page.state.error = None;
            page.failed_target = None;
            true
        })
    }

    /// Capture progress in the native callback, not in a later dispatch.
    /// This result, including its issued revision, is safe to queue.
    pub(crate) fn progressed(&mut self, key: ViewKey, value: f64) -> Option<Changed> {
        let value = estimate(value)?;
        self.update(key, |page| {
            if !active(&page.state.phase) || page.state.estimate.is_some_and(|was| was >= value) {
                return false;
            }
            page.state.estimate = Some(value);
            true
        })
    }

    pub(crate) fn committed(
        &mut self,
        key: ViewKey,
        navigation: usize,
        url: String,
    ) -> Option<Changed> {
        self.update_navigation(key, navigation, |page| {
            if page.state.phase == PageNavigationPhase::Committed && page.state.url == url {
                return false;
            }
            page.state.phase = PageNavigationPhase::Committed;
            page.state.url = url;
            true
        })
    }

    pub(crate) fn finished(
        &mut self,
        key: ViewKey,
        navigation: usize,
        url: String,
    ) -> Option<Changed> {
        self.update_navigation(key, navigation, |page| {
            page.state.phase = PageNavigationPhase::Finished;
            page.state.url = url;
            page.state.estimate = Some(1.0);
            page.state.error = None;
            page.failed_target = None;
            true
        })
    }

    /// Explicitly accepted user stop, not a fabricated NSError or success.
    /// SDK download callbacks are not enough to infer a top-level cancel.
    pub(crate) fn cancelled(&mut self, key: ViewKey) -> Option<Changed> {
        self.update(key, |page| {
            if page.state.phase != PageNavigationPhase::Opening && !active(&page.state.phase) {
                return false;
            }
            page.state.phase = PageNavigationPhase::Cancelled;
            page.state.error = None;
            page.failed_target = None;
            true
        })
    }

    pub(crate) fn failed(
        &mut self,
        key: ViewKey,
        navigation: usize,
        failure: Failure,
    ) -> Option<Changed> {
        self.update_navigation(key, navigation, |page| {
            page.failed_target = Some(if failure.target.is_empty() {
                page.state.requested_url.clone()
            } else {
                failure.target
            });
            let error = bounded_error(failure.error);
            page.state.phase = if (error.domain == "NSURLErrorDomain" && error.code == "-999")
                || (error.domain == "WebKitErrorDomain" && error.code == "102")
            {
                PageNavigationPhase::Cancelled
            } else {
                PageNavigationPhase::Failed
            };
            page.state.error = Some(error);
            true
        })
    }

    fn update_navigation(
        &mut self,
        key: ViewKey,
        navigation: usize,
        update: impl FnOnce(&mut Page) -> bool,
    ) -> Option<Changed> {
        self.update(key, |page| {
            if navigation == 0
                || page.native_navigation != Some(navigation)
                || !active(&page.state.phase)
            {
                return false;
            }
            update(page)
        })
    }

    fn update(&mut self, key: ViewKey, update: impl FnOnce(&mut Page) -> bool) -> Option<Changed> {
        let serial = self.serial.checked_add(1)?;
        let page = self.views.get_mut(&key.address)?;
        if page.key != key || !update(page) {
            return None;
        }
        self.serial = serial;
        page.state.revision = serial.to_string();
        Some(Changed {
            face: page.face.clone(),
            state: page.state.clone(),
        })
    }
}

fn bound(value: &mut String, truncated: &mut bool) {
    if value.len() <= ERROR_STRING_BYTES {
        return;
    }
    let mut end = ERROR_STRING_BYTES;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value.truncate(end);
    *truncated = true;
}

/// This bounds the public error snapshot as well as the native extractor.
/// Do not put certificates, userInfo blobs, or proxy credentials in it.
pub(crate) fn bounded_error(mut error: PageNavigationError) -> PageNavigationError {
    for value in [
        &mut error.domain,
        &mut error.code,
        &mut error.description,
        &mut error.url,
    ] {
        bound(value, &mut error.truncated);
    }
    if error.underlying.len() >= ERROR_CHAIN_LENGTH {
        error.underlying.truncate(ERROR_CHAIN_LENGTH - 1);
        error.truncated = true;
    }
    for cause in &mut error.underlying {
        for value in [
            &mut cause.domain,
            &mut cause.code,
            &mut cause.description,
            &mut cause.url,
        ] {
            bound(value, &mut error.truncated);
        }
    }
    error
}

#[cfg(test)]
mod tests {
    use super::*;
    use khor_gui_core::web::PageNavigationCause;

    fn attach(pages: &mut Pages, address: usize) -> ViewKey {
        pages
            .attach(
                address,
                "main".into(),
                "tab".into(),
                "https://first.test".into(),
                "exit".into(),
                true,
            )
            .unwrap()
            .0
    }
    fn error(code: &str) -> Failure {
        Failure {
            target: "https://failed.test".into(),
            error: PageNavigationError {
                domain: "NSURLErrorDomain".into(),
                code: code.into(),
                description: "actual refusal".into(),
                url: "https://failed.test".into(),
                underlying: vec![],
                truncated: false,
            },
        }
    }

    #[test]
    fn reload_has_new_identity_but_redirect_progress_never_rewinds() {
        let mut pages = Pages::default();
        let key = attach(&mut pages, 1);
        let first = pages
            .started(key, 10, "https://first.test".into())
            .unwrap()
            .state;
        pages.progressed(key, 0.7).unwrap();
        assert!(pages.progressed(key, 0.1).is_none());
        let redirect = pages
            .committed(key, 10, "https://redirect.test".into())
            .unwrap()
            .state;
        assert_eq!(redirect.navigation, first.navigation);
        assert_eq!(redirect.estimate, Some(0.7));
        pages
            .finished(key, 10, "https://redirect.test".into())
            .unwrap();
        // Native addresses may be reused after the previous navigation ended.
        let reload = pages
            .started(key, 10, "https://redirect.test".into())
            .unwrap()
            .state;
        assert_ne!(reload.navigation, first.navigation);
        assert_eq!(reload.estimate, None);
    }

    #[test]
    fn estimate_one_is_not_success_and_non_finite_values_are_ignored() {
        let mut pages = Pages::default();
        let key = attach(&mut pages, 1);
        assert!(pages.progressed(key, 0.8).is_none());
        pages.started(key, 10, "https://first.test".into()).unwrap();
        assert!(pages.progressed(key, f64::NAN).is_none());
        assert!(pages.progressed(key, f64::INFINITY).is_none());
        assert!(pages.progressed(key, -0.1).is_none());
        assert!(pages.progressed(key, 1.1).is_none());
        assert_eq!(
            pages.progressed(key, 1.0).unwrap().state.estimate,
            Some(0.99)
        );
        let finished = pages
            .finished(key, 10, "https://first.test".into())
            .unwrap()
            .state;
        assert_eq!(finished.phase, PageNavigationPhase::Finished);
        assert_eq!(finished.estimate, Some(1.0));
        assert!(pages.progressed(key, 0.5).is_none());
        assert!(pages.failed(key, 10, error("-1004")).is_none());
    }

    #[test]
    fn failure_and_cancellation_never_complete_a_page() {
        for (code, expected) in [
            ("-1004", PageNavigationPhase::Failed),
            ("-999", PageNavigationPhase::Cancelled),
        ] {
            let mut pages = Pages::default();
            let key = attach(&mut pages, 1);
            pages.started(key, 10, "https://first.test".into()).unwrap();
            pages.progressed(key, 0.4);
            let failed = pages.failed(key, 10, error(code)).unwrap().state;
            assert_eq!(failed.phase, expected);
            assert_eq!(failed.estimate, Some(0.4));
            assert_eq!(failed.error.unwrap().code, code);
            assert!(
                pages
                    .finished(key, 10, "https://first.test".into())
                    .is_none()
            );
        }
    }

    #[test]
    fn old_navigation_cannot_finish_a_new_request_or_load() {
        let mut pages = Pages::default();
        let key = attach(&mut pages, 1);
        pages.started(key, 10, "https://first.test".into()).unwrap();
        pages.request(key, "https://second.test".into()).unwrap();
        assert!(
            pages
                .finished(key, 10, "https://first.test".into())
                .is_none()
        );
        pages
            .started(key, 11, "https://second.test".into())
            .unwrap();
        assert!(pages.failed(key, 10, error("-999")).is_none());
        assert!(
            pages.retry_target("tab").is_none(),
            "late failure cannot install a recovery target for the new load"
        );
        assert_eq!(
            pages.snapshot("tab").unwrap().requested_url,
            "https://second.test"
        );
    }

    #[test]
    fn an_explicit_request_binds_its_native_navigation_before_callbacks_arrive() {
        let mut pages = Pages::default();
        let key = attach(&mut pages, 1);
        pages.started(key, 10, "https://first.test".into()).unwrap();
        let requested = pages.request(key, "https://second.test".into()).unwrap();
        pages.expect(key, &requested.state.revision, 11);
        let pending = pages.snapshot("tab").unwrap();
        assert!(
            pages
                .started(key, 10, "https://first.test".into())
                .is_none()
        );
        assert_eq!(pages.snapshot("tab").unwrap(), pending);
        // WK URL may still show the old document in the first callback.
        let current = pages
            .started(key, 11, "https://first.test".into())
            .unwrap()
            .state;
        assert_eq!(current.requested_url, "https://second.test");
    }

    #[test]
    fn replacing_or_detaching_a_view_rejects_queued_callbacks() {
        let mut pages = Pages::default();
        let old = attach(&mut pages, 1);
        pages.started(old, 10, "https://first.test".into()).unwrap();
        let current = attach(&mut pages, 1);
        assert_ne!(old, current);
        pages.detach(old);
        assert_eq!(pages.key(1), Some(current));
        assert!(pages.progressed(old, 0.9).is_none());
        assert!(pages.started(old, 12, "https://late.test".into()).is_none());
        let moved = attach(&mut pages, 2);
        assert!(pages.key(1).is_none());
        assert_ne!(moved.generation, current.generation);
        pages.detach(moved);
        assert!(pages.snapshot("tab").is_none());
    }

    #[test]
    fn revisions_are_ordered_across_view_replacement_without_exposing_native_addresses() {
        let mut pages = Pages::default();
        let first = attach(&mut pages, 1234);
        let a = pages
            .started(first, 9999, "https://first.test".into())
            .unwrap();
        let second = attach(&mut pages, 5678);
        let b = pages
            .started(second, 9999, "https://second.test".into())
            .unwrap();
        assert!(
            b.state.revision.parse::<u64>().unwrap() > a.state.revision.parse::<u64>().unwrap()
        );
        assert_ne!(a.state.view, "1234");
        assert_ne!(a.state.navigation, "9999");
        assert_eq!(b.face, "main");
    }

    #[test]
    fn bounded_diagnostics_do_not_truncate_the_exact_retry_target() {
        let mut pages = Pages::default();
        let key = attach(&mut pages, 1);
        pages.started(key, 10, "https://first.test".into()).unwrap();
        let target = format!("https://failed.test/{}", "中".repeat(3000));
        let mut raw = error("-1004");
        raw.target = target.clone();
        raw.error.url = target.clone();
        let failed = pages.failed(key, 10, raw).unwrap().state;
        assert!(failed.error.unwrap().url.len() <= 4096);
        assert_eq!(pages.retry_target("tab"), Some(target));
        pages.request(key, "https://next.test".into()).unwrap();
        assert!(pages.retry_target("tab").is_none());
    }

    #[test]
    fn wry_events_keep_unknown_progress_without_inventing_an_os_navigation_id() {
        let mut pages = Pages::default();
        let (key, _) = pages
            .attach_wry(
                "main".into(),
                "tab".into(),
                "https://first.test".into(),
                "exit".into(),
                true,
            )
            .unwrap();
        let started = pages
            .wry_started(key, "https://first.test".into())
            .unwrap()
            .state;
        assert_eq!(started.estimate, None);
        let finished = pages
            .wry_finished(key, "https://first.test".into())
            .unwrap()
            .state;
        assert_eq!(finished.phase, PageNavigationPhase::Settled);
        assert_eq!(finished.estimate, None);
        pages.wry_started(key, "https://first.test".into()).unwrap();
        let failed = pages
            .wry_failed(key, route_error("https://first.test".into()))
            .unwrap()
            .state;
        assert_eq!(failed.phase, PageNavigationPhase::Failed);
        assert_eq!(failed.estimate, None);
        assert!(
            pages
                .wry_finished(key, "https://first.test".into())
                .is_none()
        );
        let (_, _) = pages
            .attach_wry(
                "main".into(),
                "tab".into(),
                "https://first.test".into(),
                "exit".into(),
                true,
            )
            .unwrap();
        assert!(
            pages
                .wry_finished(key, "https://first.test".into())
                .is_none(),
            "old builder callback cannot affect replacement"
        );
    }

    #[test]
    fn accepted_user_stop_is_cancelled_without_inventing_an_error_or_success() {
        let mut pages = Pages::default();
        let key = attach(&mut pages, 1);
        pages.started(key, 10, "https://first.test".into()).unwrap();
        pages.progressed(key, 0.4);
        let cancelled = pages.cancelled(key).unwrap().state;
        assert_eq!(cancelled.phase, PageNavigationPhase::Cancelled);
        assert_eq!(cancelled.estimate, Some(0.4));
        assert!(cancelled.error.is_none());
        assert!(
            pages
                .finished(key, 10, "https://first.test".into())
                .is_none()
        );
    }

    #[test]
    fn nil_sdk_navigation_preserves_the_real_load_and_cannot_restore_over_a_new_request() {
        let mut pages = Pages::default();
        let key = attach(&mut pages, 1);
        pages.started(key, 10, "https://first.test".into()).unwrap();
        pages.progressed(key, 0.4);
        let before = pages.snapshot("tab").unwrap();
        let (ticket, _) = pages.prepare_request(key, String::new()).unwrap();
        assert!(pages.no_navigation(ticket));
        let restored = pages.snapshot("tab").unwrap();
        assert_eq!(restored.navigation, before.navigation);
        assert_eq!(restored.phase, before.phase);
        assert_eq!(restored.estimate, before.estimate);
        let (stale, _) = pages.prepare_request(key, String::new()).unwrap();
        let requested = pages.request(key, "https://new.test".into()).unwrap();
        assert!(!pages.no_navigation(stale));
        pages.expect(key, &requested.state.revision, 11);
        assert!(
            pages
                .finished(key, 10, "https://first.test".into())
                .is_none()
        );
    }

    #[test]
    fn raw_error_fields_and_underlying_chain_are_bounded_on_utf8_boundaries() {
        let mut raw = error("-1004").error;
        raw.description = "中".repeat(2000);
        raw.underlying = (0..5)
            .map(|_| PageNavigationCause {
                domain: "POSIX".into(),
                code: "61".into(),
                description: "x".repeat(5000),
                url: String::new(),
            })
            .collect();
        let bounded = bounded_error(raw);
        assert!(bounded.truncated);
        assert_eq!(bounded.underlying.len(), 3);
        assert!(bounded.description.len() <= 4096);
        assert!(
            bounded
                .description
                .is_char_boundary(bounded.description.len())
        );
        assert!(
            bounded
                .underlying
                .iter()
                .all(|cause| cause.description.len() == 4096)
        );
        assert_eq!(bounded.domain, "NSURLErrorDomain");
        assert_eq!(bounded.code, "-1004");
    }
}
