FROM ubuntu:focal
LABEL maintainer='emulator@hotmail.it'

ENV DEBIAN_FRONTEND=noninteractive
ENV TZ=Europe/Rome
ENV TZDIR=/usr/share/zoneinfo

RUN apt-get update && apt-get install -y \
    tzdata ca-certificates

RUN ln -fs $TZDIR/$TZ /etc/localtime \
    && dpkg-reconfigure -f noninteractive tzdata
