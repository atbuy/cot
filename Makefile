PREFIX ?= $(HOME)/.local
BINDIR ?= $(PREFIX)/bin

.PHONY: all build install uninstall clean

all: build

build:
	cargo build --release --locked

install: build
	install -Dm755 target/release/cot $(DESTDIR)$(BINDIR)/cot

uninstall:
	rm -f $(DESTDIR)$(BINDIR)/cot

clean:
	cargo clean
