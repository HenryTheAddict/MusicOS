FROM ubuntu:22.04
ENV DEBIAN_FRONTEND=noninteractive CARGO_HOME=/opt/cargo RUSTUP_HOME=/opt/rustup PATH=/opt/cargo/bin:/opt/node/bin:$PATH
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl xz-utils unzip build-essential pkg-config git cmake clang libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf libasound2-dev libudev-dev libssl-dev libxkbcommon-dev libwayland-dev libfontconfig1-dev libx11-xcb-dev libxcb1-dev xvfb xauth mesa-vulkan-drivers libfuse2 file python3 dbus-x11 qemu-user-static squashfs-tools && rm -rf /var/lib/apt/lists/*
ARG RUST_VERSION=1.97.1
RUN curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs -o /tmp/rustup.sh && sh /tmp/rustup.sh -y --profile minimal --default-toolchain ${RUST_VERSION} && rm /tmp/rustup.sh
ARG NODE_VERSION=24.10.0
RUN curl -fsSL https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-linux-x64.tar.xz -o /tmp/node.tar.xz && curl -fsSL https://nodejs.org/dist/v${NODE_VERSION}/SHASUMS256.txt -o /tmp/node-checksums && cd /tmp && grep " node-v${NODE_VERSION}-linux-x64.tar.xz$" node-checksums | sed 's@node-v[^ ]*-linux-x64.tar.xz@node.tar.xz@' | sha256sum -c - && mkdir /opt/node && tar -xJf node.tar.xz -C /opt/node --strip-components=1 && rm /tmp/node.tar.xz /tmp/node-checksums
WORKDIR /workspace
ENV APPIMAGE_EXTRACT_AND_RUN=1 CARGO_BUILD_JOBS=4
