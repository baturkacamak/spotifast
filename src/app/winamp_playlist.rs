//! The mini player's playlist is a collection, not the consumable queue.

use super::*;

impl App {
    fn winamp_browse_source(&self) -> Option<String> {
        self.winamp
            .playlist_pinned_source
            .clone()
            .unwrap_or_else(|| self.playing_context_uri())
    }

    pub(super) fn open_winamp_radio(&mut self, uri: String, track: &Track) {
        if !uri.starts_with("spotify:track:") {
            return;
        }
        self.remember_radio_track(&uri, track);
        if self.winamp.playlist_pinned_source.is_none() {
            self.prepare_winamp_playlist();
            self.winamp.playlist_pinned_source = Some(self.winamp.playlist_source.clone());
        }
        self.show_winamp_radio(false);
        self.winamp.radio_tab = Some(crate::winamp::RadioTab {
            seed: uri.clone(),
            showing: false,
            scroll: 0,
            selection: HashSet::new(),
        });
        self.show_winamp_radio(true);
        self.ensure_loaded(Page::Radio(uri));
    }

    pub(super) fn show_winamp_radio(&mut self, showing: bool) {
        self.hide_winamp_search();
        if showing
            && self.winamp.radio_tab.is_some()
            && self.winamp.playlist_pinned_source.is_none()
        {
            self.winamp.playlist_pinned_source = Some(self.winamp.playlist_source.clone());
        }
        if let Some(tab) = &mut self.winamp.radio_tab
            && tab.showing != showing
        {
            std::mem::swap(&mut self.winamp.playlist_scroll, &mut tab.scroll);
            std::mem::swap(&mut self.winamp.playlist_selection, &mut tab.selection);
            self.winamp.playlist_wheel = 0.0;
            tab.showing = showing;
        }
    }

    pub(super) fn close_winamp_radio(&mut self) {
        if self
            .winamp
            .radio_tab
            .as_ref()
            .is_some_and(|tab| tab.showing)
        {
            self.show_winamp_radio(false);
        }
        self.winamp.radio_tab = None;
        // Keep the original list pinned even if the radio is still playing.
    }

    /// Loaded rows in their original collection order, including played songs.
    pub(crate) fn winamp_collection(&self) -> Option<(RowContext, Vec<(u32, PlayableItem)>)> {
        let uri = self.winamp_browse_source()?;
        let context = RowContext::Context {
            uri: uri.clone(),
            editable_playlist: None,
        };
        let page = collection_page(&uri)?;
        let rows = match page {
            Page::Playlist(id) => {
                let list = &self.playlist_pages.get(&id)?.items;
                if !list.loaded_once {
                    return None;
                }
                list.items
                    .iter()
                    .enumerate()
                    .filter_map(|(index, row)| {
                        Some((list.base_offset + index as u32, row.playable()?.clone()))
                    })
                    .collect()
            }
            Page::Album(id) => {
                let list = &self.album_pages.get(&id)?.tracks;
                if !list.loaded_once {
                    return None;
                }
                list.items
                    .iter()
                    .enumerate()
                    .map(|(index, track)| {
                        (
                            list.base_offset + index as u32,
                            PlayableItem::Track(track.clone()),
                        )
                    })
                    .collect()
            }
            Page::LikedSongs => {
                let list = &self.library.liked;
                if !list.loaded_once {
                    return None;
                }
                list.items
                    .iter()
                    .enumerate()
                    .map(|(index, row)| {
                        (
                            list.base_offset + index as u32,
                            PlayableItem::Track(row.track.clone()),
                        )
                    })
                    .collect()
            }
            _ => return None,
        };
        Some((context, rows))
    }

