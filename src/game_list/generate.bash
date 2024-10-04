#!/bin/bash

set -euxo pipefail

function generate {
cat <<'EOF'
// DO NOT EDIT; this file is auto-generated.
// Run `./generate.bash titles.jsonl mod.rs` to re-generate.


pub fn find_title_by_id(title_id: u32) -> Option<String> {
    GAMES_BY_TITLE_ID
        .binary_search_by_key(&title_id, |x| x.0)
        .ok()
        .map(|i| GAMES_BY_TITLE_ID[i].1.to_owned())
}

#[rustfmt::skip]
const GAMES_BY_TITLE_ID: &[(u32, &'static str)] = &[
EOF

jq -r '"    (0x" + .TitleID + ", " + (.Name | tojson) + "),"' "$1"

cat <<'EOF'
];
EOF
}

generate "$1" > "$2"