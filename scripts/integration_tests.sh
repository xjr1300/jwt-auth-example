#!/usr/bin/env bash

cargo test -- --ignored \
    --skip can_access_protected_resource_at_within_expiration_of_refresh_token \
    --skip cannot_access_protected_resource_at_expired_expiration_of_refresh_token

cargo test -- --ignored \
    can_access_protected_resource_at_within_expiration_of_refresh_token

cargo test -- --ignored \
    cannot_access_protected_resource_at_expired_expiration_of_refresh_token
