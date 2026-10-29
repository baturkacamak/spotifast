//! The playlist window, joined to the bottom of the main one.
//!
//! Uses `pledit.bmp` for the frame and `pledit.txt` for list colors. It shows
//! the complete playing collection; double-clicking plays a row. Unsupported file
//! controls remain decorative. The panel shares the mini-player window so the
//! two parts stay attached.

use egui::{Color32, Sense};

use crate::api::models::{PlayableItem, Track};
use crate::app::{App, NowPlaying};
use crate::i18n::{gettext, pgettext};
use crate::model::{Action, Page, RowContext};
use crate::skin::layout::{self, Area};
use crate::skin::sprites;
use crate::util;

use super::View;

/// One line of the list.
struct Row {
    uri: String,
    track: Option<Track>,
    label: String,
    duration_ms: u32,
    current: bool,
    /// Where the song sits in the queue, when it is queued rather than on.
    queued: Option<usize>,
    play: Option<(RowContext, u32)>,
    /// The pages the MISC menu can open for it.
    album_id: Option<String>,
    artist_id: Option<String>,
}

/// The whole panel, `height` skin pixels tall, drawn into a view whose
/// origin is the panel's top left.
pub(super) fn show(app: &mut App, view: &mut View, now: Option<&NowPlaying>, focused: bool) {
    if app.settings.playlist_shaded {
        shade(app, view, now, focused);
        return;
    }
    let height = app
        .settings
        .playlist_height
        .max(layout::PLAYLIST_MIN_HEIGHT);
    frame(view, height, focused);

    let title = view.interact(
        Area::new(0, 0, layout::WINDOW_WIDTH, layout::PLAYLIST_TITLE_HEIGHT),
        "playlist-title",
        Sense::click_and_drag(),
    );
    if title.drag_started() {
        view.ui
            .ctx()
            .send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
    if view
        .lamp_button(
            layout::PLAYLIST_CLOSE,
            sprites::PLAYLIST_CLOSE_PRESSED,
            false,
            "playlist-close",
        )
        .clicked()
    {
        app.actions.push(Action::ToggleWinampPlaylist);
    }
    if view
        .lamp_button(
            layout::PLAYLIST_SHADE,
            sprites::PLAYLIST_SHADE_PRESSED,
            false,
            "playlist-shade",
        )
        .clicked()
    {
        app.actions.push(Action::ToggleWinampPlaylistShade);
    }

    let (rows, queue_uris) = rows(app, now);
    tabs(app, view);
    search_input(app, view);
    list(app, view, &rows, height);
    scrollbar(app, view, rows.len(), height);
    grip(app, view, height);
    times(app, view, now, &rows, height);
    mini_transport(app, view, now, height);
    menus(app, view, &rows, &queue_uris, height);
}

/// The little transport in the bottom right: the same as the big one,
/// painted into the skin, so these are only places to click.
fn mini_transport(app: &mut App, view: &mut View, now: Option<&NowPlaying>, height: u32) {
    let (x, dy) = layout::PLAYLIST_MINI_TRANSPORT;
    let y = height - layout::PLAYLIST_BOTTOM_HEIGHT + dy;
    let playing = now.is_some_and(|now| now.playing);
    for (index, name) in ["previous", "play", "pause", "stop", "next", "eject"]
        .into_iter()
        .enumerate()
    {
        let cell = Area::new(x + 10 * index as u32, y, 10, 10);
        let response = view.interact(cell, &format!("playlist-{name}"), Sense::click());
        if !response.clicked() {
            continue;
        }
        match name {
            "previous" => app.actions.push(Action::Previous),
            "play" => app.actions.push(if playing {
                Action::Seek(0)
            } else {
                Action::TogglePlay
            }),
            "pause" if now.is_some() => app.actions.push(Action::TogglePlay),
            "stop" if now.is_some() => {
                if playing {
                    app.actions.push(Action::TogglePlay);
                }
                app.actions.push(Action::Seek(0));
            }
            "next" => app.actions.push(Action::Next),
            "eject" => app.actions.push(Action::ToggleWinampWindow),
            _ => {}
        }
    }
}

/// The playlist rolled up: a bar with the song's name and time, the
/// button to roll it down again, and its X.
fn shade(app: &mut App, view: &mut View, now: Option<&NowPlaying>, focused: bool) {
    use sprites::*;
    let width = layout::WINDOW_WIDTH;
    let height = layout::PLAYLIST_SHADE_HEIGHT;
    view.sprite_at(PLAYLIST_SHADE_LEFT, 0, 0);
    let mut x = 25;
    while x < width - 50 {
        view.sprite_clipped(
            PLAYLIST_SHADE_TILE,
            x,
            0,
            Area::new(25, 0, width - 75, height),
        );
        x += 25;
    }
    let right = if focused {
        PLAYLIST_SHADE_RIGHT_ACTIVE
    } else {
        PLAYLIST_SHADE_RIGHT
    };
    view.sprite_at(right, width - 50, 0);
    let title = view.interact(
        Area::new(0, 0, width, height),
        "playlist-shade-title",
        Sense::click_and_drag(),
    );
    if title.drag_started() {
        view.ui
            .ctx()
            .send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
    if let Some(now) = now {
        let time = util::format_duration_ms(now.position_ms);
        let time_width = 5 * time.len() as u32;
        let time_x = width - 30 - time_width;
        view.text(&time, Area::new(time_x, 4, time_width, 6));
        let name = label(1, &now.subtitle, &now.title);
        let name_area = Area::new(5, 4, time_x - 5 - 5, 6);
        if name.chars().all(crate::skin::font::covered) {
            view.text(&name, name_area);
        } else {
            super::pixel_line(app, view, &name, name_area);
        }
    }
    if view
        .lamp_button(
            layout::PLAYLIST_SHADE,
            sprites::PLAYLIST_UNSHADE_PRESSED,
            false,
            "playlist-unshade",
        )
        .clicked()
    {
        app.actions.push(Action::ToggleWinampPlaylistShade);
    }
    if view
        .lamp_button(
            layout::PLAYLIST_CLOSE,
            sprites::PLAYLIST_CLOSE_PRESSED,
            false,
            "playlist-shade-close",
        )
        .clicked()
    {
        app.actions.push(Action::ToggleWinampPlaylist);
    }
}

/// The frame: corners and tiles across the top, tiles down the sides
/// however tall the list is, and the two halves of the bottom.
fn frame(view: &mut View, height: u32, focused: bool) {
    use sprites::*;
    let (top_left, title, top_tile, top_right) = if focused {
        (
            PLAYLIST_TOP_LEFT_ACTIVE,
            PLAYLIST_TITLE_ACTIVE,
            PLAYLIST_TOP_TILE_ACTIVE,
            PLAYLIST_TOP_RIGHT_ACTIVE,
        )
    } else {
        (
            PLAYLIST_TOP_LEFT,
            PLAYLIST_TITLE,
            PLAYLIST_TOP_TILE,
            PLAYLIST_TOP_RIGHT,
        )
    };
    // The title sits centred; the tiles either side of it run out to the
    // corners, the last on each side cut to fit, as Winamp cut them.
    let tile = layout::PLAYLIST_TILE_WIDTH;
    let width = layout::WINDOW_WIDTH;
    let inner = width - 2 * tile - 100;
    let left = inner / 2;
    let right = inner - left;
    view.sprite_at(top_left, 0, 0);
    let left_run = Area::new(tile, 0, left, layout::PLAYLIST_TITLE_HEIGHT);
    let mut x = tile;
    while x < tile + left {
        view.sprite_clipped(top_tile, x, 0, left_run);
        x += tile;
    }
    view.sprite_at(title, tile + left, 0);
    let right_run = Area::new(tile + left + 100, 0, right, layout::PLAYLIST_TITLE_HEIGHT);
    let mut x = tile + left + 100;
    while x < width - tile {
        view.sprite_clipped(top_tile, x, 0, right_run);
        x += tile;
    }
    view.sprite_at(top_right, width - tile, 0);

    let middle = Area::new(
        0,
        layout::PLAYLIST_TITLE_HEIGHT,
        layout::WINDOW_WIDTH,
        height - layout::PLAYLIST_TITLE_HEIGHT - layout::PLAYLIST_BOTTOM_HEIGHT,
    );
    let mut y = middle.y;
    while y < middle.y + middle.height {
        view.sprite_clipped(PLAYLIST_LEFT_TILE, 0, y, middle);
        view.sprite_clipped(
            PLAYLIST_RIGHT_TILE,
            layout::WINDOW_WIDTH - layout::PLAYLIST_RIGHT_WIDTH,
            y,
            middle,
        );
        y += layout::PLAYLIST_TILE_HEIGHT;
    }
    let bottom = height - layout::PLAYLIST_BOTTOM_HEIGHT;
    view.sprite_at(PLAYLIST_BOTTOM_LEFT, 0, bottom);
    view.sprite_at(PLAYLIST_BOTTOM_RIGHT, 125, bottom);
}

/// Tabs share the skin's list area, with one close button per temporary view.
struct Tab {
    label: String,
    tooltip: String,
    width: u32,
    selected: bool,
    action: Action,
    close: Option<Action>,
}

fn tabs(app: &mut App, view: &mut View) {
    let search = &app.winamp.search_tab;
    if app.winamp.radio_tab.is_none() && !search.open {
        return;
    }
    let radio_showing = app.winamp.radio_tab.as_ref().is_some_and(|tab| tab.showing);
    let mut tabs = vec![Tab {
        label: gettext(app.locale, "Playlist").into_owned(),
        tooltip: gettext(app.locale, "Playlist").into_owned(),
        width: 70,
        selected: !radio_showing && !search.showing,
        action: Action::ShowWinampRadio(false),
        close: None,
    }];
    if let Some(radio) = &app.winamp.radio_tab {
        let title = app
            .radio_name(&radio.seed)
            .unwrap_or_else(|| gettext(app.locale, "Radio").into_owned());
        tabs.push(Tab {
            label: if search.open {
                gettext(app.locale, "Radio").into_owned()
            } else {
                title.clone()
            },
            tooltip: title,
            width: if search.open { 82 } else { 173 },
            selected: radio_showing,
            action: Action::ShowWinampRadio(true),
            close: Some(Action::CloseWinampRadio),
        });
    }
    if search.open {
        tabs.push(Tab {
            label: gettext(app.locale, "Search").into_owned(),
            tooltip: gettext(app.locale, "Search Spotify").into_owned(),
            width: if app.winamp.radio_tab.is_some() {
                91
            } else {
                173
            },
            selected: search.showing,
            action: Action::OpenWinampSearch,
            close: Some(Action::CloseWinampSearch),
        });
    }
    let style = view.skin.playlist.clone();
    let mut x = layout::PLAYLIST_LEFT_WIDTH;
    for (index, tab) in tabs.into_iter().enumerate() {
        let area = Area::new(
            x,
            layout::PLAYLIST_TITLE_HEIGHT,
            tab.width,
            layout::PLAYLIST_TRACK_HEIGHT,
        );
        let rgb = if tab.selected {
            style.selected_background
        } else {
            style.normal_background
        };
        view.fill(
            area.x,
            area.y,
            area.width,
            area.height,
            Color32::from_rgb(rgb[0], rgb[1], rgb[2]),
        );
        let close_width = if tab.close.is_some() { 12 } else { 0 };
        let label_area = Area::new(x, area.y, area.width - close_width, area.height);
        super::pixel_line(
            app,
            view,
            &tab.label,
            Area::new(x + 3, area.y + 4, label_area.width - 6, 6),
        );
        if view
            .interact(label_area, &format!("playlist-tab-{index}"), Sense::click())
            .on_hover_text(&tab.tooltip)
            .clicked()
        {
            app.actions.push(tab.action);
        }
        if let Some(action) = tab.close {
            let close = Area::new(x + area.width - 12, area.y, 12, area.height);
            view.text("X", Area::new(close.x + 3, close.y + 4, 5, 6));
            if view
                .interact(
                    close,
                    &format!("playlist-tab-close-{index}"),
                    Sense::click(),
                )
                .on_hover_text(gettext(app.locale, "Close").as_ref())
                .clicked()
            {
                app.actions.push(action);
            }
        }
        x += area.width;
    }
}

fn search_input(app: &mut App, view: &mut View) {
    if !app.winamp.search_tab.showing {
        return;
    }
    let area = Area::new(
        layout::PLAYLIST_LEFT_WIDTH,
        layout::PLAYLIST_TITLE_HEIGHT + layout::PLAYLIST_TRACK_HEIGHT,
        243,
        layout::PLAYLIST_TRACK_HEIGHT,
    );
    let rgb = |[r, g, b]: [u8; 3]| Color32::from_rgb(r, g, b);
    let style = view.skin.playlist.clone();
    view.fill(
        area.x,
        area.y,
        area.width,
        area.height,
        rgb(style.normal_background),
    );
    view.fill(
        area.x,
        area.y + area.height - 1,
        area.width,
        1,
        rgb(style.selected_background),
    );
    let mut query = app.winamp.search_tab.query.clone();
    let rect = view.rect(area);
    let response = view
        .ui
        .scope(|ui| {
            ui.spacing_mut().interact_size.y = rect.height();
            ui.visuals_mut().selection.bg_fill = rgb(style.selected_background);
            ui.visuals_mut().text_cursor.stroke.color = rgb(style.current);
            ui.put(
                rect,
                egui::TextEdit::singleline(&mut query)
                    .id(egui::Id::new("winamp-search-query"))
                    .font(egui::FontId::monospace(8.0 * view.unit))
                    .text_color(rgb(style.normal))
                    .background_color(rgb(style.normal_background))
                    .frame(egui::Frame::NONE)
                    .margin(egui::vec2(3.0 * view.unit, 1.0 * view.unit))
                    .hint_text(gettext(app.locale, "Search Spotify").as_ref()),
            )
        })
        .inner;
    if app.winamp.search_tab.focus_requested {
        response.request_focus();
        app.actions.push(Action::WinampSearchFocused);
    }
    if response.changed() {
        app.actions.push(Action::WinampSearchQuery(query));
    }
    if response.lost_focus() && view.ui.input(|input| input.key_pressed(egui::Key::Enter)) {
        app.actions.push(Action::SubmitWinampSearch);
    }
}

fn row_menu(app: &mut App, ui: &mut egui::Ui, row: &Row) {
    if app.winamp.search_tab.showing
        && let Some(track) = &row.track
    {
        if ui
            .add_enabled(
                app.can_queue(&row.uri),
                egui::Button::new(gettext(app.locale, "Add to queue").as_ref()),
            )
            .clicked()
        {
            app.actions.push(Action::AddToQueue {
                uri: row.uri.clone(),
                label: track.name.clone(),
            });
            ui.close();
        }
        if ui
            .add_enabled(
                app.can_queue(&row.uri) && app.queue_locally_reorderable(),
                egui::Button::new(gettext(app.locale, "Play next").as_ref()),
            )
            .clicked()
        {
            app.actions.push(Action::PlayNext {
                items: vec![PlayableItem::Track(track.clone())],
            });
            ui.close();
        }
    }
    song_radio_menu(app, ui, row);
}

fn song_radio_menu(app: &mut App, ui: &mut egui::Ui, row: &Row) {
    if let Some(track) = row
        .track
        .as_ref()
        .filter(|_| row.uri.starts_with("spotify:track:"))
        && ui
            .button(gettext(app.locale, "Go to song radio").as_ref())
            .clicked()
    {
        app.actions.push(Action::OpenWinampRadio {
            uri: row.uri.clone(),
            track: Box::new(track.clone()),
        });
        ui.close();
    }
}

fn empty_status(app: &mut App, view: &mut View, area: Area) {
    let search = &app.winamp.search_tab;
    let (message, retry) = if search.showing {
        match &search.songs {
            crate::model::Loadable::Failed(error) => (
                format!("{}: {error}", gettext(app.locale, "Retry")),
                Some(Action::SubmitWinampSearch),
            ),
            crate::model::Loadable::Loaded(_) => {
                (gettext(app.locale, "Nothing here yet.").into_owned(), None)
            }
            _ if search.query.trim().is_empty() => {
                (gettext(app.locale, "Search Spotify").into_owned(), None)
            }
            _ => (gettext(app.locale, "Loading…").into_owned(), None),
        }
    } else if let Some(tab) = app.winamp.radio_tab.as_ref().filter(|tab| tab.showing) {
        let state = app.radio_pages.get(&tab.seed).map(|page| &page.songs);
        match state {
            Some(crate::model::Loadable::Failed(error)) => (
                format!("{}: {error}", gettext(app.locale, "Retry")),
                Some(Action::Reload(Page::Radio(tab.seed.clone()))),
            ),
            Some(crate::model::Loadable::Loaded(_)) => {
                (gettext(app.locale, "Nothing here yet.").into_owned(), None)
            }
            _ => (gettext(app.locale, "Loading…").into_owned(), None),
        }
    } else {
        return;
    };
    super::pixel_line(
        app,
        view,
        &message,
        Area::new(area.x + 3, area.y + 4, area.width - 6, 6),
    );
    if let Some(action) = retry
        && view
            .interact(area, "playlist-retry", Sense::click())
            .on_hover_text(&message)
            .clicked()
    {
        app.actions.push(action);
    }
}

fn header_rows(app: &App) -> u32 {
    u32::from(app.winamp.radio_tab.is_some() || app.winamp.search_tab.open)
        + u32::from(app.winamp.search_tab.showing)
}

/// The list's own area, between the tiles.
fn list_area(height: u32, header_rows: u32) -> Area {
    let tab_height = header_rows * layout::PLAYLIST_TRACK_HEIGHT;
    Area::new(
        layout::PLAYLIST_LEFT_WIDTH,
        layout::PLAYLIST_TITLE_HEIGHT + tab_height,
        layout::WINDOW_WIDTH - layout::PLAYLIST_LEFT_WIDTH - layout::PLAYLIST_RIGHT_WIDTH,
        height - layout::PLAYLIST_TITLE_HEIGHT - layout::PLAYLIST_BOTTOM_HEIGHT - tab_height,
    )
}

fn rows_visible(height: u32, header_rows: u32) -> usize {
    (list_area(height, header_rows).height / layout::PLAYLIST_TRACK_HEIGHT) as usize
}

/// The full collection, or retained loose list. The live queue remains
/// separate for queue-specific actions and saving it as a playlist.
fn rows(app: &App, now: Option<&NowPlaying>) -> (Vec<Row>, Vec<String>) {
    let queue = app.queue.get();
    if app.winamp.search_tab.showing {
        let songs = app
            .winamp
            .search_tab
            .songs
            .get()
            .map(Vec::as_slice)
            .unwrap_or_default();
        let rows = songs
            .iter()
            .enumerate()
            .map(|(index, track)| {
                // Search plays one song, as in the main window. The other
                // results are browsing choices, not a playback queue.
                let context = RowContext::Uris(std::sync::Arc::from([track.uri.clone()]));
                let mut row = collection_row(
                    app,
                    &context,
                    index as u32,
                    &PlayableItem::Track(track.clone()),
                    now,
                );
                if row.play.is_some() {
                    row.play = Some((context, 0));
                }
                row
            })
            .collect();
        return (
            rows,
            queue
                .map(|q| q.queue.iter().map(|item| item.uri().to_owned()).collect())
                .unwrap_or_default(),
        );
    }
    if let Some(tab) = app.winamp.radio_tab.as_ref().filter(|tab| tab.showing) {
        let rows = app
            .radio_pages
            .get(&tab.seed)
            .and_then(|page| {
                let songs = page.songs.get()?;
                let context_uri = page.uri.clone().or_else(|| util::station_uri(&tab.seed))?;
                let context = RowContext::View {
                    uris: songs.iter().map(|song| song.uri.clone()).collect(),
                    context_uri,
                    editable_playlist: None,
                };
                Some(
                    songs
                        .iter()
                        .enumerate()
                        .map(|(index, track)| {
                            collection_row(
                                app,
                                &context,
                                index as u32,
                                &PlayableItem::Track(track.clone()),
                                now,
                            )
                        })
                        .collect(),
                )
            })
            .unwrap_or_default();
        return (
            rows,
            queue
                .map(|q| q.queue.iter().map(|item| item.uri().to_owned()).collect())
                .unwrap_or_default(),
        );
    }
    if let Some((context, items)) = app.winamp_collection() {
        let rows = items
            .into_iter()
            .map(|(index, item)| collection_row(app, &context, index, &item, now))
            .collect();
        return (
            rows,
            queue
                .map(|q| q.queue.iter().map(|item| item.uri().to_owned()).collect())
                .unwrap_or_default(),
        );
    }
    if !app.winamp.playlist_history.is_empty() {
        let uris: std::sync::Arc<[String]> = app
            .winamp
            .playlist_history
            .iter()
            .map(|item| item.uri().to_owned())
            .collect();
        let context = RowContext::Uris(uris);
        let rows = app
            .winamp
            .playlist_history
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let mut row = collection_row(app, &context, index as u32, item, now);
                row.current = app.winamp.playlist_cursor == Some(index)
                    && now.is_some_and(|now| now.uri == item.uri());
                row.queued = app
                    .winamp
                    .playlist_cursor
                    .and_then(|cursor| index.checked_sub(cursor + 1));
                row
            })
            .collect();
        return (
            rows,
            queue
                .map(|q| q.queue.iter().map(|item| item.uri().to_owned()).collect())
                .unwrap_or_default(),
        );
    }
    let ids = |item: &PlayableItem| match item {
        PlayableItem::Track(track) => (
            track.album.as_ref().map(|album| album.id.clone()),
            track.artists.first().and_then(|artist| artist.id.clone()),
        ),
        _ => (None, None),
    };
    let mut rows = Vec::new();
    // The queue arrives over the Web API and lags a double-click by a
    // round trip; what the player says is on right now wins, and a queued
    // copy of that song is left out until the fetched queue catches up.
    let current = queue
        .and_then(|queue| queue.currently_playing.as_ref())
        .filter(|item| now.is_none_or(|now| now.uri == item.uri()));
    if let Some(item) = current {
        let (album_id, artist_id) = ids(item);
        rows.push(Row {
            uri: item.uri().to_string(),
            track: match item {
                PlayableItem::Track(track) => Some(track.clone()),
                _ => None,
            },
            label: label(1, &item.subtitle(), item.name()),
            duration_ms: item.duration_ms(),
            current: true,
            queued: None,
            play: None,
            album_id,
            artist_id,
        });
    } else if let Some(now) = now {
        rows.push(Row {
            uri: now.uri.clone(),
            track: app.now_playing_item().and_then(|item| match item {
                PlayableItem::Track(track) => Some(track),
                _ => None,
            }),
            label: label(1, &now.subtitle, &now.title),
            duration_ms: now.duration_ms,
            current: true,
            queued: None,
            play: None,
            album_id: now.album_id.clone(),
            artist_id: now.artists.first().and_then(|artist| artist.id.clone()),
        });
    }
    let queued: &[PlayableItem] = queue.map(|queue| queue.queue.as_slice()).unwrap_or(&[]);
    let stale = current.is_none() && queue.is_some();
    let mut skipped_playing = false;
    for (index, item) in queued.iter().enumerate() {
        if stale && !skipped_playing && now.is_some_and(|now| now.uri == item.uri()) {
            skipped_playing = true;
            continue;
        }
        let (album_id, artist_id) = ids(item);
        rows.push(Row {
            uri: item.uri().to_string(),
            track: match item {
                PlayableItem::Track(track) => Some(track.clone()),
                _ => None,
            },
            label: label(rows.len() + 1, &item.subtitle(), item.name()),
            duration_ms: item.duration_ms(),
            current: false,
            queued: Some(index),
            play: Some((RowContext::Queue, index as u32)),
            album_id,
            artist_id,
        });
    }
    let uris = queued.iter().map(|item| item.uri().to_string()).collect();
    (rows, uris)
}

