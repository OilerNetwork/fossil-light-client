#!/bin/bash

cargo clippy \
    -p common \
    -p ethereum \
    -p guest-types \
    -p mmr-utils \
    -p relayer \
    -p starknet-handler \
    -- \
    -W clippy::single_match \
    -W clippy::single_match_else \
    -W clippy::needless_match \
    -W clippy::needless_late_init \
    -W clippy::redundant_pattern_matching \
    -W clippy::redundant_pattern \
    -W clippy::redundant_guards \
    -W clippy::collapsible_match \
    -W clippy::match_single_binding \
    -W clippy::match_same_arms \
    -W clippy::match_ref_pats \
    -W clippy::match_bool \
    -D clippy::needless_bool \
    -W clippy::unwrap_used \
    -W clippy::expect_used