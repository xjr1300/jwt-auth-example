#!/usr/bin/env bash

cargo test --package tests -- --ignored \
    --skip can_access_protected_resource_at_within_expiration_of_refresh_token \
    --skip cannot_access_protected_resource_at_expired_expiration_of_refresh_token

cargo test --package tests -- --ignored \
    can_access_protected_resource_at_within_expiration_of_refresh_token

cargo test --package tests -- --ignored \
    cannot_access_protected_resource_at_expired_expiration_of_refresh_token
