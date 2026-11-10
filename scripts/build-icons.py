#!/usr/bin/env python3
"""Builds src/app/icons.js: the drawing that replaces each Stremio icon.

stremio-web draws icons as inline SVGs with no name on them, only their
paths, so each icon is recognized by the start of its first path. Omarchy
draws its icons with Nerd Font glyphs; these are the same Material Design
icons that Nerd Fonts carries (its md-* set), taken as SVG paths so they sit
exactly centered instead of following a font's line metrics. Filled and
outline pairs keep Stremio's selected/unselected distinction.

Run it after stremio-web moves to a new @stremio/stremio-icons release:

    scripts/build-icons.py
"""

import json
import re
import urllib.request
from pathlib import Path

STREMIO_ICONS = "5.15.0"  # @stremio/stremio-icons, as pinned by stremio-web
NERD_FONTS = "v3.5.1"
MDI = "7.4.47"  # @mdi/js, the Material Design Icons that Nerd Fonts' md-* set draws
ICONS_URL = f"https://cdn.jsdelivr.net/npm/@stremio/stremio-icons@{STREMIO_ICONS}/react/dist/index.js"
GLYPHS_URL = f"https://raw.githubusercontent.com/ryanoasis/nerd-fonts/{NERD_FONTS}/glyphnames.json"
MDI_URL = f"https://cdn.jsdelivr.net/npm/@mdi/js@{MDI}/mdi.js"
FINGERPRINT = 32  # characters of the first path; unique across the set
OUTPUT = Path(__file__).resolve().parent.parent / "src/app/icons.js"

# Stremio icon name -> Nerd Font glyph name (Material Design, md-*).
ICONS = {
 'about':'md-information_outline','account':'md-account_circle','actors':'md-account_group',
 'addons':'md-puzzle','addons-outline':'md-puzzle_outline','add':'md-plus','add-to-library':'md-bookmark_plus_outline',
 'android':'md-android','anime':'md-ninja','anime-outline':'md-ninja','arrow-down':'md-arrow_down',
 'aspect-ratio':'md-aspect_ratio','audio':'md-volume_high','audio-tracks':'md-music_box_multiple_outline',
 'bin':'md-delete_outline','calendar':'md-calendar_month','calendar-outline':'md-calendar_month_outline','calendar-thin':'md-calendar_blank_outline',
 'caret-down':'md-menu_down','caret-up':'md-menu_up','cast':'md-cast','cast-connected':'md-cast_connected',
 'channels':'md-television_classic','channels-outline':'md-television_classic','chapters':'md-format_list_numbered',
 'checkmark':'md-check','chevron-back':'md-chevron_left','chevron-down':'md-chevron_down','chevron-forward':'md-chevron_right','chevron-up':'md-chevron_up',
 'clock':'md-clock_outline','close':'md-close','cloud-library':'md-cloud_outline','cloud-sync':'md-cloud_sync_outline',
 'day':'md-white_balance_sunny','delete':'md-delete_outline','details':'md-information_outline','discover':'md-compass','discover-outline':'md-compass_outline',
 'download':'md-download','drag-handle':'md-drag_horizontal_variant','edit':'md-pencil_outline','episodes':'md-format_list_bulleted',
 'exit':'md-logout','external-player':'md-open_in_new','eye':'md-eye_outline','eye-off':'md-eye_off_outline',
 'facebook':'md-facebook','filters':'md-filter_variant','filters-outline':'md-filter_variant','glasses':'md-glasses','grid':'md-view_grid_outline',
 'hdr':'md-high_definition_box','heart':'md-heart','heart-outline':'md-heart_outline','help':'md-help_circle_outline',
 'home':'md-home','home-outline':'md-home_outline','hourglass':'md-timer_sand','in-cinema':'md-filmstrip',
 'keyboard':'md-keyboard_outline','language':'md-translate','library':'md-bookshelf','library-outline':'md-bookshelf',
 'link':'md-link_variant','linux':'md-linux','list':'md-format_list_bulleted','local-import':'md-folder_download_outline','lock':'md-lock_outline',
 'macos':'md-apple','magnet-link':'md-magnet','maximize':'md-fullscreen','megaphone':'md-bullhorn_outline','memory':'md-memory',
 'mic':'md-microphone_outline','minimize':'md-fullscreen_exit','more-horizontal':'md-dots_horizontal','more-vertical':'md-dots_vertical',
 'movies':'md-movie','movies-outline':'md-movie_outline','network':'md-lan','next':'md-skip_next','night':'md-weather_night',
 'notifications':'md-bell','notifications-outline':'md-bell_outline','open-in-browser':'md-open_in_new','pause':'md-pause',
 'person':'md-account','person-outline':'md-account_outline','person-outline-edit':'md-account_edit_outline',
 'play':'md-play','play-outline':'md-play_outline','remote':'md-remote','remove':'md-minus','remove-from-library':'md-bookmark_remove_outline',
 'replay':'md-replay','reset':'md-restore','rotate':'md-rotate_right','scale':'md-resize','search':'md-magnify','search-outline':'md-magnify',
 'series':'md-television','series-outline':'md-television','settings':'md-cog','settings-outline':'md-cog_outline','share':'md-share_variant_outline',
 'skip-15-back':'md-rewind_15','skip-15-forward':'md-fast_forward_15','skip-back':'md-skip_previous','skip-forward':'md-skip_next',
 'sort-up-down':'md-sort','speed':'md-speedometer','star':'md-star','star-outline':'md-star_outline','star-rating-full':'md-star','star-rating-half':'md-star_half_full',
 'subtitles':'md-subtitles_outline','thumbs-up':'md-thumb_up','thumbs-up-outline':'md-thumb_up_outline','trailer':'md-movie_open_play_outline',
 'tv':'md-television','tv-outline':'md-television','video-scale-crop':'md-crop','video-scale-fit':'md-fit_to_screen_outline','video-scale-stretch':'md-arrow_expand_all',
 'volume-high':'md-volume_high','volume-low':'md-volume_low','volume-medium':'md-volume_medium','volume-mute':'md-volume_mute','volume-off':'md-volume_off',
 'warning':'md-alert_outline','windows':'md-microsoft_windows','reddit':'md-reddit','youtube':'md-youtube','x':'md-close',
}

