FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y --no-install-recommends \
    bc \
    bison \
    build-essential \
    ca-certificates \
    cpio \
    curl \
    file \
    flex \
    git \
    libncurses-dev \
    libssl-dev \
    python3 \
    rsync \
    unzip \
    wget \
    xz-utils \
  && rm -rf /var/lib/apt/lists/*