fn collection_row(
    app: &App,
    context: &RowContext,
    index: u32,
    item: &PlayableItem,
    now: Option<&NowPlaying>,
) -> Row {
    let (album_id, artist_id) = match item {
        PlayableItem::Track(track) => (
            track.album.as_ref().map(|album| album.id.clone()),
            track.artists.first().and_then(|artist| artist.id.clone()),
        ),
        _ => (None, None),
    };
    Row {
        uri: item.uri().to_owned(),
        track: match item {
            PlayableItem::Track(track) => Some(track.clone()),
            _ => None,
        },
        label: label(index as usize + 1, &item.subtitle(), item.name()),
        duration_ms: item.duration_ms(),
        current: now.is_some_and(|now| now.uri == item.uri())
            && app.row_is_played_copy(context, index as usize, item.uri()),
        queued: None,
        play: app.row_playable(item).then(|| (context.clone(), index)),
        album_id,
        artist_id,
    }
}

fn label(number: usize, subtitle: &str, title: &str) -> String {
    if subtitle.is_empty() {
        format!("{number}. {title}")
    } else {
        format!("{number}. {subtitle} - {title}")
    }
}

/// The rows, in the skin's colours and a font the size Winamp's was.
fn list(app: &mut App, view: &mut View, rows: &[Row], height: u32) {
    let style = view.skin.playlist.clone();
    let rgb = |color: [u8; 3]| Color32::from_rgb(color[0], color[1], color[2]);
    let area = list_area(height, header_rows(app));
    view.fill(
        area.x,
        area.y,
        area.width,
        area.height,
        rgb(style.normal_background),
    );

    let visible = rows_visible(height, header_rows(app));
    let most = rows.len().saturating_sub(visible);
    // The wheel, over the list.
    // `contains_pointer`, not `hovered`: the rows drawn on top of the list
    // would take the hover and the wheel with it.
    let hovered = view
        .interact(area, "playlist-list", Sense::hover())
        .contains_pointer();
    if hovered {
        let row_points = layout::PLAYLIST_TRACK_HEIGHT as f32 * view.unit;
        let (lines, points, pages) = view.ui.ctx().input(|input| {
            input
                .events
                .iter()
                .fold((0.0, 0.0, 0.0), |sum, event| match event {
                    egui::Event::MouseWheel { unit, delta, .. } => match unit {
                        // One detent is one event however many lines the
                        // system says it is worth; a free-spinning wheel's
                        // fractions still add up.
                        egui::MouseWheelUnit::Line => {
                            let notch = if delta.y.abs() >= 1.0 {
                                delta.y.signum()
                            } else {
                                delta.y
                            };
                            (sum.0 + notch, sum.1, sum.2)
                        }
                        egui::MouseWheelUnit::Point => (sum.0, sum.1 + delta.y, sum.2),
                        egui::MouseWheelUnit::Page => (sum.0, sum.1, sum.2 + delta.y),
                    },
                    _ => sum,
                })
        });
        app.winamp.playlist_wheel += rows_for_wheel(lines, points, pages, row_points, visible);
        let rows_moved = app.winamp.playlist_wheel.trunc();
        if rows_moved != 0.0 {
            app.winamp.playlist_wheel -= rows_moved;
            let scroll = app.winamp.playlist_scroll as i64 + rows_moved as i64;
            app.winamp.playlist_scroll = scroll.clamp(0, most as i64) as usize;
        }
    }
    app.winamp.playlist_scroll = app.winamp.playlist_scroll.min(most);
    let scroll = app.winamp.playlist_scroll;
    if rows.is_empty() {
        empty_status(app, view, area);
    }
    let autoscroll_rect = view.rect(area);
    crate::autoscroll::playlist(
        view.ui,
        autoscroll_rect,
        scroll,
        most,
        layout::PLAYLIST_TRACK_HEIGHT as f32 * view.unit,
    );

    let ctx = view.ui.ctx().clone();
    let clip = view.rect(area).intersect(view.ui.clip_rect());
    let painter = view.ui.painter().with_clip_rect(clip);
    let pad = 3;
    for (offset, row) in rows.iter().skip(scroll).take(visible).enumerate() {
        let line = Area::new(
            area.x,
            area.y + offset as u32 * layout::PLAYLIST_TRACK_HEIGHT,
            area.width,
            layout::PLAYLIST_TRACK_HEIGHT,
        );
        let duration = util::format_duration_ms(row.duration_ms);
        let duration_width = app.winamp.playlist_text.width(&duration).ceil() as u32;
        let heart_width = if row.uri.starts_with("spotify:track:") {
            11
        } else {
            0
        };
        let heart = Area::new(
            line.x + line.width - pad - duration_width - heart_width,
            line.y,
            heart_width,
            line.height,
        );
        let rect = view.rect(line);
        let response = view.ui.interact(
            rect,
            egui::Id::new(("playlist-row", scroll + offset)),
            Sense::click(),
        );
        crate::autoscroll::row(view.ui, &response);
        super::menu(
            egui::Popup::context_menu(&response),
            view.skin,
            view.unit,
            |ui| row_menu(app, ui, row),
        );
        let on_heart = heart_width > 0
            && response
                .interact_pointer_pos()
                .is_some_and(|pos| view.rect(heart).contains(pos));
        if response.clicked() && !on_heart {
            let adding = view.ui.ctx().input(|input| input.modifiers.command);
            // Selection is by row, not by song: the same song can sit in
            // the list more than once and one click means one row.
            let index = scroll + offset;
            let selection = &mut app.winamp.playlist_selection;
            if adding {
                if !selection.remove(&index) {
                    selection.insert(index);
                }
            } else {
                selection.clear();
                selection.insert(index);
            }
        }
        if response.double_clicked()
            && !on_heart
            && let Some((context, index)) = &row.play
        {
            app.actions.push(Action::PlayFromRow {
                context: context.clone(),
                uri: row.uri.clone(),
                index: *index,
            });
        }
        if app.winamp.playlist_selection.contains(&(scroll + offset)) {
            painter.rect_filled(rect, 0.0, rgb(style.selected_background));
        }
        let color = rgb(if row.current {
            style.current
        } else {
            style.normal
        });
        let text = &mut app.winamp.playlist_text;
        let title_room = line
            .width
            .saturating_sub(3 * pad + duration_width + heart_width);
        let title = fit(text, &row.label, title_room as f32);
        let duration_line = text.line(&ctx, &duration);
        let duration_at = Area::new(
            line.x + line.width - pad - duration_line.width,
            line.y + (line.height.saturating_sub(duration_line.height)) / 2,
            duration_line.width,
            duration_line.height,
        );
        painter.image(
            duration_line.texture.id(),
            view.rect(duration_at),
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            color,
        );
        let title_line = text.line(&ctx, &title);
        let title_at = Area::new(
            line.x + pad,
            line.y + (line.height.saturating_sub(title_line.height)) / 2,
            title_line.width.min(title_room),
            title_line.height,
        );
        let uv_right = title_at.width as f32 / title_line.width.max(1) as f32;
        painter.image(
            title_line.texture.id(),
            view.rect(title_at),
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(uv_right, 1.0)),
            color,
        );
        if heart_width > 0 {
            super::track_like_button(app, view, &row.uri, heart, false);
        }
    }
}

