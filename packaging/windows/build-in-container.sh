#!/bin/bash
# EXPERIMENTAL / unfinished: the Docker route stalls at getting fxc.exe into the image (GPUI needs
# it to compile shaders). GitHub Actions (release.yml) is the supported way to build Windows.
# Build the Windows installer on a Mac (or Linux) with Docker/OrbStack:
#
#   ./packaging/windows/build-in-container.sh
#
# Output: dist/Spelly-Setup.exe (plus spelly.exe and spelly-ui.exe).
# The image and three cache volumes are kept between runs, so only the first build is slow.
set -euo pipefail
cd "$(dirname "$0")/../.."

IMG=spelly-windows-builder
if [ "${REBUILD_IMAGE:-}" = "1" ] || ! docker image inspect "$IMG" >/dev/null 2>&1; then
  docker build --platform linux/amd64 -t "$IMG" packaging/windows/docker
fi

docker run --rm --platform linux/amd64 \
  -v "$PWD":/work \
  -v spelly-cargo:/cargo -v spelly-target:/target -v spelly-xwin:/xwin \
  -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target -e XWIN_CACHE_DIR=/xwin \
  "$IMG" /work/packaging/windows/container-build.sh
