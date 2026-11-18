{ lib
, rustPlatform
, pkg-config
, makeWrapper
, wayland
, libxkbcommon
, libGL
, vulkan-loader
, libx11
, libxcursor
, libxrandr
, libxi
}:

rustPlatform.buildRustPackage {
  pname = "legionspace";
  version = "1.4.0";

  src = lib.cleanSource ../.;

  cargoLock = {
    lockFile = ../Cargo.lock;
  };

  nativeBuildInputs = [
    pkg-config
    makeWrapper
  ];

  buildInputs = [
    wayland
    libxkbcommon
    libGL
    vulkan-loader
    libx11
    libxcursor
    libxrandr
    libxi
  ];

  postInstall = ''
    # Install udev rules for unprivileged hardware control
    install -Dm644 -t $out/lib/udev/rules.d/ udev/99-lenovo-legion.rules

    # Install Freedesktop .desktop entry
    install -Dm644 desktop/legion-space.desktop $out/share/applications/legion-space.desktop

    # Install scalable SVG icon
    install -Dm644 desktop/legion-space.svg $out/share/icons/hicolor/scalable/apps/legion-space.svg

    # Create convenient symlink for legion-gui
    ln -sf $out/bin/legionspace $out/bin/legion-gui

    # Wrap GUI with runtime Wayland, OpenGL, and X11 graphics libraries
    wrapProgram $out/bin/legionspace \
      --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath [
        wayland
        libxkbcommon
        libGL
        vulkan-loader
        libx11
        libxcursor
        libxrandr
        libxi
      ]}
  '';

  meta = with lib; {
    description = "Native Linux hardware driver, CLI utility, and Material You GUI for Lenovo Legion Pro 5";
    homepage = "https://github.com/AayamPokhrel/LegionSpace4Linux";
    license = licenses.mit;
    platforms = platforms.linux;
    mainProgram = "legionspace";
  };
}
