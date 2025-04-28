.POSIX:
.SUFFIXES:
.SUFFIXES: .1 .5 .1.scd .5.scd

VPATH=doc
PREFIX?=/usr/local
BINDIR?=$(PREFIX)/bin
MANDIR?=$(PREFIX)/share/man
CONFDIR?=/etc/stringsimile
RULEDIR?=/var/lib/stringsimile

export VERSION ?= $(shell command -v cat bin/stringsimile-service/Cargo.toml | grep version | cut -f 3 -d " " | cut -f2 -d '"' || echo unknown)

DOCS := $(addprefix target/man/,\
	stringsimile.1 \
	stringsimile-config.5 \
	stringsimile-rule-config.5)

all: target/release/stringsimile $(DOCS) deb rpm

target/release/stringsimile:
	cargo build --release

deb: target/debian/stringsimile_$(VERSION)-1_amd64.deb

rpm: target/generate-rpm/stringsimile_$(VERSION)-1.x86_64.rpm

target/debian/stringsimile_$(VERSION)-1_amd64.deb: target/release/stringsimile $(DOCS)
	cargo deb

target/generate-rpm/stringsimile_$(VERSION)-1.x86_64.rpm: target/release/stringsimile $(DOCS)
	cargo generate-rpm -p bin/stringsimile-service

.PHONY: dev
dev:
	cargo build

.PHONY: fmt
fmt:
	cargo fmt

.PHONY: lint
lint:
	cargo clippy

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

install: $(DOCS) target/release/stringsimile
	mkdir -m755 -p $(DESTDIR)$(BINDIR) $(DESTDIR)$(MANDIR)/man1 $(DESTDIR)$(MANDIR)/man5 $(CONFDIR) ($RULEDIR)
	install -m755 target/release/stringsimile $(DESTDIR)$(BINDIR)/stringsimile
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

.PHONY: all deb rpm doc clean install uninstall debug