/// How many rows a frame's wheel moves the list, positive downwards. A
/// notch scrolls three rows, the Windows default Winamp's list followed;
/// a trackpad's points scroll a row per row's height; a page is the rows
/// in view. egui's deltas are positive when the content moves down, which
/// is scrolling up.
fn rows_for_wheel(lines: f32, points: f32, pages: f32, row_points: f32, visible: usize) -> f32 {
    -(lines * 3.0 + points / row_points + pages * visible as f32)
}

/// Text cut to a width, with an ellipsis when it had to be.
fn fit(text: &mut super::PixelText, label: &str, width: f32) -> String {
    if text.width(label) <= width {
        return label.to_string();
    }
    let chars: Vec<char> = label.chars().collect();
    let mut keep = chars.len();
    while keep > 0 {
        keep -= 1;
        let candidate: String = chars[..keep].iter().collect::<String>() + "\u{2026}";
        if text.width(&candidate) <= width {
            return candidate;
        }
    }
    "\u{2026}".to_string()
}

/// The handle in the right-hand tiles, dragged to scroll.
fn scrollbar(app: &mut App, view: &mut View, rows: usize, height: u32) {
    let visible = rows_visible(height, header_rows(app));
    let most = rows.saturating_sub(visible);
    let top = list_area(height, header_rows(app)).y;
    let travel = height
        .saturating_sub(top + layout::PLAYLIST_BOTTOM_HEIGHT)
        .saturating_sub(layout::PLAYLIST_SCROLL_HANDLE_HEIGHT);
    let fraction = if most == 0 {
        0.0
    } else {
        app.winamp.playlist_scroll as f32 / most as f32
    };
    let y = top + (fraction * travel as f32).round() as u32;
    let handle = Area::new(
        layout::PLAYLIST_SCROLL_X,
        y,
        8,
        layout::PLAYLIST_SCROLL_HANDLE_HEIGHT,
    );
    let response = view.interact(handle, "playlist-scroll", Sense::click_and_drag());
    if response.dragged()
        && most > 0
        && let Some(pos) = response.interact_pointer_pos()
    {
        let pointer = (pos.y - view.origin.y) / view.unit
            - top as f32
            - layout::PLAYLIST_SCROLL_HANDLE_HEIGHT as f32 / 2.0;
        let fraction = (pointer / travel as f32).clamp(0.0, 1.0);
        app.winamp.playlist_scroll = (fraction * most as f32).round() as usize;
    }
    let sprite = if response.dragged() || response.is_pointer_button_down_on() {
        sprites::PLAYLIST_SCROLL_HANDLE_PRESSED
    } else {
        sprites::PLAYLIST_SCROLL_HANDLE
    };
    view.sprite(sprite, handle);
}

