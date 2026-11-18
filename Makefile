PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/bin
UDEVDIR ?= /etc/udev/rules.d

TARGET_DIR = legion-cli/target/release
BINARIES = $(TARGET_DIR)/legion-cli $(TARGET_DIR)/legionctl
UDEV_RULE = udev/99-lenovo-legion.rules

CARGO_RUN = cd legion-cli && if command -v cargo >/dev/null 2>&1; then cargo $(1); else nix-shell -p cargo rustc --run "cargo $(1)"; fi

.PHONY: all build debug check test clean install install-user uninstall

all: build

build:
	@echo "==> Building legion-cli in release mode..."
	@$(call CARGO_RUN,build --release)
	@echo "==> Binaries built successfully in $(TARGET_DIR)"

debug:
	@echo "==> Building legion-cli in debug mode..."
	@$(call CARGO_RUN,build)

check:
	@echo "==> Running cargo check..."
	@$(call CARGO_RUN,check)

test:
	@echo "==> Running test suite..."
	@$(call CARGO_RUN,test)

clean:
	@echo "==> Cleaning build artifacts..."
	@$(call CARGO_RUN,clean)

install:
	@./install.sh

install-user: build
	@echo "==> Installing binaries & GUI launcher to $(HOME)/.local/bin..."
	install -d $(HOME)/.local/bin $(HOME)/.local/share/applications $(HOME)/.local/share/icons/hicolor/scalable/apps
	install -m 755 $(TARGET_DIR)/legion-cli $(HOME)/.local/bin/legion-cli
	install -m 755 $(TARGET_DIR)/legionctl $(HOME)/.local/bin/legionctl
	install -m 755 bin/legionspace $(HOME)/.local/bin/legionspace
	install -m 755 bin/legionspace $(HOME)/.local/bin/legion-gui
	install -m 644 desktop/legion-space.desktop $(HOME)/.local/share/applications/legion-space.desktop
	install -m 644 desktop/legion-space.svg $(HOME)/.local/share/icons/hicolor/scalable/apps/legion-space.svg
	@echo "==> Binaries & Legion Space GUI installed to $(HOME)/.local/bin"
	@echo ""
	@echo "To install udev rules for unprivileged hardware control, run:"
	@echo "  sudo cp $(UDEV_RULE) /etc/udev/rules.d/"
	@echo "  sudo udevadm control --reload-rules && sudo udevadm trigger"

uninstall:
	@echo "==> Removing binaries..."
	rm -f $(DESTDIR)$(BINDIR)/legion-cli $(DESTDIR)$(BINDIR)/legionctl
	rm -f $(HOME)/.local/bin/legion-cli $(HOME)/.local/bin/legionctl
	@if [ -f $(DESTDIR)$(UDEVDIR)/99-lenovo-legion.rules ]; then \
		echo "==> Removing udev rules..."; \
		rm -f $(DESTDIR)$(UDEVDIR)/99-lenovo-legion.rules; \
		udevadm control --reload-rules || true; \
	fi
	@echo "==> Uninstallation complete."
