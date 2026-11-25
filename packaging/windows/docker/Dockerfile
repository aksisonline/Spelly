# Builds the Windows version of Spelly from macOS/Linux:
#   cross-compiles with cargo-xwin (MSVC target) and packages Spelly-Setup.exe with
#   Inno Setup running under Wine. Runs as linux/amd64 (OrbStack uses Rosetta on Apple silicon).
FROM --platform=linux/amd64 rust:1-bookworm

ENV DEBIAN_FRONTEND=noninteractive \
    WINEPREFIX=/opt/wine \
    WINEARCH=win32 \
    WINEDEBUG=-all \
    WINEDLLOVERRIDES="mscoree,mshtml="

# clang/lld/llvm: linker + llvm-rc (resource compiler) for the MSVC target.
# wine32 + xvfb: Inno Setup's compiler is a 32-bit Windows program.
RUN dpkg --add-architecture i386 && apt-get update && apt-get install -y --no-install-recommends \
      clang lld llvm cmake ninja-build pkg-config ca-certificates curl git \
      xvfb xauth wine wine32 \
    && rm -rf /var/lib/apt/lists/* \
    && (ls /usr/lib/llvm-*/bin/llvm-rc >/dev/null && ln -sf $(ls /usr/lib/llvm-*/bin/llvm-rc | head -1) /usr/local/bin/llvm-rc)

# Harper needs a recent stable Rust.
RUN rustup update stable && rustup default stable && rustup target add x86_64-pc-windows-msvc \
    && cargo install cargo-xwin --locked

# Inno Setup 6 into a 32-bit Wine prefix.
RUN curl -fsSL -o /tmp/is.exe https://github.com/jrsoftware/issrc/releases/download/is-6_7_4/innosetup-6.7.4.exe \
    && xvfb-run -a wineboot --init \
    && xvfb-run -a wine /tmp/is.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SP- \
    && wineserver -w; rm -f /tmp/is.exe; \
    ls "$WINEPREFIX/drive_c/Program Files/Inno Setup 6/ISCC.exe" 2>/dev/null || ls "$WINEPREFIX/drive_c/Program Files (x86)/Inno Setup 6/ISCC.exe"

WORKDIR /work

# fxc.exe (HLSL compiler) from Microsoft's official SDK build-tools package. GPUI compiles its
# DirectX shaders with it at build time; we run the 32-bit one under Wine (see fxc wrapper).
RUN apt-get update && apt-get install -y --no-install-recommends unzip python3 \
    && rm -rf /var/lib/apt/lists/* \
    && curl -fsSL -o /tmp/bt.nupkg https://api.nuget.org/v3-flatcontainer/microsoft.windows.sdk.buildtools/10.0.28000.1839/microsoft.windows.sdk.buildtools.10.0.28000.1839.nupkg \
    && mkdir -p /opt/fxc /tmp/bt && unzip -q /tmp/bt.nupkg 'bin/*/x86/*' -d /tmp/bt \
    && cp -r /tmp/bt/bin/*/x86/. /opt/fxc/ && rm -rf /tmp/bt /tmp/bt.nupkg \
    && printf '#!/bin/sh\nexec wine /opt/fxc/fxc.exe "$@"\n' > /usr/local/bin/fxc && chmod +x /usr/local/bin/fxc \
    && ls -la /opt/fxc/fxc.exe
ENV GPUI_FXC_PATH=/usr/local/bin/fxc
