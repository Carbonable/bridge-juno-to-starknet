FROM rust:1.66-slim-bullseye as builder
RUN apt update && apt install pkg-config libssl-dev -y

WORKDIR /srv/www

COPY . .
RUN --mount=type=cache,target=/srv/www/target \
		--mount=type=cache,target=/usr/local/cargo/registry \
		--mount=type=cache,target=/usr/local/cargo/git \
		--mount=type=cache,target=/usr/local/rustup \
		set -eux; \
		rustup install stable; \
	 	cargo build --release; \
		objcopy --compress-debug-sections target/release/api ./api; \
		objcopy --compress-debug-sections target/release/worker ./worker



FROM debian:bullseye-slim as production-runtime
RUN apt update && apt install libssl-dev -y
RUN set -eux; \
		export DEBIAN_FRONTEND=noninteractive; \
	  apt update; \
		apt install --yes --no-install-recommends bind9-dnsutils iputils-ping iproute2 curl ca-certificates htop; \
		apt clean autoclean; \
		apt autoremove --yes; \
		rm -rf /var/lib/{apt,dpkg,cache,log}/; \
		echo "Installed base utils!"

WORKDIR /srv/www

COPY --from=builder /srv/www/api ./api
COPY --from=builder /srv/www/worker ./worker

CMD ["tail", "-f", "/dev/null"]
