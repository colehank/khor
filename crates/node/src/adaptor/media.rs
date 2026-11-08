//! Vendor media becomes protocol content, without opening a URL or file.
use serde_json::{Value, json};

pub(super) fn image(address: &str) -> Option<Value> {
    if let Some(encoded) = address.strip_prefix("data:") {
        let (mime, data) = encoded.split_once(";base64,")?;
        return mime
            .starts_with("image/")
            .then(|| json!({"type":"image","mimeType":mime,"data":data}));
    }
    if address.starts_with("https://")
        || address.starts_with("http://")
        || address.starts_with("file://")
        || address.starts_with('/')
    {
        return Some(json!({"type":"resource_link","uri":address,"name":"图片"}));
    }
    None
}

pub(super) fn claude(block: &Value) -> Option<Value> {
    let kind = block["type"].as_str()?;
    if !matches!(kind, "image" | "document") {
        return None;
    }
    let source = &block["source"];
    let mime = source["media_type"].as_str().unwrap_or(if kind == "image" {
        "image/png"
    } else {
        "application/octet-stream"
    });
    if source["type"] == "base64" {
        let data = source["data"].as_str()?;
        return Some(if kind == "image" {
            json!({"type":"image","mimeType":mime,"data":data})
        } else {
            json!({"type":"resource","resource":{"uri":"attachment:document","mimeType":mime,"blob":data}})
        });
    }
    let uri = source["url"].as_str()?;
    Some(
        json!({"type":"resource_link","uri":uri,"name":block["title"].as_str().unwrap_or(if kind=="image" {"图片"} else {"文档"}),"mimeType":mime}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn image_bytes_stay_media_and_addresses_never_trigger_a_fetch() {
        let block = claude(&json!({"type":"image","source":{"type":"base64","media_type":"image/png","data":"aW1hZ2U="}})).unwrap();
        assert_eq!(block["type"], "image");
        assert_eq!(block["data"], "aW1hZ2U=");
        assert_eq!(
            image("https://example.com/a.png").unwrap()["uri"],
            "https://example.com/a.png"
        );
        assert!(image("javascript:alert(1)").is_none());
        assert!(image("not-an-image").is_none());
        let _: agent_client_protocol::schema::v1::ContentBlock =
            serde_json::from_value(block).unwrap();
    }
}
