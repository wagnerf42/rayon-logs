with import <nixpkgs> {}; {
  env = stdenv.mkDerivation {
    name = "dispatch";
    buildInputs = [ python37Packages.python-language-server python37Packages.more-itertools ];
  };
}
