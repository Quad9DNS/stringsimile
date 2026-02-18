.POSIX:
.SUFFIXES:
.SUFFIXES: .1 .5 .1.scd .5.scd

VPATH=doc
PREFIX?=/usr/local
BINDIR?=$(PREFIX)/bin
MANDIR?=$(PREFIX)/share/man
CONFDIR?=/etc/stringsimile
RULEDIR?=/var/lib/stringsimile

VERSION?=$(shell cat bin/stringsimile-service/Cargo.toml | grep version | cut -f 3 -d " " | cut -f2 -d '"' || echo unknown)

# Override the container tool. Tries docker first and then tries podman.
export CONTAINER_TOOL ?= auto
ifeq ($(CONTAINER_TOOL),auto)
	ifeq ($(shell docker version >/dev/null 2>&1 && echo docker), docker)
		override CONTAINER_TOOL = docker
	else ifeq ($(shell podman version >/dev/null 2>&1 && echo podman), podman)
		override CONTAINER_TOOL = podman
	else
		override CONTAINER_TOOL = unknown
	endif
endif

DOCS := $(addprefix target/man/,\
	stringsimile.1 \
	stringsimile-config.5 \
	stringsimile-rule-config.5)

all: $(DOCS) target/default/release/stringsimile
	cp target/default/release/stringsimile target/stringsimile

basic: $(DOCS) target/basic/release/stringsimile
	cp target/basic/release/stringsimile target/stringsimile

container-debian-static: deb
	$(CONTAINER_TOOL) build --build-arg CARGO_TARGET_DIR="target/default" -f distribution/container/Containerfile.debian-static .

container-debian-dynamic: deb-dynamic
	$(CONTAINER_TOOL) build --build-arg CARGO_TARGET_DIR="target/default" -f distribution/container/Containerfile.debian .

container-alpine:
	CARGO_TARGET_DIR="target/default" CARGO_BUILD_TARGET="x86_64-unknown-linux-musl" cargo build --release
	$(CONTAINER_TOOL) build --build-arg CARGO_TARGET_DIR="target/default" --build-arg CARGO_BUILD_TARGET="x86_64-unknown-linux-musl" -f distribution/container/Containerfile.alpine .

target/%/release/stringsimile:
	CARGO_TARGET_DIR="target/$*" cargo build --release --no-default-features --features $*

all-deb: deb deb-dynamic deb-basic
deb: target/default/debian/stringsimile_$(VERSION)-1_amd64.deb
deb-dynamic: target/all-dynamic/debian/stringsimile_$(VERSION)-1_amd64.deb
deb-basic: target/basic/debian/stringsimile_$(VERSION)-1_amd64.deb

all-rpm: rpm rpm-dynamic rpm-basic
rpm: target/default/generate-rpm/stringsimile_$(VERSION)-1.x86_64.rpm
rpm-dynamic: target/all-dynamic/generate-rpm/stringsimile_$(VERSION)-1.x86_64.rpm
rpm-basic: target/basic/generate-rpm/stringsimile_$(VERSION)-1.x86_64.rpm

target/%/debian/stringsimile_$(VERSION)-1_amd64.deb: target/%/release/stringsimile $(DOCS)
	CARGO_TARGET_DIR="target/$*" cargo deb --variant $*

target/%/generate-rpm/stringsimile_$(VERSION)-1.x86_64.rpm: target/%/release/stringsimile $(DOCS)
	cargo generate-rpm -p bin/stringsimile-service --target-dir "target/$*" --variant $*

.PHONY: dev
dev:
	cargo build

.PHONY: fmt
fmt:
	cargo fmt

.PHONY: fmt-check
fmt-check:
	cargo fmt --check

.PHONY: lint
lint:
	cargo clippy

.PHONY: check
check:
	cargo check
	cargo check --no-default-features
	cargo check --no-default-features --features basic

.PHONY: check-all
check-all: check lint test check-deny fmt-check

.PHONY: check-deny
check-deny:
	cargo deny check

.PHONY: test
test:
	cargo test

target/man/%.1: doc/%.1.scd
	@mkdir -p target/man
	scdoc < $? > $@

target/man/%.5: doc/%.5.scd
	@mkdir -p target/man
	scdoc < $? > $@

doc: $(DOCS)

# Exists in GNUMake but not in NetBSD make and others.
RM?=rm -f

clean:
	cargo clean

install: $(DOCS) target/default/release/stringsimile
	mkdir -m755 -p $(DESTDIR)$(BINDIR) $(DESTDIR)$(MANDIR)/man1 $(DESTDIR)$(MANDIR)/man5 $(CONFDIR) $(RULEDIR)
	install -m755 target/stringsimile $(DESTDIR)$(BINDIR)/stringsimile
	install -m644 target/man/stringsimile.1 $(DESTDIR)$(MANDIR)/man1/stringsimile.1
	install -m644 target/man/stringsimile-config.5 $(DESTDIR)$(MANDIR)/man5/stringsimile-config.5
	install -m644 target/man/stringsimile-rule-config.5 $(DESTDIR)$(MANDIR)/man5/stringsimile-rule-config.5
	install -m644 target/man/stringsimile-rule-config.5 $(DESTDIR)$(MANDIR)/man5/stringsimile-rule-config.5
	install -m644 distribution/config.yaml $(CONFDIR)/stringsimile.yaml
	install -m644 distribution/rules/* $(RULEDIR)/

RMDIR_IF_EMPTY:=sh -c '! [ -d $$0 ] || ls -1qA $$0 | grep -q . || rmdir $$0'

uninstall:
	$(RM) $(DESTDIR)$(BINDIR)/stringsimile
	$(RM) $(DESTDIR)$(MANDIR)/man1/stringsimile.1
	$(RM) $(DESTDIR)$(MANDIR)/man5/stringsimile-config.5
	$(RM) $(DESTDIR)$(MANDIR)/man5/stringsimile-rule-config.5
	${RMDIR_IF_EMPTY} $(DESTDIR)$(BINDIR)
	$(RMDIR_IF_EMPTY) $(DESTDIR)$(MANDIR)/man1
	$(RMDIR_IF_EMPTY) $(DESTDIR)$(MANDIR)/man5
	$(RMDIR_IF_EMPTY) $(DESTDIR)$(MANDIR)

.PHONY: all all-deb deb deb-dynamic deb-basic all-rpm rpm rpm-dynamic rpm-basic container-debian-static container-debian-dynamic container-alpine doc clean install uninstall debug
