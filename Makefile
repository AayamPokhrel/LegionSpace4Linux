PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/bin
UDEVDIR ?= /etc/udev/rules.d

CLI_TARGET_DIR = legion-cli/target/release
GUI_TARGET_DIR = legion-gui/target/release
CLI_BINARIES = $(CLI_TARGET_DIR)/legion-cli $(CLI_TARGET_DIR)/legionctl
GUI_BINARY = $(GUI_TARGET_DIR)/legionspace
UDEV_RULE = udev/99-lenovo-legion.rules

CARGO_RUN_CLI = cd legion-cli && if command -v cargo >/dev/null 2>&1; then cargo $(1); else nix-shell -p cargo rustc --run "cargo $(1)"; fi
CARGO_RUN_GUI = cd legion-gui && if command -v cargo >/dev/null 2>&1; then cargo $(1); else nix-shell -p cargo rustc pkg-config wayland libxkbcommon libGL --run "cargo $(1)"; fi

.PHONY: all build build-cli build-gui debug check test clean install install-user uninstall

all: build

build: build-cli build-gui

build-cli:
	@echo "==> Building legion-cli in release mode..."
	@$(call CARGO_RUN_CLI,build --release)
	@echo "==> CLI binaries built successfully in $(CLI_TARGET_DIR)"

build-gui:
	@echo "==> Building legion-gui (Material You native desktop app) in release mode..."
	@$(call CARGO_RUN_GUI,build --release)
	@echo "==> GUI binary built successfully in $(GUI_TARGET_DIR)"

debug:
	@echo "==> Building legion-cli & legion-gui in debug mode..."
	@$(call CARGO_RUN_CLI,build)
	@$(call CARGO_RUN_GUI,build)

check:
	@echo "==> Running cargo check..."
	@$(call CARGO_RUN_CLI,check)
	@$(call CARGO_RUN_GUI,check)

test:
	@echo "==> Running test suite..."
	@$(call CARGO_RUN_CLI,test)

clean:
	@echo "==> Cleaning build artifacts..."
	@$(call CARGO_RUN_CLI,clean)
	@$(call CARGO_RUN_GUI,clean)

install:
	@./install.sh

install-user: build
	@echo "==> Installing binaries & GUI launcher to $(HOME)/.local/bin..."
	install -d $(HOME)/.local/bin $(HOME)/.local/share/applications $(HOME)/.local/share/icons/hicolor/scalable/apps
	install -m 755 $(CLI_TARGET_DIR)/legion-cli $(HOME)/.local/bin/legion-cli
	install -m 755 $(CLI_TARGET_DIR)/legionctl $(HOME)/.local/bin/legionctl
	install -m 755 $(GUI_TARGET_DIR)/legionspace $(HOME)/.local/bin/legionspace
	install -m 755 $(GUI_TARGET_DIR)/legionspace $(HOME)/.local/bin/legion-gui
	install -m 644 desktop/legion-space.desktop $(HOME)/.local/share/applications/legion-space.desktop
	install -m 644 desktop/legion-space.svg $(HOME)/.local/share/icons/hicolor/scalable/apps/legion-space.svg
	@echo "==> Binaries & Legion Space GUI installed to $(HOME)/.local/bin"
	@echo ""
	@echo "To install udev rules for unprivileged hardware control, run:"
	@echo "  sudo cp $(UDEV_RULE) /etc/udev/rules.d/"
	@echo "  sudo udevadm control --reload-rules && sudo udevadm trigger"

uninstall:
	@echo "==> Removing binaries..."
	rm -f $(DESTDIR)$(BINDIR)/legion-cli $(DESTDIR)$(BINDIR)/legionctl $(DESTDIR)$(BINDIR)/legionspace $(DESTDIR)$(BINDIR)/legion-gui
	rm -f $(HOME)/.local/bin/legion-cli $(HOME)/.local/bin/legionctl $(HOME)/.local/bin/legionspace $(HOME)/.local/bin/legion-gui
	@if [ -f $(DESTDIR)$(UDEVDIR)/99-lenovo-legion.rules ]; then \
		echo "==> Removing udev rules..."; \
		rm -f $(DESTDIR)$(UDEVDIR)/99-lenovo-legion.rules; \
		udevadm control --reload-rules || true; \
	fi
	@echo "==> Uninstallation complete."
