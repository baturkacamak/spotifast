//! The Jam window, under the mini player's playlist.
//!
//! Classic skins have no artwork for a Jam, so the window borrows the
//! playlist's colours and font: a title bar, one row per listener, the
//! link, and the two buttons that matter while hosting. The window is part
//! of the same stack as the equalizer and the playlist, so it moves and
//! scales with them.

use egui::{Color32, Sense, ViewportCommand};

use crate::app::App;
use crate::i18n::gettext;
use crate::jam::State;
use crate::model::Action;
use crate::skin::layout::{self, Area};

use super::View;

const TITLE_HEIGHT: u32 = 20;
const ROW_HEIGHT: u32 = 12;
const ROWS: usize = 5;
const FIRST_ROW: u32 = 24;
const LINK_TOP: u32 = 88;
const BUTTON_TOP: u32 = 102;
const BUTTON_HEIGHT: u32 = 14;

pub(super) fn show(app: &mut App, view: &mut View) {
    let (background, frame) = {
        let playlist = &view.skin.playlist;
        (
            rgb(playlist.normal_background),
            rgb(playlist.selected_background),
        )
    };
    let width = layout::WINDOW_WIDTH;
    let height = layout::JAM_HEIGHT;

    view.fill(0, 0, width, height, background);
    view.fill(0, 0, width, 1, frame);
    view.fill(0, height - 1, width, 1, frame);
    view.fill(0, 0, 1, height, frame);
    view.fill(width - 1, 0, 1, height, frame);
    view.fill(1, 1, width - 2, TITLE_HEIGHT - 1, frame);

    let title = view.interact(
        Area::new(0, 0, width, TITLE_HEIGHT),
        "jam-title",
        Sense::click_and_drag(),
    );
    if title.drag_started() {
        view.ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }
    let state = app.jam.clone();
    let count = state.session().map(|session| session.members.len());
    let heading = match count {
        Some(count) => format!("JAM {}", count.max(1)),
        None => "JAM".to_string(),
    };
    let heading_x = (width - heading.chars().count() as u32 * 5) / 2;
    view.text(&heading, Area::new(heading_x, 7, width, 6));

    let close = view.interact(Area::new(264, 3, 9, 9), "jam-close", Sense::click());
    view.text("X", Area::new(266, 4, 5, 6));
    if close.clicked() {
        app.actions.push(Action::ToggleWinampJam);
    }

    match state {
        State::Idle | State::Failed(_) => {
            let area = Area::new((width - 100) / 2, 54, 100, 18);
            button(
                app,
                view,
                area,
                "start-jam",
                &gettext(app.locale, "Start a Jam"),
                Action::StartJam,
            );
        }
        State::Starting => status(app, view, &gettext(app.locale, "Starting the Jam…")),
        State::Ending(_) => status(app, view, &gettext(app.locale, "Ending the Jam…")),
        State::Hosting(session) => {
            for (index, member) in session.members.iter().take(ROWS).enumerate() {
                let y = FIRST_ROW + index as u32 * ROW_HEIGHT;
                view.fill(8, y, 9, 9, frame);
                let initial: String = member
                    .display_name()
                    .chars()
                    .next()
                    .map(|character| character.to_uppercase().to_string())
                    .unwrap_or_default();
                view.text(&initial, Area::new(10, y + 2, 5, 6));
                view.text(member.display_name(), Area::new(20, y + 2, 240, 6));
            }
            super::pixel_line(app, view, &session.link(), Area::new(8, LINK_TOP, 259, 9));
            button(
                app,
                view,
                Area::new(8, BUTTON_TOP, 86, BUTTON_HEIGHT),
                "jam-copy",
                &gettext(app.locale, "Copy Jam link"),
                Action::CopyJamLink,
            );
            button(
                app,
                view,
                Area::new(100, BUTTON_TOP, 64, BUTTON_HEIGHT),
                "jam-end",
                &gettext(app.locale, "End the Jam"),
                Action::EndJam,
            );
        }
    }
}

/// A Winamp button in the playlist's colours: the label centred in a
/// framed block, pressed with the pointer.
fn button(app: &mut App, view: &mut View, area: Area, id: &str, label: &str, action: Action) {
    let (normal, selected) = {
        let playlist = &view.skin.playlist;
        (
            rgb(playlist.normal_background),
            rgb(playlist.selected_background),
        )
    };
    let response = view.interact(area, id, Sense::click());
    let fill = if response.is_pointer_button_down_on() {
        selected
    } else {
        normal
    };
    view.fill(area.x, area.y, area.width, area.height, fill);
    let border = selected;
    view.fill(area.x, area.y, area.width, 1, border);
    view.fill(area.x, area.y + area.height - 1, area.width, 1, border);
    view.fill(area.x, area.y, 1, area.height, border);
    view.fill(area.x + area.width - 1, area.y, 1, area.height, border);
    let text = Area::new(
        area.x + (area.width - label.chars().count() as u32 * 5) / 2,
        area.y + (area.height - 6) / 2,
        area.width,
        6,
    );
    view.text(label, text);
    if response.clicked() {
        app.actions.push(action);
    }
}

/// A line of state, in the pixel face so it can carry any language.
fn status(app: &mut App, view: &mut View, text: &str) {
    super::pixel_line(app, view, text, Area::new(8, 54, 259, 9));
}

fn rgb(color: [u8; 3]) -> Color32 {
    Color32::from_rgb(color[0], color[1], color[2])
}
