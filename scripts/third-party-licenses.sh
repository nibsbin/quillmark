#!/bin/bash
set -e
set -o pipefail

# Writes THIRD_PARTY_LICENSES.md, which the npm package and the wheels ship
# beside LICENSE and NOTICE: the license text of every crate linked into a
# binary package (cargo-about, policy in scripts/about.toml), then the NOTICE
# files those crates carry, which Apache-2.0 §4(d) passes on. Needs
# cargo-about and jq. release-prepare.yml runs it, so each release commits the
# file its packages ship.

cd "$(dirname "$0")/.."

OUT=THIRD_PARTY_LICENSES.md

cargo about generate --workspace --all-features --locked --fail \
    -c scripts/about.toml -o "$OUT" scripts/third-party-licenses.hbs

printf '# Notices\n\n' >> "$OUT"
cargo metadata --format-version 1 --locked \
    | jq -r --rawfile listed "$OUT" '
        ($listed | [scan("(?m)^- (\\S+) (\\S+)") | join(" ")] | unique) as $crates
        | .packages[]
        | select(.source != null and ((.name + " " + .version) as $k | $crates | index($k)))
        | "\(.name) \(.version) \(.manifest_path | sub("/Cargo.toml$"; ""))"' \
    | sort \
    | while read -r name version dir; do
        for notice in "$dir"/NOTICE "$dir"/NOTICE.*; do
            [ -f "$notice" ] || continue
            printf '## %s %s\n\n~~~~text\n%s\n~~~~\n\n' "$name" "$version" "$(cat "$notice")" >> "$OUT"
        done
    done
