---
title: The Winamp Mini Player
description: Use classic Winamp 2 skins with an analyser, equalizer, and playlist.
nav_order: 4
---

Open the mini player with Ctrl+M (Cmd+Shift+M on macOS), the shrink button, or
**Switch to it** in Settings. It supports classic Winamp 2 `.wsz` skins. Find
skins at the [Winamp Skin Museum](https://skins.webamp.org).

Only one player window is open at a time. Click the skin logo or Eject, or use
the shortcut again, to return to the main window.

![The mini player wearing the built-in skin](/assets/images/winamp.png)

## Skins and window size

Drop a `.wsz` file on either window to install and use it. Settings lists the
installed skins and can open the skins folder.

Choose **Random**, first in that list, to get a different skin each time you
switch to the mini player, never the same one twice in a row. Settings says
which skin it picked, and choosing a skin yourself turns Random off.

You can also use an unpacked skin folder. Spotifast finds skin files inside
its subfolders, up to eight folders deep, so you do not need to move them all
into one folder.

Right-click the title bar, or click **O**, to choose a size from 1x to 4x.
Each size keeps the classic pixels sharp. The same menu can keep the player
above other windows. **D** steps through the sizes, 1x to 4x, and **A**
toggles always-on-top.
Spotifast remembers the window position where your desktop allows it.

Skins can have transparent areas and shapes other than rectangles. Modern
Winamp 3 and 5 skin formats are not supported; choose classic Winamp 2 skins.

## Main controls

Most controls match Winamp. As in Winamp, Play on a song that is already
playing starts it again from the beginning; Pause pauses. These work
differently:

- The heart adds or removes the current song from Liked Songs, including in the rolled-up player.
- **Stop** pauses and rewinds.
- **I** opens the playing album in the main window.
- Repeat is either on or off.
- The X button and both logos return to the main window.

Click the time to switch between elapsed and remaining time. The balance and
MONO/STEREO controls affect playback on this computer. Quit from the
right-click menu or with Ctrl+Q.

The shade button, or a double-click on the title bar, rolls the player up. Its
tiny spectrum analyser or oscilloscope keeps moving in the collapsed bar;
click the display to cycle through bars, waveform and off, just like the full
player. The playlist and equalizer have their own shade buttons.

Switching to the main window and back keeps the shade modes and restores the
mini player's own position. It does not inherit the main window's fullscreen
or maximized state. On Linux desktops using Wayland, the desktop chooses where
windows appear and may ignore the saved position.

The left display shows bars that react to the sound (a spectrum analyser).
Click it to switch to a moving sound wave (an oscilloscope), then off.
You can also use the **V** menu. Changes to the equalizer affect the picture;
turning down the volume does not, even at zero. The display reacts only to
music playing on this computer.

## Jam

Start a Jam from the **O** menu, the one with Size and Always on top. Its Jam
submenu lists who is in the Jam and offers **Copy Jam link** and **End the
Jam**; starting one copies its link at the same time, so it is ready to paste.

While a Jam runs, the strip beside the bitrate shows a small **JAM** badge with
the listener count. Clicking it opens the Jam window under the player: who is
listening, the link, and the buttons to copy it or end the Jam. It is built
from the playlist's own title bar and borders, so it matches the equalizer and
the playlist in any skin, and its shade button rolls it up to a one-line bar
that names who is listening, and says so for a moment when someone joins or
leaves. Its X closes the window only, not the Jam; the
music and the session carry on, and the badge stays lit. The listener count in
the submenu opens the same window.

Someone joining or leaving is announced for a moment in the player's scrolling
line, the one that usually shows the song.

Guests join from their own Spotify apps, so nobody needs anything extra on
this computer. This computer plays the music, which means playback has to be
on here and the account has to be Premium.

## Playlist

**PL** opens the playlist below the player. It keeps the playing playlist,
album, or Liked Songs in its original order, including songs already played.
The playing song is highlighted without removing earlier rows or changing
your scroll position. More rows load as you scroll. Double-click any song,
including an earlier one, to play from there. Ctrl-click selects several rows;
drag the lower-right corner to resize the window. Use X or **PL** to close it.

The heart beside each Spotify song shows whether it is in Liked Songs. Click
it to save or unsave that song without changing playback or the selected rows.
Without a collection, the list keeps the songs played since it was opened
along with the remaining queue. The main window still offers the live queue.

- **ADD → Search Spotify** opens the Search tab inside the mini player.
  **ADD → Liked Songs** opens Liked Songs in the main window.
- **SEL** selects rows.
- **MISC** opens song, artist, and album pages, or the selected song’s radio.
- **LIST OPTS** starts one of your playlists or saves the queue as a new one.
  **LIST OPTS → Mix** shows Spotify's options for the playlist that is
  playing, such as "Give me more indie pop" on Discover Weekly. Choosing one
  reshapes the list on Spotify's side; the song and place already playing are
  kept, and the queue follows the new list. Choosing it again turns it off.
  The menu shows when options are loading. A failed read shows its error and
  a Retry button; "No options for this playlist" means Spotify answered with
  no options for that playlist.
- **REM → Remove selected** takes the selected songs you queued out of the
  queue when it is showing a loose queue, and **REM → Remove all** clears every
  song you queued. Both work when
  this computer is playing. Songs from the playing album or playlist stay.

Right-click a Spotify song and choose **Go to song radio**, or use **MISC**,
to open its radio in a second tab inside the playlist panel. Opening the tab
does not change playback. Double-click a radio song to start that mix from the
chosen row. The **Playlist** tab keeps the original list and its scroll
position and selection; each tab remembers its own. Double-click an original
playlist row to return to playing that list. The radio tab's X closes its view
without stopping playback or replacing the original playlist. Starting another
playlist from **LIST OPTS** or the main window follows that new playlist.

Radio uses the existing local playback session and has the same availability
as the main window's radio page. Loading, an empty result, or an error stays in
the radio tab. Click the error to retry; the original playlist remains usable.

**Ctrl+F** (**Cmd+F** on macOS) also opens the **Search** tab and focuses its
field, opening or unrolling the playlist panel if needed. Search by song or
artist name. After a short pause in typing, matching songs appear with their
hearts and durations. Enter submits immediately. Double-click a result to play
that song on its own, as in the main window. The other results are not queued;
unavailable songs cannot start playback.
Right-click a result for **Add to queue** or **Play next**. Play next works
while this computer is the active player, including when paused.

The search field appears only in the Search tab. Switching tabs keeps the
query, results, scroll position, and selected rows independently from Playlist
and Radio. Closing Search keeps playback and the original playlist, and
reopening it restores the last completed search. A failed search offers a
click to retry; clearing the field clears its results. The main window's
search is independent. Search queries and results in the mini player stay in
memory for this run and are cleared on sign-out.

Notices from the main window scroll through the mini player's text display.

## Equalizer

**EQ** opens the ten-band equalizer. It affects playback on this computer, not
other Spotify Connect devices. The preamp ranges from -12 to 12 dB. **AUTO**
resets all bands. The same controls and presets are in Settings.

To keep your own curve, such as a headphone correction, type a name under the
presets in **Settings → Equalizer** and choose **Save preset**. It stores the
preamp and the ten bands, and appears after the built-in presets in Settings
and in the mini player's **PRESETS** menu. Saving under a name you already
used replaces that preset. While a saved preset is in use, **Delete** beside
**Save preset** removes it; the sound stays as it is.
