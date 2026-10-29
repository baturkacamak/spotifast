//! Debounced, songs-only search for the mini player's third tab.

use super::*;

impl App {
    pub(super) fn open_winamp_search(&mut self) {
        if self.winamp.playlist_pinned_source.is_none() {
            self.prepare_winamp_playlist();
            self.winamp.playlist_pinned_source = Some(self.winamp.playlist_source.clone());
        }
        if !self.winamp.search_tab.showing {
            let return_to_radio = self
                .winamp
                .radio_tab
                .as_ref()
                .is_some_and(|tab| tab.showing);
            self.show_winamp_radio(false);
            let search = &mut self.winamp.search_tab;
            std::mem::swap(&mut self.winamp.playlist_scroll, &mut search.scroll);
            std::mem::swap(&mut self.winamp.playlist_selection, &mut search.selection);
            search.showing = true;
            search.return_to_radio = return_to_radio;
            self.winamp.playlist_wheel = 0.0;
        }
        let search = &mut self.winamp.search_tab;
        search.open = true;
        search.focus_requested = true;
        if search.query.trim() != search.committed && !search.query.trim().is_empty() {
            search.typed_at.get_or_insert_with(Instant::now);
        }
        if !self.settings.playlist_open || self.settings.playlist_shaded {
            self.settings.playlist_open = true;
            self.settings.playlist_shaded = false;
            self.settings_dirty = true;
        }
    }

    /// Park the search view and restore the original playlist. Radio is
    /// selected separately, so its own scroll and selection stay independent.
    pub(super) fn hide_winamp_search(&mut self) {
        let search = &mut self.winamp.search_tab;
        if search.showing {
            std::mem::swap(&mut self.winamp.playlist_scroll, &mut search.scroll);
            std::mem::swap(&mut self.winamp.playlist_selection, &mut search.selection);
            search.showing = false;
            search.focus_requested = false;
            self.winamp.playlist_wheel = 0.0;
        }
    }

    pub(super) fn close_winamp_search(&mut self) {
        let return_to_radio =
            self.winamp.search_tab.showing && self.winamp.search_tab.return_to_radio;
        self.hide_winamp_search();
        let search = &mut self.winamp.search_tab;
        search.open = false;
        search.typed_at = None;
        search.focus_requested = false;
        search.serial = search.serial.wrapping_add(1);
        if search.songs.is_loading() {
            search.songs = Loadable::NotLoaded;
            search.committed.clear();
        }
        self.backend.api(ApiRequest::WinampSearch {
            query: String::new(),
            serial: search.serial,
        });
        if return_to_radio {
            self.show_winamp_radio(true);
        }
    }

    pub(super) fn edit_winamp_search(&mut self, query: String) {
        let search = &mut self.winamp.search_tab;
        if search.query == query {
            return;
        }
        search.query = query;
        search.serial = search.serial.wrapping_add(1);
        search.committed.clear();
        let empty = search.query.trim().is_empty();
        search.songs = if empty {
            Loadable::NotLoaded
        } else {
            Loadable::Loading
        };
        search.typed_at = (!empty).then(Instant::now);
        if search.showing {
            self.winamp.playlist_scroll = 0;
            self.winamp.playlist_selection.clear();
        } else {
            search.scroll = 0;
            search.selection.clear();
        }
        // Cancel the previous query immediately, including quota waits. Only
        // the debounced submission below sends a nonempty request to Spotify.
        self.backend.api(ApiRequest::WinampSearch {
            query: String::new(),
            serial: search.serial,
        });
    }

    pub(super) fn submit_winamp_search(&mut self) {
        let search = &mut self.winamp.search_tab;
        search.typed_at = None;
        let query = search.query.trim().to_owned();
        if query == search.committed && !search.songs.needs_load() {
            return;
        }
        search.serial = search.serial.wrapping_add(1);
        search.committed = query.clone();
        search.songs = if query.is_empty() {
            Loadable::NotLoaded
        } else {
            Loadable::Loading
        };
        self.backend.api(ApiRequest::WinampSearch {
            query,
            serial: search.serial,
        });
    }

    pub(super) fn tick_winamp_search(&mut self, ctx: &egui::Context) {
        if let Some(typed) = self.winamp.search_tab.typed_at {
            if typed.elapsed() >= SEARCH_DEBOUNCE {
                self.submit_winamp_search();
            } else {
                ctx.request_repaint_after(SEARCH_DEBOUNCE - typed.elapsed());
            }
        }
    }

    pub(super) fn receive_winamp_search(
        &mut self,
        query: String,
        serial: u64,
        result: Result<Vec<Track>, crate::api::ApiError>,
    ) {
        let search = &self.winamp.search_tab;
        if serial != search.serial || query != search.committed || query != search.query.trim() {
            return;
        }
        if let Ok(tracks) = &result {
            for track in tracks {
                if let Some(id) = &track.id {
                    self.track_cache.insert(id.clone(), track.clone());
                    self.track_used.insert(id.clone(), Instant::now());
                }
            }
            self.request_contains(tracks.iter().map(|track| track.uri.clone()).collect());
        }
        self.winamp.search_tab.songs = Loadable::from_result(result);
    }
}
