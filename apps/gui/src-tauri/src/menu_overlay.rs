//! A completion list has one owner and one lease, regardless of its native host.
//!
//! The app's face owns filtering, focus and selection. Native adapters only
//! host the trusted list over another native WebView. A stale size, dismissal
//! or press must never act on a newer field's list.

use std::sync::{Mutex, OnceLock};

#[cfg(desktop)]
pub mod desktop;
#[cfg(target_os = "ios")]
pub mod ios;

#[derive(Clone, Debug, serde::Deserialize)]
pub struct MenuSpot {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub view: f64,
    pub across: f64,
    #[serde(default)]
    pub above: bool,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct MenuRequest {
    pub id: String,
    pub at: MenuSpot,
    pub rows: serde_json::Value,
    #[serde(default)]
    pub html: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct MenuSnapshot {
    pub id: String,
    pub rows: serde_json::Value,
}

impl MenuRequest {
    pub fn validate(&self) -> Result<(), String> {
        let at = &self.at;
        if self.id.is_empty() || self.id.len() > 256 {
            return Err("invalid menu lease".into());
        }
        if ![at.x, at.y, at.w, at.view, at.across]
            .iter()
            .all(|n| n.is_finite())
            || at.w <= 0.0
            || at.view <= 0.0
            || at.across <= 0.0
        {
            return Err("invalid menu geometry".into());
        }
        if !self
            .rows
            .get("groups")
            .is_some_and(serde_json::Value::is_array)
        {
            return Err("invalid menu rows".into());
        }
        if self
            .html
            .as_ref()
            .is_some_and(|html| html.len() > 2_000_000)
        {
            return Err("menu document is too large".into());
        }
        Ok(())
    }

    pub fn snapshot(&self) -> MenuSnapshot {
        MenuSnapshot {
            id: self.id.clone(),
            rows: self.rows.clone(),
        }
    }

    fn contains(&self, key: &str) -> bool {
        self.rows["groups"].as_array().is_some_and(|groups| {
            groups.iter().any(|group| {
                group["candidates"]
                    .as_array()
                    .is_some_and(|rows| rows.iter().any(|row| row["key"].as_str() == Some(key)))
            })
        })
    }
}

#[derive(Clone, Debug)]
pub struct HeldMenu {
    pub owner: String,
    pub request: MenuRequest,
}

#[derive(Default)]
struct Lease {
    held: Option<HeldMenu>,
}

impl Lease {
    fn claim(&mut self, owner: &str, request: &MenuRequest) -> Result<(), String> {
        request.validate()?;
        // These are khor's trusted faces. A remote browsing child must never
        // publish trusted HTML or borrow the app's candidate channel.
        if owner != "main" && owner != "settings" {
            return Err("this window cannot own a menu".into());
        }
        self.held = Some(HeldMenu {
            owner: owner.into(),
            request: request.clone(),
        });
        Ok(())
    }

    fn matches(&self, owner: &str, id: &str) -> bool {
        self.held
            .as_ref()
            .is_some_and(|held| held.owner == owner && held.request.id == id)
    }

    fn dismiss(&mut self, owner: &str, id: &str) -> bool {
        if !self.matches(owner, id) {
            return false;
        }
        self.held = None;
        true
    }

    fn resize(&self, id: &str, height: f64) -> bool {
        height.is_finite()
            && height > 0.0
            && self.held.as_ref().is_some_and(|held| held.request.id == id)
    }

    fn pick(&self, id: &str, key: &str) -> Option<String> {
        self.held
            .as_ref()
            .filter(|held| held.request.id == id && held.request.contains(key))
            .map(|held| held.owner.clone())
    }
}

fn lease() -> &'static Mutex<Lease> {
    static STATE: OnceLock<Mutex<Lease>> = OnceLock::new();
    STATE.get_or_init(Default::default)
}

pub fn claim(owner: &str, request: &MenuRequest) -> Result<(), String> {
    lease()
        .lock()
        .map_err(|_| "menu state unavailable".to_string())?
        .claim(owner, request)
}

pub fn held() -> Option<HeldMenu> {
    lease().lock().ok()?.held.clone()
}

pub fn snapshot() -> Option<MenuSnapshot> {
    held().map(|held| held.request.snapshot())
}

pub fn matches(owner: &str, id: &str) -> bool {
    lease().lock().is_ok_and(|state| state.matches(owner, id))
}

pub fn dismiss(owner: &str, id: &str) -> bool {
    lease()
        .lock()
        .is_ok_and(|mut state| state.dismiss(owner, id))
}

pub fn accepts_size(id: &str, height: f64) -> bool {
    lease().lock().is_ok_and(|state| state.resize(id, height))
}

/// Native adapters call this only from their own trusted renderer channel.
pub fn pick_owner(id: &str, key: &str) -> Option<String> {
    lease().lock().ok()?.pick(id, key)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(id: &str) -> MenuRequest {
        MenuRequest {
            id: id.into(),
            at: MenuSpot {
                x: 2.0,
                y: 3.0,
                w: 120.0,
                view: 500.0,
                across: 300.0,
                above: false,
            },
            rows: serde_json::json!({"groups":[{"candidates":[{"key":"m:local"}]}],"at":0}),
            html: None,
        }
    }

    #[test]
    fn stale_callbacks_cannot_act_on_the_new_owner() {
        let mut state = Lease::default();
        state.claim("main", &request("old")).unwrap();
        state.claim("settings", &request("new")).unwrap();
        assert!(!state.dismiss("main", "old"));
        assert!(!state.resize("old", 100.0));
        assert_eq!(state.pick("old", "m:local"), None);
        assert!(!state.dismiss("main", "new"));
        assert!(state.resize("new", 100.0));
        assert_eq!(state.pick("new", "m:local"), Some("settings".into()));
        assert!(state.dismiss("settings", "new"));
        assert!(!state.resize("new", 100.0));
    }

    #[test]
    fn only_current_candidates_can_be_pressed() {
        let mut state = Lease::default();
        state.claim("main", &request("one")).unwrap();
        assert_eq!(state.pick("one", "m:remote"), None);
        let mut next = request("one");
        next.rows = serde_json::json!({"groups":[]});
        state.claim("main", &next).unwrap();
        assert_eq!(state.pick("one", "m:local"), None);
    }

    #[test]
    fn rejects_untrusted_hosts_and_nonfinite_geometry() {
        let mut state = Lease::default();
        assert!(state.claim("page:remote", &request("x")).is_err());
        let mut bad = request("bad");
        bad.at.y = f64::NAN;
        assert!(state.claim("main", &bad).is_err());
        state.claim("main", &request("good")).unwrap();
        for h in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(!state.resize("good", h));
        }
    }
}
