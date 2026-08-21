set shell := ["bash", "-euo", "pipefail", "-c"]

check:
    ./scripts/check-contracts.sh

drift sigil_checkout:
    ./scripts/check-wit-drift.sh {{sigil_checkout}}
