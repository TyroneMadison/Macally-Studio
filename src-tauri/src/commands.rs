//! Tauri commands invoked from the webview (lib/ipc.ts).

/// The 107-key layout, compiled into the binary (no resource file to ship).
pub const LAYOUT_JSON: &str = include_str!("../../src/lib/layout.json");

#[tauri::command]
pub fn get_layout() -> Result<serde_json::Value, String> {
    serde_json::from_str(LAYOUT_JSON).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap;

    #[test]
    fn every_layout_code_is_in_keymap() {
        let layout = get_layout().unwrap();
        let keys = layout["keys"].as_array().unwrap();
        assert_eq!(keys.len(), 107);
        let missing: Vec<&str> = keys
            .iter()
            .map(|k| k["code"].as_str().unwrap())
            .filter(|c| keymap::def(c).is_none())
            .collect();
        assert!(missing.is_empty(), "codes missing from keymap::KEYS: {missing:?}");
    }
}
