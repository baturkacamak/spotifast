//! The Jam window, under the mini player's playlist.
//!
//! Classic skins have no artwork for a Jam, so the window is built from the
//! playlist's own pieces: its title bar and side borders, its colours, and
//! its font. It sits in the same stack as the equalizer and the playlist,
//! moves and scales with them, and rolls up to a shade bar the way they do.

use egui::{Color32, Sense, ViewportCommand};

use crate::app::App;
use crate::i18n::gettext;
use crate::jam::State;
use crate::model::Action;
use crate::skin::layout::{self, Area};
use crate::skin::sprites;

use super::View;

const TITLE_HEIGHT: u32 = 20;
const ROW_HEIGHT: u32 = 12;
const ROWS: usize = 5;
const FIRST_ROW: u32 = 26;
const LINK_TOP: u32 = 90;
const BUTTON_TOP: u32 = 102;
const BUTTON_HEIGHT: u32 = 14;

pub(super) fn show(app: &mut App, view: &mut View, focused: bool) {
    let width = layout::WINDOW_WIDTH;
    let height = layout::JAM_HEIGHT;
    let (background, frame) = {
        let playlist = &view.skin.playlist;
        (
            rgb(playlist.normal_background),
            rgb(playlist.selected_background),
        )
    };

    view.fill(0, 0, width, height, background);
    title_bar(view, focused);
    let mut y = TITLE_HEIGHT;
    while y < height {
        let clip = Area::new(0, TITLE_HEIGHT, width, height - TITLE_HEIGHT);
        view.sprite_clipped(sprites::PLAYLIST_LEFT_TILE, 0, y, clip);
        view.sprite_clipped(
            sprites::PLAYLIST_RIGHT_TILE,
            width - layout::PLAYLIST_RIGHT_WIDTH,
            y,
            clip,
        );
        y += layout::PLAYLIST_TILE_HEIGHT;
    }
    view.fill(0, height - 2, width, 2, frame);

    let state = app.jam.clone();
    let count = state.session().map(|session| session.members.len());
    let heading = match count {
        Some(count) => format!("JAM {}", count.max(1)),
        None => "JAM".to_string(),
    };
    let heading_x = (width - heading.chars().count() as u32 * 5) / 2;
    view.text(&heading, Area::new(heading_x, 7, width - heading_x, 6));

    let title = view.interact(
        Area::new(0, 0, width, TITLE_HEIGHT),
        "jam-title",
        Sense::click_and_drag(),
    );
    if title.drag_started() {
        view.ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }
    if title.double_clicked() {
        app.actions.push(Action::ToggleWinampJamShade);
    }
    if view
        .lamp_button(
            layout::PLAYLIST_SHADE,
            sprites::PLAYLIST_SHADE_PRESSED,
            false,
            "jam-shade",
        )
        .clicked()
    {
        app.actions.push(Action::ToggleWinampJamShade);
    }
    if view
        .lamp_button(
            layout::PLAYLIST_CLOSE,
            sprites::PLAYLIST_CLOSE_PRESSED,
            false,
            "jam-close",
        )
        .clicked()
    {
        app.actions.push(Action::ToggleWinampJam);
    }

    match state {
        State::Idle | State::Failed(_) => {
            let area = Area::new(88, 58, 100, 18);
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
                view.fill(16, y, 9, 9, frame);
                let initial: String = member
                    .display_name()
                    .chars()
                    .next()
                    .map(|character| character.to_uppercase().to_string())
                    .unwrap_or_default();
                view.text(&initial, Area::new(18, y + 2, 5, 6));
                view.text(member.display_name(), Area::new(28, y + 2, 210, 6));
            }
            super::pixel_line(app, view, &session.link(), Area::new(16, LINK_TOP, 243, 9));
            button(
                app,
                view,
                Area::new(16, BUTTON_TOP, 90, BUTTON_HEIGHT),
                "jam-copy",
                &gettext(app.locale, "Copy Jam link"),
                Action::CopyJamLink,
            );
            button(
                app,
                view,
                Area::new(112, BUTTON_TOP, 70, BUTTON_HEIGHT),
                "jam-end",
                &gettext(app.locale, "End the Jam"),
                Action::EndJam,
            );
        }
    }
}

/// The playlist's title bar without its word: corners, the tiles between
/// them, and the shade and close glyphs the corners carry.
fn title_bar(view: &mut View, focused: bool) {
    use sprites::*;
    let (left, tile, right) = if focused {
        (
            PLAYLIST_TOP_LEFT_ACTIVE,
            PLAYLIST_TOP_TILE_ACTIVE,
            PLAYLIST_TOP_RIGHT_ACTIVE,
        )
    } else {
        (PLAYLIST_TOP_LEFT, PLAYLIST_TOP_TILE, PLAYLIST_TOP_RIGHT)
    };
    let width = layout::WINDOW_WIDTH;
    let corner = layout::PLAYLIST_TILE_WIDTH;
    let run = Area::new(corner, 0, width - 2 * corner, TITLE_HEIGHT);
    view.sprite_at(left, 0, 0);
    let mut x = corner;
    while x < width - corner {
        view.sprite_clipped(tile, x, 0, run);
        x += corner;
    }
    view.sprite_at(right, width - corner, 0);
}

/// Rolled up: the playlist's shade bar with the Jam's count where the
/// playing time would be.
pub(super) fn shade(app: &mut App, view: &mut View, focused: bool) {
    use sprites::*;
    let width = layout::WINDOW_WIDTH;
    let height = layout::JAM_SHADE_HEIGHT;
    view.sprite_at(PLAYLIST_SHADE_LEFT, 0, 0);
    let run = Area::new(25, 0, width - 75, height);
    let mut x = 25;
    while x < width - 75 {
        view.sprite_clipped(PLAYLIST_SHADE_TILE, x, 0, run);
        x += 25;
    }
    let right = if focused {
        PLAYLIST_SHADE_RIGHT_ACTIVE
    } else {
        PLAYLIST_SHADE_RIGHT
    };
    view.sprite_at(right, width - 50, 0);

    let count = app.jam.session().map(|session| session.members.len());
    let heading = match count {
        Some(count) => format!("JAM {}", count.max(1)),
        None => "JAM".to_string(),
    };
    view.text(&heading, Area::new(5, 4, width - 60, 6));

    let title = view.interact(
        Area::new(0, 0, width, height),
        "jam-shade-title",
        Sense::click_and_drag(),
    );
    if title.drag_started() {
        view.ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }
    if view
        .lamp_button(
            layout::PLAYLIST_SHADE,
            sprites::PLAYLIST_UNSHADE_PRESSED,
            false,
            "jam-unshade",
        )
        .clicked()
    {
        app.actions.push(Action::ToggleWinampJamShade);
    }
    if view
        .lamp_button(
            layout::PLAYLIST_CLOSE,
            sprites::PLAYLIST_CLOSE_PRESSED,
            false,
            "jam-shade-close",
        )
        .clicked()
    {
        app.actions.push(Action::ToggleWinampJam);
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
    view.fill(area.x, area.y, area.width, 1, selected);
    view.fill(area.x, area.y + area.height - 1, area.width, 1, selected);
    view.fill(area.x, area.y, 1, area.height, selected);
    view.fill(area.x + area.width - 1, area.y, 1, area.height, selected);
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
    super::pixel_line(app, view, text, Area::new(16, 58, 243, 9));
}

fn rgb(color: [u8; 3]) -> Color32 {
    Color32::from_rgb(color[0], color[1], color[2])
}
