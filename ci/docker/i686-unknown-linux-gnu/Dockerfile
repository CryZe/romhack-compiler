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
    libgtk-3-dev:amd64 \
    libgtk-3-dev:i386

COPY xargo.sh /
RUN bash /xargo.sh
