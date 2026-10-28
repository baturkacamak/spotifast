#!/usr/bin/env bash
# Build Spotifast and install the result over the user's Flatpak, so the app
# grid and `flatpak run` use this checkout. For local testing only: the build
# is unsigned and comes from a local repository, not from a release.
#
#   packaging/flatpak/install-local.sh [--debug] [--run] [--clean]
#
# The Flatpak must be installed once already. The script reuses its runtime
# and permissions and keeps its data (sign-in, settings), so an upgrade from a
# release to a local build does not sign the listener out.
#
#   --debug   install a debug build (faster to make, much larger)
#   --run     open the app when the install finishes
#   --clean   drop the local repository and its remote first
set -euo pipefail
cd "$(dirname "$0")/../.."

app=rocks.spotifast.Spotifast
remote=spotifast-local
cache="${XDG_CACHE_HOME:-$HOME/.cache}/spotifast-flatpak"
build="$cache/build"
repo="$cache/repo"
manifest="$(dirname "$0")/$app.bundle.yml"
profile=release
run=no

for arg in "$@"; do
    case "$arg" in
        --debug) profile=debug ;;
        --run) run=yes ;;
        --clean)
            rm -rf "$cache"
            flatpak remote-delete --user --force "$remote" 2>/dev/null || true
            ;;
        -h | --help)
            sed -n '2,14p' "$0" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *)
            echo "unknown option: $arg (try --help)" >&2
            exit 2
            ;;
    esac
done

if ! flatpak info --user "$app" >/dev/null 2>&1; then
    echo "Install $app once first: this script reuses its sandbox and data." >&2
    exit 1
fi

echo "==> building ($profile)"
if [ "$profile" = release ]; then
    cargo build --release
    bin=target/release/spotifast
else
    cargo build
    bin=target/debug/spotifast
fi

echo "==> staging"
rm -rf "$build"
mkdir -p "$build/files/bin" \
    "$build/files/share/applications" \
    "$build/files/share/icons/hicolor/scalable/apps" \
    "$build/files/share/metainfo" \
    "$build/files/share/licenses/spotifast"
install -Dm755 "$bin" "$build/files/bin/spotifast"
flatpak info --user --show-metadata "$app" >"$build/metadata"
install -Dm644 packaging/applications/spotifast.desktop \
    "$build/files/share/applications/$app.desktop"
sed -i "s/^Icon=.*$/Icon=$app/;s/^StartupWMClass=spotifast$/StartupWMClass=$app/" \
    "$build/files/share/applications/$app.desktop"
install -Dm644 packaging/icons/spotifast.svg \
    "$build/files/share/icons/hicolor/scalable/apps/$app.svg"
install -Dm644 "packaging/flatpak/$app.metainfo.xml" \
    "$build/files/share/metainfo/$app.metainfo.xml"
install -Dm644 LICENSE "$build/files/share/licenses/spotifast/LICENSE"

# The sandbox permissions come from the manifest, so they cannot drift from
# what Flathub would install.
finish=()
while IFS= read -r line; do
    case "$line" in
        "  - "*) finish+=("${line#  - }") ;;
    esac
done < <(sed -n '/^finish-args:/,/^[^ ]/p' "$manifest")
flatpak build-finish --command=spotifast "${finish[@]}" "$build" >/dev/null

echo "==> exporting"
subject="Local build $(git rev-parse --short HEAD)"
git diff --quiet || subject="$subject (modified)"
flatpak build-export --arch="$(uname -m)" --subject="$subject" \
    "$repo" "$build" master
flatpak build-update-repo --prune "$repo" >/dev/null
flatpak remote-add --user --if-not-exists --no-gpg-verify "$remote" "$repo"

echo "==> installing"
flatpak install --user -y --reinstall "$remote" "$app" >/dev/null
echo "installed $(flatpak info --user --show-commit "$app" | cut -c1-12) ($subject)"
if flatpak ps --columns=application 2>/dev/null | grep -qx "$app"; then
    echo "Spotifast is running: quit and open it again to use this build."
fi

if [ "$run" = yes ]; then
    exec flatpak run "$app"
fi
