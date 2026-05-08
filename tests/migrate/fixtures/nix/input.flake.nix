{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-23.11";

  outputs = { self, nixpkgs, ... }:
    let pkgs = import nixpkgs { system = "x86_64-linux"; };
    in {
      devShells.default = pkgs.mkShell {
        buildInputs = with pkgs; [
          nodejs_20
          python312
          docker-client
          terraform
          kubectl
        ];
        shellHook = ''
          export NODE_ENV=development
          echo "shell ready"
        '';
      };
    };
}