# Brand logos stay as Stremio draws them: a glyph would misrepresent them.
KEEP = {'x','imdb','imdb-outline','trakt','vlc','symbol','dot-arrow'}


def fetch(url):
    with urllib.request.urlopen(url, timeout=30) as response:
        return response.read().decode()


def main():
    glyphs = json.loads(fetch(GLYPHS_URL))
    source = fetch(ICONS_URL)
    entries = re.findall(r'"?([a-z0-9-]+)"?: \{ viewBox: "[^"]+", paths: \[(.*?)\] \}', source)

    # mdi.js exports each icon as `mdiHomeOutline="M10 20v-6..."`.
    paths = dict(re.findall(r'\b(mdi[A-Za-z0-9]+) = "([^"]+)"', fetch(MDI_URL)))

    def mdi_path(glyph):
        """md-home_outline -> the path of mdiHomeOutline, if both sets have it."""
        if glyph not in glyphs:
            return None
        words = glyph.removeprefix("md-").split("_")
        return paths.get("mdi" + "".join(word.capitalize() for word in words))

    table, unmapped = {}, []
    for name, icon_paths in entries:
        if name in KEEP:
            continue
        drawing = mdi_path(ICONS.get(name, ""))
        if drawing is None:
            unmapped.append(name)
            continue
        first_path = re.search(r'd: "([^"]+)"', icon_paths).group(1)
        key = first_path[:FINGERPRINT]
        # Stremio ships some icons twice under two names (tv and series); the
        # same drawing is fine as long as both names map to the same icon.
        if table.get(key, drawing) != drawing:
            raise SystemExit(f"fingerprint collision on {name}; raise FINGERPRINT")
        table[key] = drawing

    if unmapped:
        raise SystemExit(f"no icon for: {', '.join(sorted(unmapped))}")

    OUTPUT.write_text(
        "// Generated by scripts/build-icons.py from @stremio/stremio-icons "
        f"{STREMIO_ICONS} and Material Design Icons {MDI} (Apache-2.0). Do not edit.\n"
        f"const OMATRIMIO_ICON_FINGERPRINT = {FINGERPRINT};\n"
        f"const OMATRIMIO_ICONS = {json.dumps(table, sort_keys=True, indent=4)};\n"
    )
    print(f"{len(table)} icons -> {OUTPUT}")


if __name__ == "__main__":
    main()
