{ lib, rustPlatform }:

rustPlatform.buildRustPackage {
  pname = "legion-cli";
  version = "1.4.0";

  src = ../legion-cli;

  cargoLock = {
    lockFile = ../legion-cli/Cargo.lock;
  };

  postInstall = ''
    mkdir -p $out/lib/udev/rules.d $out/bin $out/share/applications $out/share/icons/hicolor/scalable/apps
    cp udev/99-lenovo-legion.rules $out/lib/udev/rules.d/99-lenovo-legion.rules
    if [ -f ../bin/legionspace ]; then
      cp ../bin/legionspace $out/bin/legionspace
      cp ../bin/legionspace $out/bin/legion-gui
      chmod +x $out/bin/legionspace $out/bin/legion-gui
    fi
    if [ -f ../desktop/legion-space.desktop ]; then
      cp ../desktop/legion-space.desktop $out/share/applications/
    fi
    if [ -f ../desktop/legion-space.svg ]; then
      cp ../desktop/legion-space.svg $out/share/icons/hicolor/scalable/apps/
    fi
  '';

  meta = with lib; {
    description = "Native Linux hardware control utility for Lenovo Legion laptops (Pro 5 16ADR10)";
    homepage = "https://github.com/AayamPokhrel/LegionSpace4Linux";
    license = licenses.mit;
    maintainers = [ ];
    platforms = platforms.linux;
    mainProgram = "legion-cli";
  };
}

