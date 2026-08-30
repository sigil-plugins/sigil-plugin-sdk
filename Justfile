set shell := ["bash", "-euo", "pipefail", "-c"]

check:
    ./scripts/check-contracts.sh

reproducible:
    ./scripts/check-sql-reproducible.sh

sigil-check sigil_binary:
    ./scripts/check-sigil-compatibility.sh {{sigil_binary}}

drift sigil_checkout:
    ./scripts/check-wit-drift.sh {{sigil_checkout}}
