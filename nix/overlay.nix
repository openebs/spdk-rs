{}:
final: prev: rec {
  fio         = prev.callPackage ./pkgs/fio { };
  libnvme     = prev.callPackage ./pkgs/libnvme { };
  nvme-cli    = prev.callPackage ./pkgs/nvme-cli { };
  libspdk     = prev.callPackage ./pkgs/libspdk { with-fio = false; build-type = "release"; };
  libspdk-fio = prev.callPackage ./pkgs/libspdk { with-fio = true;  build-type = "release"; multi-outputs = true; };
  libspdk-dev = prev.callPackage ./pkgs/libspdk { with-fio = true;  build-type = "debug";   };
}