/// The corner that stretches the list, a tile row at a time.
fn grip(app: &mut App, view: &mut View, height: u32) {
    let corner = Area::new(
        layout::WINDOW_WIDTH - layout::PLAYLIST_GRIP,
        height - layout::PLAYLIST_GRIP,
        layout::PLAYLIST_GRIP,
        layout::PLAYLIST_GRIP,
    );
    let response = view
        .interact(corner, "playlist-grip", Sense::drag())
        .on_hover_cursor(egui::CursorIcon::ResizeVertical);
    if response.dragged() {
        app.winamp.playlist_resize += response.drag_delta().y / view.unit;
        let step = layout::PLAYLIST_RESIZE_STEP as f32;
        let steps = (app.winamp.playlist_resize / step).trunc();
        if steps != 0.0 {
            app.winamp.playlist_resize -= steps * step;
            let wanted = (height as i64 + steps as i64 * step as i64).clamp(
                layout::PLAYLIST_MIN_HEIGHT as i64,
                layout::PLAYLIST_MAX_HEIGHT as i64,
            ) as u32;
            if wanted != height {
                app.actions.push(Action::SetPlaylistHeight(wanted));
            }
        }
    }
    if response.drag_stopped() {
        app.winamp.playlist_resize = 0.0;
    }
}

