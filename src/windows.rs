//! Which windows are on screen right now, according to Hyprland.

use std::collections::HashSet;
use std::process::{Command, Stdio};

use serde::de::DeserializeOwned;
use serde::Deserialize;

use crate::geometry::Rect;

#[derive(Deserialize)]
struct Workspace {
    name: String,
    #[serde(rename = "type", default)]
    kind: String,
}

#[derive(Deserialize)]
struct Client {
    at: [i32; 2],
    size: [i32; 2],
    workspace: Workspace,
    mapped: bool,
    hidden: bool,
    floating: bool,
    pinned: bool,
    fullscreen: i32,
    #[serde(rename = "focusHistoryID")]
    focus_history: i32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Monitor {
    active_workspace: Workspace,
    special_workspace: Workspace,
}

/// Hyprland's value for a true fullscreen, as opposed to maximized.
const FULLSCREEN: i32 = 2;

/// The visible windows, topmost first.
fn visible_stack(clients: Vec<Client>, monitors: &[Monitor]) -> Vec<Rect> {
    let visible: HashSet<&str> = monitors
        .iter()
        .flat_map(|m| [m.active_workspace.name.as_str(), m.special_workspace.name.as_str()])
        .filter(|name| !name.is_empty())
        .collect();
    let mut shown: Vec<Client> = clients
        .into_iter()
        .filter(|c| c.mapped && !c.hidden && c.size[0] > 0 && c.size[1] > 0)
        .filter(|c| c.pinned || visible.contains(c.workspace.name.as_str()))
        .collect();
    // false sorts first, so each test reads as "not in this higher layer".
    shown.sort_by_key(|c| {
        (
            c.workspace.kind != "special",
            c.fullscreen < FULLSCREEN,
            !(c.floating || c.pinned),
            c.focus_history,
        )
    });
    shown
        .iter()
        .map(|c| Rect { x: c.at[0], y: c.at[1], w: c.size[0], h: c.size[1] })
        .collect()
}

fn query<T: DeserializeOwned>(what: &str) -> Result<T, String> {
    let output = Command::new("hyprctl")
        .args([what, "-j"])
        .stderr(Stdio::null())
        .output()
        .map_err(|e| format!("could not run hyprctl: {e}"))?;
    serde_json::from_slice(&output.stdout).map_err(|e| format!("unexpected hyprctl {what} output: {e}"))
}

/// The windows on screen, topmost first. Without Hyprland's answer there is
/// simply nothing to snap to, and dragging a selection still works.
pub fn on_screen() -> Vec<Rect> {
    let stack = query("clients").and_then(|clients| Ok(visible_stack(clients, &query::<Vec<Monitor>>("monitors")?)));
    stack.unwrap_or_else(|e| {
        eprintln!("cot: window snapping is off: {e}");
        Vec::new()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitors() -> Vec<Monitor> {
        serde_json::from_str(
            r#"[
                {"activeWorkspace": {"id": 1, "type": "normal", "name": "1"},
                 "specialWorkspace": {"type": "", "name": ""}},
                {"activeWorkspace": {"id": 3, "type": "normal", "name": "3"},
                 "specialWorkspace": {"type": "special", "name": "special:magic"}}
            ]"#,
        )
        .unwrap()
    }

    /// A tiled window on workspace 1, with its x position as an identifier.
    fn client(x: i32) -> Client {
        Client {
            at: [x, 0],
            size: [100, 100],
            workspace: Workspace { name: "1".into(), kind: "normal".into() },
            mapped: true,
            hidden: false,
            floating: false,
            pinned: false,
            fullscreen: 0,
            focus_history: 5,
        }
    }

    fn xs(clients: Vec<Client>) -> Vec<i32> {
        visible_stack(clients, &monitors()).iter().map(|r| r.x).collect()
    }

    #[test]
    fn parses_hyprctl_clients() {
        let json = r#"[{"at": [12, 56], "size": [3416, 1322],
            "workspace": {"address": "special:magic", "type": "special", "name": "special:magic"},
            "mapped": true, "hidden": false, "floating": false, "pinned": false,
            "fullscreen": 0, "fullscreenClient": 0, "focusHistoryID": 4, "title": "x"}]"#;
        let clients: Vec<Client> = serde_json::from_str(json).unwrap();
        assert_eq!(clients[0].at, [12, 56]);
        assert_eq!(clients[0].focus_history, 4);
    }

    #[test]
    fn leaves_out_other_workspaces_and_hidden_windows() {
        let mut other = client(1);
        other.workspace.name = "2".into();
        let mut hidden = client(2);
        hidden.hidden = true;
        let mut unmapped = client(3);
        unmapped.mapped = false;
        assert_eq!(xs(vec![other, hidden, unmapped, client(4)]), [4]);
    }

    #[test]
    fn a_pinned_window_shows_on_any_workspace() {
        let mut pinned = client(1);
        pinned.workspace.name = "9".into();
        pinned.pinned = true;
        assert_eq!(xs(vec![pinned]), [1]);
    }

    #[test]
    fn recently_focused_windows_are_on_top() {
        let (mut older, mut newer) = (client(1), client(2));
        older.focus_history = 3;
        newer.focus_history = 1;
        assert_eq!(xs(vec![older, newer]), [2, 1]);
    }

    #[test]
    fn layers_stack_special_then_fullscreen_then_floating_then_tiled() {
        let tiled = client(1);
        let mut floating = client(2);
        floating.floating = true;
        let mut fullscreen = client(3);
        fullscreen.fullscreen = 2;
        let mut special = client(4);
        special.workspace = Workspace { name: "special:magic".into(), kind: "special".into() };
        assert_eq!(xs(vec![tiled, floating, fullscreen, special]), [4, 3, 2, 1]);
    }

    #[test]
    fn a_maximized_window_does_not_count_as_fullscreen() {
        let mut maximized = client(1);
        maximized.fullscreen = 1;
        let mut floating = client(2);
        floating.floating = true;
        assert_eq!(xs(vec![maximized, floating]), [2, 1]);
    }
}