    /// Load through existing asynchronous collection requests. Drawing only
    /// queues actions; these reads and cursor changes happen before the view.
    pub(crate) fn prepare_winamp_playlist(&mut self) {
        let source = self.winamp_browse_source();
        if self.winamp.playlist_source != source {
            self.winamp.playlist_source = source.clone();
            self.winamp.playlist_history.clear();
            self.winamp.playlist_cursor = None;
            self.winamp.playlist_selection.clear();
            self.winamp.playlist_scroll = 0;
        }
        if let Some(page) = source.as_deref().and_then(collection_page)
            && matches!(page, Page::Playlist(_) | Page::Album(_) | Page::LikedSongs)
            && self.is_connected()
            && !self.offline
        {
            self.ensure_loaded(page.clone());
            let list = match &page {
                Page::Playlist(id) => self.playlist_pages.get(id).map(|p| {
                    (
                        p.items.base_offset,
                        p.items.loaded_once,
                        p.items.can_load_more(),
                    )
                }),
                Page::Album(id) => self.album_pages.get(id).map(|p| {
                    (
                        p.tracks.base_offset,
                        p.tracks.loaded_once,
                        p.tracks.can_load_more(),
                    )
                }),
                Page::LikedSongs => Some((
                    self.library.liked.base_offset,
                    self.library.liked.loaded_once,
                    self.library.liked.can_load_more(),
                )),
                _ => None,
            };
            if let Some((base, loaded, more)) = list {
                if base > 0 {
                    self.load_window(page, 0);
                } else if loaded
                    && more
                    && self.winamp.original_playlist_scroll() + 32
                        >= self.winamp_collection().map_or(0, |(_, rows)| rows.len())
                {
                    self.load_more(page);
                }
            }
        }
        // A loose queue has no collection to reopen. Keep its already played
        // prefix and refresh only the future, preserving duplicate occurrences.
        let collection = self.winamp_collection();
        if collection.is_none() && self.winamp.playlist_pinned_source.is_none() {
            let now = self.now_playing_item();
            let queue = self.queue.get();
            let changing = self
                .intent_track
                .as_ref()
                .is_some_and(|intent| intent.at.elapsed() < PLAYBACK_HOLD);
            if let Some(now) = now {
                let history = &mut self.winamp.playlist_history;
                let old = self.winamp.playlist_cursor;
                let cursor = old
                    .filter(|&i| history.get(i).is_some_and(|item| item.uri() == now.uri()))
                    .or_else(|| {
                        history
                            .iter()
                            .enumerate()
                            .skip(old.map_or(0, |i| i + 1))
                            .find(|(_, item)| item.uri() == now.uri())
                            .map(|(i, _)| i)
                    });
                let cursor = cursor.unwrap_or_else(|| {
                    history.truncate(old.map_or(0, |i| i + 1));
                    history.push(now.clone());
                    history.len() - 1
                });
                history[cursor] = now.clone();
                if let Some(queue) = queue.filter(|q| {
                    !changing
                        && q.currently_playing
                            .as_ref()
                            .is_some_and(|item| item.uri() == now.uri())
                }) {
                    history.truncate(cursor + 1);
                    history.extend(queue.queue.iter().cloned());
                }
                self.winamp.playlist_cursor = Some(cursor);
            }
        }
        let visible = (self.settings.playlist_height / crate::skin::layout::PLAYLIST_TRACK_HEIGHT
            + 1) as usize;
        let search_uris = self.winamp.search_tab.showing.then(|| {
            self.winamp
                .search_tab
                .songs
                .get()
                .into_iter()
                .flatten()
                .skip(self.winamp.playlist_scroll)
                .take(visible)
                .map(|track| track.uri.clone())
                .collect()
        });
        let radio_uris = self
            .winamp
            .radio_tab
            .as_ref()
            .filter(|tab| tab.showing)
            .and_then(|tab| self.radio_pages.get(&tab.seed))
            .and_then(|page| page.songs.get())
            .map(|songs| {
                songs
                    .iter()
                    .skip(self.winamp.playlist_scroll)
                    .take(visible)
                    .map(|track| track.uri.clone())
                    .collect()
            });
        let uris = search_uris.or(radio_uris).unwrap_or_else(|| {
            collection.map_or_else(
                || {
                    self.winamp
                        .playlist_history
                        .iter()
                        .skip(self.winamp.playlist_scroll)
                        .take(visible)
                        .map(|item| item.uri().to_owned())
                        .collect()
                },
                |(_, rows)| {
                    rows.into_iter()
                        .skip(self.winamp.playlist_scroll)
                        .take(visible)
                        .map(|(_, item)| item.uri().to_owned())
                        .collect()
                },
            )
        });
        if self.is_connected() && !self.offline {
            self.request_contains(uris);
        }
    }
}

fn collection_page(uri: &str) -> Option<Page> {
    if uri.ends_with(":collection") {
        Some(Page::LikedSongs)
    } else {
        Page::from_uri(uri)
    }
}