/// The selected songs' time over the queue's total, as Winamp's list
/// showed them, and the song's own time where Winamp kept it, in the
/// skin's small font.
fn times(app: &App, view: &mut View, now: Option<&NowPlaying>, rows: &[Row], height: u32) {
    let bottom = height - layout::PLAYLIST_BOTTOM_HEIGHT;
    let total: u32 = rows.iter().map(|row| row.duration_ms).sum();
    let selected: u32 = rows
        .iter()
        .enumerate()
        .filter(|(index, _)| app.winamp.playlist_selection.contains(index))
        .map(|(_, row)| row.duration_ms)
        .sum();
    let running = format!(
        "{}/{}",
        util::format_duration_ms(selected),
        util::format_duration_ms(total)
    );
    let (x, dy) = layout::PLAYLIST_RUNNING_TIME;
    view.text(
        &running,
        Area::new(x, bottom + dy, 5 * running.len() as u32, 6),
    );
    if let Some(now) = now {
        let (x, dy) = layout::PLAYLIST_TRACK_TIME;
        let elapsed = util::format_duration_ms(now.position_ms);
        view.text(
            &elapsed,
            Area::new(x, bottom + dy, 5 * elapsed.len() as u32, 6),
        );
    }
}

/// Bottom menus adapted to supported Spotify actions.
fn menus(app: &mut App, view: &mut View, rows: &[Row], queue_uris: &[String], height: u32) {
    let bottom = height - layout::PLAYLIST_BOTTOM_HEIGHT;
    let unit = view.unit;
    for (name, x) in layout::PLAYLIST_MENUS {
        let area = Area::new(x, bottom + 8, 22, 18);
        let button = view.interact(area, &format!("playlist-menu-{name}"), Sense::click());
        super::menu(
            egui::Popup::menu(&button),
            view.skin,
            unit,
            |ui| match name {
                "add" => add_menu(app, ui),
                "rem" => rem_menu(app, ui, rows),
                "sel" => sel_menu(app, ui, rows),
                "misc" => misc_menu(app, ui, rows),
                _ => list_menu(app, ui, rows, queue_uris),
            },
        );
    }
}

/// Where a song comes from here: the big window's search or Liked Songs.
fn add_menu(app: &mut App, ui: &mut egui::Ui) {
    let locale = app.locale;
    if ui
        .button(gettext(locale, "Search Spotify").as_ref())
        .clicked()
    {
        app.actions.push(Action::OpenWinampSearch);
    }
    if ui.button(gettext(locale, "Liked Songs").as_ref()).clicked() {
        app.actions.push(Action::Open(Page::LikedSongs));
        app.actions.push(Action::ToggleWinampWindow);
    }
}

fn rem_menu(app: &mut App, ui: &mut egui::Ui, rows: &[Row]) {
    let locale = app.locale;
    // Selection is by list row; only the songs the user queued can go.
    let removable: Vec<(usize, String)> = rows
        .iter()
        .enumerate()
        .filter(|(row, _)| app.winamp.playlist_selection.contains(row))
        .filter_map(|(_, row)| row.queued.map(|index| (index, row.uri.clone())))
        .filter(|(index, _)| app.can_remove_from_queue(*index))
        .collect();
    let mut remove = ui.add_enabled(
        !removable.is_empty(),
        egui::Button::new(gettext(locale, "Remove selected").as_ref()),
    );
    if !app.queue_locally_reorderable() {
        remove = remove.on_disabled_hover_text(
            gettext(locale, "You can only change this computer's queue.").as_ref(),
        );
    }
    if remove.clicked() {
        // The rows below move up, so the selection's indexes would name
        // other songs.
        app.winamp.playlist_selection.clear();
        app.actions
            .push(Action::RemoveFromQueue { rows: removable });
    }
    if ui
        .add_enabled(
            app.can_clear_queue(),
            egui::Button::new(gettext(locale, "Remove all").as_ref()),
        )
        .on_disabled_hover_text(
            gettext(locale, "You can only clear this computer's queue.").as_ref(),
        )
        .clicked()
    {
        app.actions.push(Action::ClearQueue);
    }
}

fn sel_menu(app: &mut App, ui: &mut egui::Ui, rows: &[Row]) {
    let locale = app.locale;
    // Translators: Selects every song in the list.
    let all = pgettext(locale, "selection", "All");
    // Translators: Clears the selection of songs in the list.
    let none = pgettext(locale, "selection", "None");
    // Translators: Selects the songs that are not selected, and clears the rest.
    let invert = pgettext(locale, "selection", "Invert");
    let selection = &mut app.winamp.playlist_selection;
    if ui.button(all.as_ref()).clicked() {
        selection.extend(0..rows.len());
    }
    if ui.button(none.as_ref()).clicked() {
        selection.clear();
    }
    if ui.button(invert.as_ref()).clicked() {
        for index in 0..rows.len() {
            if !selection.remove(&index) {
                selection.insert(index);
            }
        }
    }
}

/// The pages of the selected song, or of the one playing.
fn misc_menu(app: &mut App, ui: &mut egui::Ui, rows: &[Row]) {
    let locale = app.locale;
    let chosen = rows
        .iter()
        .enumerate()
        .find(|(index, _)| app.winamp.playlist_selection.contains(index))
        .map(|(_, row)| row)
        .or_else(|| rows.iter().find(|row| row.current));
    let Some(row) = chosen else {
        ui.add_enabled(
            false,
            egui::Button::new(gettext(locale, "Song info").as_ref()),
        );
        return;
    };
    song_radio_menu(app, ui, row);
    if let Some(album) = &row.album_id
        && ui.button(gettext(locale, "Song info").as_ref()).clicked()
    {
        app.actions.push(Action::Open(Page::Album(album.clone())));
        app.actions.push(Action::ToggleWinampWindow);
    }
    if let Some(artist) = &row.artist_id
        && ui.button(gettext(locale, "Artist").as_ref()).clicked()
    {
        app.actions.push(Action::Open(Page::Artist(artist.clone())));
        app.actions.push(Action::ToggleWinampWindow);
    }
}

/// The options Spotify offers for the playlist the player is playing.
fn mix_menu(app: &mut App, ui: &mut egui::Ui, id: &str) {
    let locale = app.locale;
    let Some(state) = app.playing_signals.as_ref().filter(|state| state.id == id) else {
        ui.add_enabled(
            false,
            egui::Button::new(gettext(locale, "Loading options...").as_ref()),
        );
        app.actions.push(Action::LoadPlayingSignals);
        return;
    };
    if let Some(error) = &state.error {
        ui.label(error);
        if ui.button(gettext(locale, "Retry").as_ref()).clicked() {
            app.actions.push(Action::LoadPlayingSignals);
        }
        return;
    }
    let loading = state.loading;
    let controls = state.controls.clone();
    let selected = state.selected.clone();
    let applying = state.applying;
    if controls.is_empty() {
        let text = if loading {
            gettext(locale, "Loading options...")
        } else {
            gettext(locale, "No options for this playlist")
        };
        ui.add_enabled(false, egui::Button::new(text.as_ref()));
        return;
    }
    ui.add_enabled_ui(!applying, |ui| {
        for control in &controls {
            let active = selected.iter().any(|signal| signal == &control.signal);
            if ui
                .selectable_label(active, control.label.as_str())
                .clicked()
            {
                app.actions.push(Action::ApplyPlayingSignal {
                    signal: control.signal.clone(),
                });
                ui.close();
            }
        }
    });
}

