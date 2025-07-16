green := "\\033[32m"
yellow := "\\033[33m"
reset := "\\033[0m"

# List all recipes
default:
	@just --list

# updates Cargo packages to latest available
update:
    cargo upgrade && cargo update

# updates node package.json to latest available
outdated:
    @printf '{{ yellow }}=={{ reset }}NPM{{ yellow }}=={{ reset }}\n'
    npm outdated || true # `npm outdated` returns exit code 1 on finding outdated stuff ?!
    @printf '{{ yellow }}={{ reset }}Cargo{{ yellow }}={{ reset }}\n'
    cargo outdated -d 1
    @printf '{{ yellow }}======={{ reset }}\n'

clippy *args:
   cargo clippy --no-deps {{args}} -- \
     -W clippy::uninlined_format_args \
     -W clippy::unnecessary_mut_passed \
     -W clippy::unused_async

lint: clippy

fix:
   @just clippy --fix

fmt:
  just --fmt --unstable
  cargo fmt

