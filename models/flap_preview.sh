#!/usr/bin/env bash
mkdir -p preview_pngs
find previews -type f -name "*.svg" -exec bash -c 'rsvg-convert -h 512 "$0" > preview_pngs/`basename "$0"`.png' {} \;
ffmpeg -y -i preview_pngs/preview_%d.svg.png -filter_complex "tile=10x5:padding=30:margin=30:color=white" preview.png