/// Your playlists to play, and the queue kept as a new one.
fn list_menu(app: &mut App, ui: &mut egui::Ui, rows: &[Row], queue_uris: &[String]) {
    let locale = app.locale;
    if let Some(id) = app.playing_playlist_id() {
        ui.menu_button(gettext(locale, "Mix").as_ref(), |ui| {
            mix_menu(app, ui, &id);
        });
        ui.separator();
    }
    let playlists: Vec<(String, String)> = app
        .library
        .playlists
        .get()
        .map(|playlists| {
            playlists
                .iter()
                .map(|playlist| (playlist.name.clone(), playlist.uri.clone()))
                .collect()
        })
        .unwrap_or_default();
    ui.menu_button(gettext(locale, "Load list").as_ref(), |ui| {
        crate::autoscroll::show(
            ui,
            egui::ScrollArea::vertical().max_height(super::menu_limit(ui)),
            egui::Vec2b::new(false, true),
            |ui| {
                if playlists.is_empty() {
                    ui.add_enabled(
                        false,
                        egui::Button::new(gettext(locale, "No playlists yet").as_ref()),
                    );
                }
                for (name, uri) in &playlists {
                    if ui.button(name).clicked() {
                        app.actions.push(Action::PlayContext {
                            uri: uri.clone(),
                            offset_uri: None,
                            offset_index: None,
                        });
                        ui.close();
                    }
                }
            },
        );
    });
    let saveable = !queue_uris.is_empty() || rows.iter().any(|row| row.current);
    if ui
        .add_enabled(
            saveable,
            egui::Button::new(gettext(locale, "Save list").as_ref()),
        )
        .on_hover_text(gettext(locale, "Save the queue as a new playlist").as_ref())
        .clicked()
    {
        let mut uris: Vec<String> = rows
            .iter()
            .filter(|row| row.current)
            .map(|row| row.uri.clone())
            .collect();
        uris.extend(queue_uris.iter().cloned());
        app.actions.push(Action::CreatePlaylist {
            name: app.queue_playlist_name(),
            public: false,
            add_uris: uris,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_are_numbered_the_way_winamp_did() {
        assert_eq!(label(1, "Bonobo", "Rosewood"), "1. Bonobo - Rosewood");
        assert_eq!(label(12, "", "Episode 12"), "12. Episode 12");
    }

    #[test]
    fn a_notch_scrolls_three_rows_as_windows_did() {
        assert_eq!(rows_for_wheel(-1.0, 0.0, 0.0, 20.0, 8), 3.0);
        assert_eq!(rows_for_wheel(2.0, 0.0, 0.0, 20.0, 8), -6.0);
        assert_eq!(rows_for_wheel(0.0, -40.0, 0.0, 20.0, 8), 2.0);
        assert_eq!(rows_for_wheel(0.0, 0.0, -1.0, 20.0, 8), 8.0);
    }

    #[test]
    fn the_list_holds_whole_rows_only() {
        assert_eq!(rows_visible(116, 0), 4);
        assert_eq!(rows_visible(174, 0), 8);
        assert_eq!(list_area(174, 0), Area::new(12, 20, 243, 116));
        assert_eq!(list_area(174, 1), Area::new(12, 33, 243, 103));
        assert_eq!(rows_visible(116, 1), 3);
        assert_eq!(list_area(174, 2), Area::new(12, 46, 243, 90));
        assert_eq!(rows_visible(116, 2), 2);
    }

    fn demo_app(name: &str) -> App {
        let root =
            std::env::temp_dir().join(format!("spotifast-winamp-{name}-{}", std::process::id()));
        let mut app = App::new(
            &crate::backend::Waker::default(),
            crate::paths::AppDirs {
                config: root.join("config"),
                state: root.join("state"),
                cache: root.join("cache"),
            },
            crate::settings::Settings::default(),
            crate::app::AppOptions {
                media_controls: false,
                restore_sign_in: false,
                tray: false,
                autostart: false,
            },
        );
        crate::demo::populate(&mut app);
        app
    }

    fn open_demo_radio(app: &mut App) -> String {
        let seed = "spotify:track:4uLU6hMCjMI75M1A2tKUQC".to_string();
        let mut track = match app.now_playing_item().unwrap() {
            PlayableItem::Track(track) => track,
            _ => unreachable!(),
        };
        track.uri = seed.clone();
        track.id = Some("4uLU6hMCjMI75M1A2tKUQC".into());
        let mut second = track.clone();
        second.uri = "spotify:track:0VjIjW4GlUZAMYd2vXMi3b".into();
        second.id = Some("0VjIjW4GlUZAMYd2vXMi3b".into());
        second.name = "Radio's second song".into();
        // Real radio responses cache all their metadata before the view opens.
        app.track_cache
            .insert(second.id.clone().unwrap(), second.clone());
        app.radio_pages.insert(
            seed.clone(),
            crate::model::RadioPage {
                songs: crate::model::Loadable::Loaded(vec![track.clone(), second]),
                uri: Some("spotify:playlist:37i9dQZF1E4AaxlmEozmrV".into()),
                ..Default::default()
            },
        );
        app.settings.winamp_window = true;
        app.apply(
            Action::OpenWinampRadio {
                uri: seed.clone(),
                track: Box::new(track),
            },
            &egui::Context::default(),
        );
        seed
    }

    fn fill_search(app: &mut App) {
        let songs: Vec<_> = app
            .winamp_collection()
            .unwrap()
            .1
            .into_iter()
            .take(5)
            .filter_map(|(_, item)| match item {
                PlayableItem::Track(track) => Some(track),
                _ => None,
            })
            .collect();
        app.winamp.search_tab.query = "Bonobo".into();
        app.winamp.search_tab.committed = "Bonobo".into();
        app.winamp.search_tab.typed_at = None;
        app.winamp.search_tab.focus_requested = false;
        app.winamp.search_tab.songs = crate::model::Loadable::Loaded(songs);
    }

    #[test]
    fn search_preserves_all_three_views_and_closing_a_tab_leaves_playback_alone() {
        let mut app = demo_app("search-tab-state");
        app.prepare_winamp_playlist();
        app.winamp.playlist_scroll = 2;
        app.winamp.playlist_selection.insert(3);
        open_demo_radio(&mut app);
        app.winamp.playlist_scroll = 1;
        app.winamp.playlist_selection.insert(1);
        let original = app.winamp_collection().unwrap();
        let playing = app.now_playing().unwrap().uri;
        let ctx = egui::Context::default();
        app.apply(Action::OpenWinampSearch, &ctx);
        fill_search(&mut app);
        assert_eq!(app.winamp.playlist_scroll, 0);
        assert!(app.winamp.playlist_selection.is_empty());
        app.winamp.playlist_scroll = 1;
        app.winamp.playlist_selection.insert(2);
        app.apply(Action::ShowWinampRadio(true), &ctx);
        assert_eq!(app.winamp.playlist_scroll, 1);
        assert_eq!(
            app.winamp.playlist_selection,
            std::collections::HashSet::from([1])
        );
        app.apply(Action::OpenWinampSearch, &ctx);
        assert_eq!(app.winamp.playlist_scroll, 1);
        assert_eq!(
            app.winamp.playlist_selection,
            std::collections::HashSet::from([2])
        );
        assert_eq!(rows(&app, app.now_playing().as_ref()).0.len(), 5);
        app.apply(Action::CloseWinampSearch, &ctx);
        assert!(app.winamp.radio_tab.as_ref().unwrap().showing);
        assert_eq!(app.winamp.search_tab.query, "Bonobo");
        assert_eq!(app.winamp.search_tab.songs.get().unwrap().len(), 5);
        app.apply(Action::OpenWinampSearch, &ctx);
        app.apply(Action::CloseWinampRadio, &ctx);
        assert!(app.winamp.search_tab.showing);
        assert_eq!(
            app.winamp.playlist_selection,
            std::collections::HashSet::from([2])
        );
        app.apply(Action::CloseWinampSearch, &ctx);
        assert_eq!(app.winamp.playlist_scroll, 2);
        assert_eq!(
            app.winamp.playlist_selection,
            std::collections::HashSet::from([3])
        );
        assert_eq!(app.winamp_collection().unwrap(), original);
        assert_eq!(app.now_playing().unwrap().uri, playing);
        app.backend.shutdown();
    }

    fn search_frame(
        ctx: &egui::Context,
        app: &mut App,
        events: Vec<egui::Event>,
    ) -> egui::accesskit::TreeUpdate {
        let skin = app.winamp.skin.clone();
        let textures = app.winamp.textures(ctx);
        let now = app.now_playing();
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(550.0, 400.0),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                crate::ui::keys::handle(app, ctx);
                let mut view = View {
                    ui,
                    origin: egui::Pos2::ZERO,
                    unit: 2.0,
                    skin: &skin,
                    textures: &textures,
                    mask: None,
                };
                show(app, &mut view, now.as_ref(), true);
            },
        );
        output.textures_delta.clear();
        output.platform_output.accesskit_update.unwrap()
    }

    fn apply_pending(app: &mut App, ctx: &egui::Context) {
        for action in std::mem::take(&mut app.actions) {
            app.apply(action, ctx);
        }
    }

    fn pointer(pos: egui::Pos2, button: egui::PointerButton, pressed: bool) -> Vec<egui::Event> {
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button,
                pressed,
                modifiers: egui::Modifiers::NONE,
            },
        ]
    }

    #[test]
    fn ctrl_f_focuses_the_search_field_and_a_result_double_click_plays_without_replacing_the_playlist()
     {
        let mut app = demo_app("search-keyboard");
        app.settings.winamp_window = true;
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let page = app.page().clone();
        search_frame(
            &ctx,
            &mut app,
            vec![egui::Event::Key {
                key: egui::Key::F,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::COMMAND,
            }],
        );
        apply_pending(&mut app, &ctx);
        assert!(app.winamp.search_tab.showing && app.settings.playlist_open);
        search_frame(&ctx, &mut app, vec![]);
        apply_pending(&mut app, &ctx);
        assert!(ctx.memory(|memory| memory.has_focus(egui::Id::new("winamp-search-query"))));
        search_frame(&ctx, &mut app, vec![egui::Event::Text("Bonobo".into())]);
        apply_pending(&mut app, &ctx);
        assert_eq!(app.winamp.search_tab.query, "Bonobo");
        assert!(app.winamp.search_tab.typed_at.is_some());
        assert_eq!(app.page(), &page);
        fill_search(&mut app);
        let original = app.winamp_collection().unwrap();
        let (items, _) = rows(&app, app.now_playing().as_ref());
        let pos = egui::pos2(100.0, 130.0);
        search_frame(&ctx, &mut app, vec![]);
        for _ in 0..2 {
            search_frame(
                &ctx,
                &mut app,
                pointer(pos, egui::PointerButton::Primary, true),
            );
            search_frame(
                &ctx,
                &mut app,
                pointer(pos, egui::PointerButton::Primary, false),
            );
        }
        assert!(
            matches!(app.actions.as_slice(), [Action::PlayFromRow { context: RowContext::Uris(uris), uri, index: 0 }] if uris.as_ref() == [items[1].uri.clone()] && uri == &items[1].uri),
            "{:?}",
            app.actions
        );
        apply_pending(&mut app, &ctx);
        app.prepare_winamp_playlist();
        assert_eq!(app.now_playing().unwrap().uri, items[1].uri);
        assert_eq!(app.winamp_collection().unwrap(), original);
        app.apply(Action::CloseWinampSearch, &ctx);
        assert_eq!(app.now_playing().unwrap().uri, items[1].uri);
        app.backend.shutdown();
    }

    #[test]
    fn search_results_offer_queue_and_play_next_through_their_context_menu() {
        for next in [false, true] {
            let mut app = demo_app(if next {
                "search-play-next"
            } else {
                "search-queue"
            });
            app.settings.winamp_window = true;
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            app.apply(Action::OpenWinampSearch, &ctx);
            fill_search(&mut app);
            app.local_ready = true;
            app.local.track = Some(crate::player::LocalTrack {
                uri: app.now_playing().unwrap().uri,
                ..Default::default()
            });
            app.local.playback = crate::player::Playback::Paused;
            assert!(app.queue_locally_reorderable());
            search_frame(&ctx, &mut app, vec![]);
            let pos = egui::pos2(100.0, 104.0);
            search_frame(
                &ctx,
                &mut app,
                pointer(pos, egui::PointerButton::Secondary, true),
            );
            search_frame(
                &ctx,
                &mut app,
                pointer(pos, egui::PointerButton::Secondary, false),
            );
            let tree = search_frame(&ctx, &mut app, vec![]);
            let label = if next { "Play next" } else { "Add to queue" };
            let button = tree
                .nodes
                .iter()
                .find(|(_, node)| node.label() == Some(label))
                .expect("queue command is in the context menu")
                .0;
            search_frame(
                &ctx,
                &mut app,
                vec![egui::Event::AccessKitActionRequest(
                    egui::accesskit::ActionRequest {
                        target_tree: egui::accesskit::TreeId::ROOT,
                        target_node: button,
                        action: egui::accesskit::Action::Click,
                        data: None,
                    },
                )],
            );
            if next {
                assert!(
                    matches!(app.actions.as_slice(), [Action::PlayNext { items }] if items.len() == 1)
                );
            } else {
                assert!(matches!(
                    app.actions.as_slice(),
                    [Action::AddToQueue { .. }]
                ));
            }
            app.backend.shutdown();
        }
    }

    #[test]
    fn radio_tab_buttons_and_row_context_menu_emit_their_actions() {
        let mut app = demo_app("radio-controls");
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let draw = |app: &mut App, events| {
            let skin = app.winamp.skin.clone();
            let textures = app.winamp.textures(&ctx);
            let now = app.now_playing();
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(550.0, 400.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    let mut view = View {
                        ui,
                        origin: egui::Pos2::ZERO,
                        unit: 2.0,
                        skin: &skin,
                        textures: &textures,
                        mask: None,
                    };
                    show(app, &mut view, now.as_ref(), true);
                },
            );
            output.textures_delta.clear();
            output.platform_output.accesskit_update.unwrap()
        };
        draw(&mut app, vec![]);
        let pos = egui::pos2(100.0, 52.0);
        draw(
            &mut app,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Secondary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        draw(
            &mut app,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Secondary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        let tree = draw(&mut app, vec![]);
        let radio = tree
            .nodes
            .iter()
            .find(|(_, node)| node.label() == Some("Go to song radio"))
            .expect("right-click exposes song radio")
            .0;
        draw(
            &mut app,
            vec![egui::Event::AccessKitActionRequest(
                egui::accesskit::ActionRequest {
                    target_tree: egui::accesskit::TreeId::ROOT,
                    target_node: radio,
                    action: egui::accesskit::Action::Click,
                    data: None,
                },
            )],
        );
        assert!(matches!(
            app.actions.as_slice(),
            [Action::OpenWinampRadio { .. }]
        ));
        app.actions.clear();
        open_demo_radio(&mut app);
        draw(&mut app, vec![]);
        for (x, expected) in [(70.0, Some(false)), (250.0, Some(true)), (500.0, None)] {
            let pos = egui::pos2(x, 54.0);
            draw(
                &mut app,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
            draw(
                &mut app,
                vec![egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            match expected {
                Some(showing) => assert!(
                    matches!(app.actions.as_slice(), [Action::ShowWinampRadio(value)] if *value == showing),
                    "{:?}",
                    app.actions
                ),
                None => assert!(
                    matches!(app.actions.as_slice(), [Action::CloseWinampRadio]),
                    "{:?}",
                    app.actions
                ),
            }
            let action = app.actions.pop().unwrap();
            app.apply(action, &ctx);
            draw(&mut app, vec![]);
        }
        app.backend.shutdown();
    }

    #[test]
    fn radio_browsing_playback_and_closing_preserve_the_original_playlist() {
        let mut app = demo_app("radio-tabs");
        app.remote.as_mut().unwrap().state.is_playing = false;
        app.prepare_winamp_playlist();
        let original = app.winamp_collection().unwrap();
        let playing = app.now_playing().unwrap();
        let page = app.page().clone();
        app.winamp.playlist_scroll = 2;
        app.winamp.playlist_selection.insert(3);
        open_demo_radio(&mut app);
        assert_eq!(app.now_playing().unwrap().uri, playing.uri);
        assert_eq!(app.now_playing().unwrap().position_ms, playing.position_ms);
        assert_eq!(app.page(), &page);
        assert_eq!(app.winamp.playlist_scroll, 0);
        assert!(app.winamp.playlist_selection.is_empty());
        let (radio, _) = rows(&app, app.now_playing().as_ref());
        assert_eq!(radio.len(), 2);
        let (context, index) = radio[1].play.clone().unwrap();
        assert!(
            matches!(&context, RowContext::View { uris, context_uri, .. } if uris.len() == 2 && context_uri == "spotify:playlist:37i9dQZF1E4AaxlmEozmrV")
        );
        app.apply(
            Action::PlayFromRow {
                context,
                uri: radio[1].uri.clone(),
                index,
            },
            &egui::Context::default(),
        );
        app.prepare_winamp_playlist();
        assert_eq!(app.now_playing().unwrap().uri, radio[1].uri);
        assert_eq!(app.winamp_collection().unwrap(), original);
        app.winamp.playlist_scroll = 1;
        app.winamp.playlist_selection.insert(1);
        app.apply(Action::ShowWinampRadio(false), &egui::Context::default());
        assert_eq!(app.winamp.playlist_scroll, 2);
        assert_eq!(
            app.winamp.playlist_selection,
            std::collections::HashSet::from([3])
        );
        assert_eq!(
            rows(&app, app.now_playing().as_ref()).0.len(),
            original.1.len()
        );
        app.apply(Action::ShowWinampRadio(true), &egui::Context::default());
        assert_eq!(app.winamp.playlist_scroll, 1);
        assert_eq!(
            app.winamp.playlist_selection,
            std::collections::HashSet::from([1])
        );
        let playing = app.now_playing().unwrap();
        app.apply(Action::CloseWinampRadio, &egui::Context::default());
        app.prepare_winamp_playlist();
        assert!(app.winamp.radio_tab.is_none());
        assert_eq!(app.now_playing().unwrap().uri, playing.uri);
        assert_eq!(app.winamp_collection().unwrap(), original);
        assert_eq!(app.winamp.playlist_scroll, 2);
        app.backend.shutdown();
    }

    #[test]
    fn a_failed_or_empty_radio_leaves_playback_and_the_original_list_alone() {
        let mut app = demo_app("radio-retry");
        let seed = open_demo_radio(&mut app);
        let playing = app.now_playing().unwrap().uri;
        let original = app.winamp_collection().unwrap();
        app.radio_pages.get_mut(&seed).unwrap().songs =
            crate::model::Loadable::Failed("Radio unavailable".into());
        assert!(rows(&app, app.now_playing().as_ref()).0.is_empty());
        app.apply(
            Action::Reload(Page::Radio(seed.clone())),
            &egui::Context::default(),
        );
        assert!(app.radio_pages[&seed].songs.is_loading());
        assert_eq!(app.now_playing().unwrap().uri, playing);
        app.radio_pages.get_mut(&seed).unwrap().songs = crate::model::Loadable::Loaded(Vec::new());
        assert!(rows(&app, app.now_playing().as_ref()).0.is_empty());
        app.apply(Action::ShowWinampRadio(false), &egui::Context::default());
        assert_eq!(app.winamp_collection().unwrap(), original);
        app.backend.shutdown();
    }

    #[test]
    fn advancing_and_returning_keep_the_complete_playlist_and_its_positions() {
        let mut app = demo_app("stable-playlist");
        let now = app.now_playing();
        let (before, _) = rows(&app, now.as_ref());
        assert!(before.len() > 2);
        let uris: Vec<_> = before.iter().map(|row| row.uri.clone()).collect();
        app.winamp.playlist_selection.insert(1);
        app.winamp.playlist_scroll = 2;
        app.prepare_winamp_playlist();
        // Set the view after the initial source is established.
        app.winamp.playlist_selection.insert(1);
        app.winamp.playlist_scroll = 2;
        let item = app.winamp_collection().unwrap().1[2].1.clone();
        app.remote.as_mut().unwrap().state.item = Some(item.clone());
        app.queue.get_mut().unwrap().currently_playing = Some(item);
        app.queue.get_mut().unwrap().queue.clear();
        app.prepare_winamp_playlist();
        let (after, _) = rows(&app, app.now_playing().as_ref());
        assert_eq!(
            after.iter().map(|row| row.uri.clone()).collect::<Vec<_>>(),
            uris
        );
        assert!(after[2].current);
        assert!(!after[0].current);
        assert_eq!(app.winamp.playlist_scroll, 2);
        assert!(app.winamp.playlist_selection.contains(&1));
        assert!(matches!(
            &after[0].play,
            Some((RowContext::Context { .. }, 0))
        ));
        let first = app.winamp_collection().unwrap().1[0].1.clone();
        app.remote.as_mut().unwrap().state.item = Some(first);
        let (back, _) = rows(&app, app.now_playing().as_ref());
        assert!(back[0].current);
        assert_eq!(back.len(), before.len());
        app.backend.shutdown();
    }

    #[test]
    fn liked_songs_and_repeated_playlist_rows_keep_their_real_positions() {
        let mut app = demo_app("collection-positions");
        app.remote
            .as_mut()
            .unwrap()
            .state
            .context
            .as_mut()
            .unwrap()
            .uri = "spotify:user:demo:collection".into();
        let (liked, _) = rows(&app, app.now_playing().as_ref());
        assert_eq!(liked.len(), app.library.liked.items.len());
        assert!(
            matches!(&liked[0].play, Some((RowContext::Context { uri, .. }, 0)) if uri.ends_with(":collection"))
        );
        app.remote
            .as_mut()
            .unwrap()
            .state
            .context
            .as_mut()
            .unwrap()
            .uri = "spotify:playlist:pl1".into();
        let page = app.playlist_pages.get_mut("pl1").unwrap();
        let first = page.items.items[0].clone();
        page.items.items.insert(1, first);
        let (before, _) = rows(&app, app.now_playing().as_ref());
        let (context, index) = before[1].play.clone().unwrap();
        app.apply(
            Action::PlayFromRow {
                context,
                uri: before[1].uri.clone(),
                index,
            },
            &egui::Context::default(),
        );
        let (after, _) = rows(&app, app.now_playing().as_ref());
        assert!(!after[0].current);
        assert!(after[1].current);
        assert_eq!(after[0].uri, after[1].uri);
        app.backend.shutdown();
    }

    #[test]
    fn a_loose_queue_keeps_played_rows_and_duplicate_occurrences() {
        let mut app = demo_app("loose-playlist");
        app.remote.as_mut().unwrap().state.context = None;
        let first = app.now_playing_item().unwrap();
        let second = app.queue.get().unwrap().queue[0].clone();
        app.queue.get_mut().unwrap().queue = vec![second.clone(), first.clone()];
        app.prepare_winamp_playlist();
        assert_eq!(app.winamp.playlist_history.len(), 3);
        app.remote.as_mut().unwrap().state.item = Some(second.clone());
        app.queue.get_mut().unwrap().currently_playing = Some(second);
        app.queue.get_mut().unwrap().queue = vec![first.clone()];
        app.prepare_winamp_playlist();
        assert_eq!(app.winamp.playlist_cursor, Some(1));
        app.remote.as_mut().unwrap().state.item = Some(first.clone());
        app.queue.get_mut().unwrap().currently_playing = Some(first);
        app.queue.get_mut().unwrap().queue.clear();
        app.prepare_winamp_playlist();
        assert_eq!(app.winamp.playlist_cursor, Some(2));
        let (rows, _) = rows(&app, app.now_playing().as_ref());
        assert_eq!(rows.len(), 3);
        assert!(!rows[0].current);
        assert!(rows[2].current);
        app.settings.winamp_window = true;
        let (context, index) = rows[0].play.clone().unwrap();
        app.apply(
            Action::PlayFromRow {
                context,
                uri: rows[0].uri.clone(),
                index,
            },
            &egui::Context::default(),
        );
        app.prepare_winamp_playlist();
        assert_eq!(app.winamp.playlist_cursor, Some(0));
        assert_eq!(app.winamp.playlist_history.len(), 3);
        app.backend.shutdown();
    }

    #[test]
    fn the_row_heart_toggles_optimistically_without_selecting_or_playing() {
        let mut app = demo_app("row-heart");
        let ctx = egui::Context::default();
        let (rows, _) = rows(&app, app.now_playing().as_ref());
        let uri = rows[0].uri.clone();
        let saved = app.is_saved(&uri).unwrap();
        app.winamp.playlist_selection.insert(2);
        let mut pos = egui::Pos2::ZERO;
        let mut draw = |app: &mut App, events| {
            let skin = app.winamp.skin.clone();
            let textures = app.winamp.textures(&ctx);
            let mut output = ctx.run_ui(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ui| {
                    let mut view = View {
                        ui,
                        origin: egui::Pos2::ZERO,
                        unit: 2.0,
                        skin: &skin,
                        textures: &textures,
                        mask: None,
                    };
                    list(app, &mut view, &rows, 174);
                    let duration = util::format_duration_ms(rows[0].duration_ms);
                    let width = app.winamp.playlist_text.width(&duration).ceil();
                    pos = egui::pos2((12.0 + 243.0 - 3.0 - width - 5.0) * 2.0, 52.0);
                },
            );
            output.textures_delta.clear();
            pos
        };
        draw(&mut app, vec![]);
        let pos = draw(&mut app, vec![]);
        draw(
            &mut app,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        draw(
            &mut app,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(
            matches!(app.actions.as_slice(), [Action::ToggleSaved(held)] if held == &uri),
            "{:?}",
            app.actions
        );
        assert_eq!(
            app.winamp.playlist_selection,
            std::collections::HashSet::from([2])
        );
        let action = app.actions.pop().unwrap();
        app.apply(action, &ctx);
        assert_eq!(app.is_saved(&uri), Some(!saved));
        app.backend.shutdown();
    }

    /// REM's Remove selected names queue rows, not list rows: the list
    /// opens with the playing song, so a selected line sits one below its
    /// place in the queue. Only the songs the user queued go (#721).
    #[test]
    fn remove_selected_takes_the_selected_queued_songs() {
        use crate::app::AppOptions;
        use crate::paths::AppDirs;
        use crate::settings::Settings;
        use egui::accesskit::{Action as Access, ActionRequest, Role, TreeId};

        let root =
            std::env::temp_dir().join(format!("spotifast-winamp-remove-{}", std::process::id()));
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let waker = crate::backend::Waker::default();
        let mut app = App::new(
            &waker,
            AppDirs {
                config: root.join("config"),
                state: root.join("state"),
                cache: root.join("cache"),
            },
            Settings::default(),
            AppOptions {
                media_controls: false,
                restore_sign_in: false,
                tray: false,
                autostart: false,
            },
        );
        crate::demo::populate(&mut app);
        let now = app.now_playing();
        app.local_ready = true;
        app.local.track = Some(crate::player::LocalTrack {
            uri: now.as_ref().unwrap().uri.clone(),
            ..Default::default()
        });
        app.local.playback = crate::player::Playback::Paused;
        let queued: Vec<String> = app.queue.get().unwrap().queue[..2]
            .iter()
            .map(|item| item.uri().to_string())
            .collect();
        app.manual_queue = queued.clone();
        app.remote.as_mut().unwrap().state.context = None;
        let (rows, _) = rows(&app, now.as_ref());
        assert!(rows[0].current);
        // The second queued song, and the first song of Next up after it.
        app.winamp.playlist_selection.extend([2, 3]);

        let frame = |app: &mut App, events| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ui| rem_menu(app, ui, &rows),
            );
            output.textures_delta.clear();
            output.platform_output.accesskit_update.unwrap()
        };
        let tree = frame(&mut app, vec![]);
        let remove = tree
            .nodes
            .iter()
            .find(|(_, node)| {
                node.label() == Some("Remove selected") && node.role() == Role::Button
            })
            .unwrap()
            .0;
        frame(
            &mut app,
            vec![egui::Event::AccessKitActionRequest(ActionRequest {
                target_tree: TreeId::ROOT,
                target_node: remove,
                action: Access::Click,
                data: None,
            })],
        );
        assert!(
            matches!(
                app.actions.as_slice(),
                [Action::RemoveFromQueue { rows }] if *rows == [(1, queued[1].clone())]
            ),
            "{:?}",
            app.actions
        );
        assert!(app.winamp.playlist_selection.is_empty());
        app.backend.shutdown();
    }
}
