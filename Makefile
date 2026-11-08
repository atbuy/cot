PREFIX ?= $(HOME)/.local
BINDIR ?= $(PREFIX)/bin
ZENSICAL ?= zensical

.PHONY: all build install uninstall clean docs-build docs-serve

all: build

build:
	cargo build --release --locked

install: build
	install -Dm755 target/release/cot $(DESTDIR)$(BINDIR)/cot

uninstall:
	rm -f $(DESTDIR)$(BINDIR)/cot

clean:
	cargo clean
	rm -rf site

docs-build:
	$(ZENSICAL) build

docs-serve:
	$(ZENSICAL) serve
