{ lib, rustPlatform }:

rustPlatform.buildRustPackage {
  pname = "legion-cli";
  version = "0.9.0";

  src = ../legion-cli;

  cargoLock = {
    lockFile = ../legion-cli/Cargo.lock;
  };

  postInstall = ''
    mkdir -p $out/lib/udev/rules.d
    cp udev/99-lenovo-legion.rules $out/lib/udev/rules.d/99-lenovo-legion.rules
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

