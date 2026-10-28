# Devam notu: Spotify radyo oturum kontrolleri

## Amaç

Kullanıcı, Spotify'ın mobil uygulamasında radyo sayfalarının üzerindeki
"Give me more soul", "More discovery tracks", "Go deeper into rock" gibi
seçenekleri Spotifast'ta da kullanmak istiyor. Seçenek seçildiğinde liste
filtrelenmiyor: Spotify o seçenek için yeni bir kişiselleştirilmiş liste
üretiyor. Seçenekler listeye ve kullanıcıya göre değişiyor.

## Doğrulanan protokol (27 Ekim 2026, canlı hesap)

Telefondaki "almost real. Radio" sayfası Apollo station'ı değil, Spotify'ın
radyo playlist'i:

1. `GET /inspiredby-mix/v2/seed_to_playlist/{seed}?response-format=json`
   - Track/album/artist seed'i `spotify:playlist:37i9dQZF1E...` döndürüyor.
   - Playlist seed'i `spotify:station:playlist:...` döndürüyor, yani o türde
     playlist radyosu yok; mevcut station yolu korunuyor.
2. `GET /playlist/v2/playlist/{id}?decorate=revision,length,attributes,owner`
   yanıtı:
   - Sıra `spotify:track:` URI'leri.
   - `contents.available_signals` içinde sinyal kimlikleri, örneğin
     `session_control_display$<liste>$more_discovery`,
     `session_control_display$<liste>$soul:nl_genre`, `session-control-reset`.
   - Çip yazıları `attributes.format_attributes` içinde
     `session_control_display.displayName.<sinyal adı>` anahtarlarıyla.
   - Aktif seçim `session_control.selected_signals` (virgülle ayrılmış).
3. Seçenek uygulama:
   `POST /playlist/v2/playlist/{id}/signals`, gövde
   `ListSignals{base_revision, emitted_signals:[Signal{identifier}]}`,
   yanıt `SelectedListContent` (yeni liste, yeni revision, seçim ve yazılar).
   - Uygulanan sinyal `available_signals` başında görünüyor.
   - Yeni bir tür sinyali öncekini değiştiriyor (birikmiyor).
   - `session-control-reset` seçimi temizliyor.
4. Gözlem: "Give me more soul" 50 şarkının 49'unu değiştirdi, ilk şarkıyı
   korudu. "Go deeper into rock" tüm listeyi değiştirdi. Reset geri aldı.

## Uygulama (yazıldı)

- `src/session_reads.rs`: `RadioMix`, `radio()`, `radio_signal()`,
  inspiredby-mix çözümü, çip etiketleri, seçim durumu. Şarkı seed'i yine
  listenin başında kalıyor (mevcut davranış korundu).
- `src/model.rs`: `RadioPage` alanları ve `RadioControl`;
  `Action::ApplyRadioSignal`.
- `src/backend.rs`: `Command::RadioSignal`, `Event::RadioSignal`,
  `start_radio_signal`; radyo yüklemesi yeni akışa bağlı.
- `src/app/radio.rs`: seçim anında gösteriliyor, eski/geç yanıt yeni seçimi
  ezmiyor, hata önceki seçimi geri getiriyor.
- `src/ui/radio.rs`: erişilebilir çip satırı, seçili çipe tekrar basınca
  kapanıyor, Play radyonun playlist'ini çalıyor.
- Playlist sayfaları (Discover Weekly, Release Radar, kayıtlı radyo
  playlist'leri) aynı çipleri gösteriyor: `src/app/session_controls.rs`,
  `PlaylistPage` alanları, `src/ui/widgets.rs` içindeki ortak çip satırı,
  `src/ui/collection.rs` içindeki playlist sayfası.
- Discover Weekly'nin gerçekten 6 çip + reset verdiği doğrulandı
  (`37i9dQZEVXcGbZ9SuFTBvX`).
- `src/demo.rs`: demo çipleri (radyo ve Discover Weekly).
- Winamp mini player: playlist penceresindeki **LIST OPTS → Mix** menüsü,
  çalan playlist'in seçeneklerini gösteriyor. Seçim uygulanınca çalan şarkı
  ve konumu korunarak kuyruk Spotify'ın yeni listesiyle değiştiriliyor
  (`src/app/session_controls.rs`, `src/ui/winamp/playlist.rs`).
- Dokümanlar: `docs/_reference/what-spotify-allows.md`,
  `docs/_reference/how-it-connects.md`.
- Testler: session_reads'e 4, app'e 4 yeni test; radyo testleri 22/22.

## Durum

- Yerel Flatpak, `spotifast-auto-update` ile aynı yöntemle derlenip kuruldu.
  Radyo çipleri kurulu sürümde. Playlist çipleri eklendikten sonra yeniden
  derlenip kurulacak.
- Yerel derleme için `deps` ağacına `flex` ve `m4` eklendi (libprojectM
  CMake'i istiyordu).
- `cargo fmt --all --check`, clippy (default ve all-features), lib testleri
  geçti. Tek istisna `mpris_search_links_open_search_on_a_private_bus`:
  SDK içinde `dbus-run-session` yok, host'ta geçiyor.
- Kalan: `--all-targets` ve `--all-features` testleri, doc testleri,
  `cargo doc`, demo görsel kontrolü, commit.
- Telefondaki radyo resetlendi (seçili seçenek yok).
