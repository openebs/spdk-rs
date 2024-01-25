{}:
self: super: rec {
  fio = super.callPackage ./pkgs/fio { };
  sourcer = super.callPackage ./lib/sourcer.nix { };
  libnvme = super.callPackage ./pkgs/libnvme { };
  nvme-cli = super.callPackage ./pkgs/nvme-cli { };
  libspdk = (super.callPackage ./pkgs/libspdk { with-fio = false; }).release;
  libspdk-fio = (super.callPackage ./pkgs/libspdk { with-fio = true; multi-outputs = true; }).release;
  libspdk-dev = (super.callPackage ./pkgs/libspdk { with-fio = true; }).debug;
}
