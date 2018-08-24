FROM ubuntu:14.04

RUN dpkg --add-architecture i386 && \
    apt-get update && \
    apt-get install -y \
    ca-certificates \
    cmake3 \
    g++-multilib \
    libc6-dev \
    make \
    pkg-config \
    git-all \
    gir1.2-gtk-3.0 \
    libglib2.0-dev \
    libgdk-pixbuf2.0-dev \
    libpango1.0-dev \
    libatk1.0-dev \
    libatk-bridge2.0-dev \
    libcairo2-dev \
    libxkbcommon-dev \
    libxi-dev \
    libxcomposite-dev \
    libgtk-3-dev:amd64 \
    libwayland-dev:i386 \
    libgtk-3-dev:i386

COPY xargo.sh /
RUN bash /xargo.sh
