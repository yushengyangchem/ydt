repo_root := `git rev-parse --show-toplevel`

default:
    just --list --unsorted

# Update flake.lock
update:
    cd {{ repo_root }} && nix flake update
