//! Spotify's session controls above a playlist's songs: the options its own
//! app shows, such as "Give me more indie pop" on Discover Weekly. Applying
//! one reshapes the playlist on Spotify's side, so the page reads its rows
//! again after a successful apply and, when that playlist is what plays, the
//! queue follows the new list too.
//!
//! The Winamp player's menu shows the options for the playlist it is playing,
//! which may not have a page open.

use super::*;

impl App {
    /// The playlist the player is playing, when its context is one.
    pub(crate) fn playing_playlist_id(&self) -> Option<String> {
        self.playing_context_uri()
            .as_deref()
            .and_then(|uri| uri.strip_prefix("spotify:playlist:"))
            .map(str::to_string)
    }

    /// Asks for the options of the playlist the player is playing, once, for
    /// the Winamp player's menu.
    pub(super) fn load_playing_signals(&mut self) {
        let Some(id) = self.playing_playlist_id() else {
            return;
        };
        if self
            .playing_signals
            .as_ref()
            .is_some_and(|state| state.id == id && state.requested)
        {
            return;
        }
        self.load_generation = self.load_generation.wrapping_add(1);
        let generation = self.load_generation;
        self.playing_signals = Some(PlayingSignals {
            id: id.clone(),
            generation,
            requested: true,
            ..PlayingSignals::default()
        });
        self.backend
            .send(Command::PlaylistSignals { id, generation });
    }

    /// Applies one of the playing playlist's options, or turns the applied
    /// one off, from the Winamp player's menu.
    pub(super) fn apply_playing_signal(&mut self, signal: &str) {
        let Some(id) = self.playing_playlist_id() else {
            return;
        };
        let Some(state) = self.playing_signals.as_mut().filter(|state| state.id == id) else {
            return;
        };
        if state.applying {
            return;
        }
        let turning_off = state.selected.iter().any(|applied| applied == signal);
        let Some(chosen) = (if turning_off {
            state.reset.clone()
        } else {
            state
                .controls
                .iter()
                .find(|control| control.signal == signal)
                .map(|control| control.signal.clone())
        }) else {
            return;
        };
        state.previous_selection = std::mem::take(&mut state.selected);
        state.selected = if turning_off {
            Vec::new()
        } else {
            vec![signal.to_string()]
        };
        state.applying = true;
        self.load_generation = self.load_generation.wrapping_add(1);
        state.generation = self.load_generation;
        let generation = state.generation;
        self.backend.send(Command::PlaylistSignal {
            id,
            generation,
            signal: chosen,
        });
    }

    /// Adopts the options Spotify offers for a playlist, or the ones it has
    /// after an apply. An answer to an older request is left out, so a slow
    /// reply cannot undo the option just chosen.
    pub(super) fn receive_playlist_signals(
        &mut self,
        id: &str,
        generation: u64,
        result: Result<crate::session_reads::PlaylistSignals, String>,
    ) {
        let Ok(answer) = &result else {
            // A playlist read without playback has no options to show.
            return;
        };
        if let Some(page) = self
            .playlist_pages
            .get_mut(id)
            .filter(|page| page.signals_generation == generation)
        {
            page.controls = answer.controls.clone();
            page.reset = answer.reset.clone();
            page.selected = answer.selected.clone();
        }
        if let Some(state) = self
            .playing_signals
            .as_mut()
            .filter(|state| state.id == id && state.generation == generation)
        {
            state.controls = answer.controls.clone();
            state.reset = answer.reset.clone();
            state.selected = answer.selected.clone();
        }
    }

    /// Adopts the answer to the newest option request. The signal reshaped the
    /// playlist, so its page and the playing queue are read again from it; a
    /// refusal puts the previous selection back.
    pub(super) fn receive_playlist_signal(
        &mut self,
        id: &str,
        generation: u64,
        result: Result<crate::session_reads::PlaylistSignals, String>,
    ) {
        let mut applied = false;
        let mut failure = None;
        if let Some(page) = self
            .playlist_pages
            .get_mut(id)
            .filter(|page| page.signals_generation == generation)
        {
            page.applying = false;
            match &result {
                Ok(answer) => {
                    page.controls = answer.controls.clone();
                    page.reset = answer.reset.clone();
                    page.selected = answer.selected.clone();
                    page.previous_selection.clear();
                    applied = true;
                }
                Err(error) => {
                    page.selected = std::mem::take(&mut page.previous_selection);
                    failure = Some(error.clone());
                }
            }
        }
        if let Some(state) = self
            .playing_signals
            .as_mut()
            .filter(|state| state.id == id && state.generation == generation)
        {
            state.applying = false;
            match &result {
                Ok(answer) => {
                    state.controls = answer.controls.clone();
                    state.reset = answer.reset.clone();
                    state.selected = answer.selected.clone();
                    state.previous_selection.clear();
                    applied = true;
                }
                Err(error) => {
                    state.selected = std::mem::take(&mut state.previous_selection);
                    failure = Some(error.clone());
                }
            }
        }
        if applied {
            if self.playing_playlist_id().as_deref() == Some(id)
                && let Ok(answer) = &result
            {
                self.replace_playing_queue(id, &answer.uris);
            }
            if self.playlist_pages.contains_key(id) {
                self.reload(Page::Playlist(id.to_string()));
            }
        }
        if let Some(error) = failure {
            self.toast(error);
        }
    }

    /// Plays the playlist's new list, keeping the song and place that was
    /// playing when it is still in it.
    fn replace_playing_queue(&mut self, id: &str, uris: &[String]) {
        if uris.is_empty() {
            return;
        }
        let Some(now) = self.now_playing() else {
            return;
        };
        let mut request = PlayRequest::context(format!("spotify:playlist:{id}"));
        if uris.iter().any(|uri| uri == &now.uri) {
            request.offset_uri = Some(now.uri.clone());
            request.position_ms = now.position_ms;
        }
        self.play_request(request, false);
    }

    /// Applies one of Spotify's options to the playlist, or turns the applied
    /// one off. The page shows the choice at once; the rows are replaced when
    /// Spotify's answer arrives, and a refusal puts the selection back.
    pub(super) fn apply_playlist_signal(&mut self, id: &str, signal: &str) {
        let Some(page) = self.playlist_pages.get_mut(id) else {
            return;
        };
        if page.applying || page.playlist.get().is_none() {
            return;
        }
        let turning_off = page.selected.iter().any(|applied| applied == signal);
        let Some(chosen) = (if turning_off {
            page.reset.clone()
        } else {
            page.controls
                .iter()
                .find(|control| control.signal == signal)
                .map(|control| control.signal.clone())
        }) else {
            return;
        };
        page.previous_selection = std::mem::take(&mut page.selected);
        page.selected = if turning_off {
            Vec::new()
        } else {
            vec![signal.to_string()]
        };
        page.applying = true;
        self.load_generation = self.load_generation.wrapping_add(1);
        page.signals_generation = self.load_generation;
        let generation = page.signals_generation;
        self.backend.send(Command::PlaylistSignal {
            id: id.to_string(),
            generation,
            signal: chosen,
        });
    }
}
