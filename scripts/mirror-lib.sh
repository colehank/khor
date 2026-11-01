#!/bin/sh
# Shared by the local mirror writer and the remote publication command.
# Installer checksum helpers must be loaded before these declarations.
khor_validate_mirror_release() (
    set -eu
    _release="$1"
    [ -d "$_release" ] && [ ! -L "$_release" ] || exit 1
    for _asset in khor-linux-x86_64 khor-linux-aarch64 khor-macos-arm64 khor-macos-x86_64; do
        for _suffix in .gz .gz.sha256; do
            [ -f "$_release/$_asset$_suffix" ] && [ ! -L "$_release/$_asset$_suffix" ] || {
                echo "khor-mirror: incomplete release ($_asset$_suffix)" >&2; exit 1;
            }
        done
        _expected=$(khor_published_digest "$_release/$_asset.gz.sha256" "$_asset.gz") || exit 1
        _actual=$(khor_file_digest "$_release/$_asset.gz")
        [ "$_expected" = "$_actual" ] || { echo "khor-mirror: checksum mismatch ($_asset)" >&2; exit 1; }
        # Validate the whole gzip and program header without executing it.
        _program=$(mktemp "$_release/.verify.XXXXXXXX")
        trap 'rm -f "$_program"' EXIT
        gzip -dc "$_release/$_asset.gz" > "$_program" || exit 1
        case "$(head -c 4 "$_program" | od -An -tx1 | tr -d ' \n')" in
            7f454c46|cffaedfe|cafebabe) : ;;
            *) echo "khor-mirror: asset is not a program ($_asset)" >&2; exit 1 ;;
        esac
        rm -f "$_program"
        trap - EXIT
    done
    [ -f "$_release/install.sh" ] && [ ! -L "$_release/install.sh" ] || exit 1
    /bin/sh -n "$_release/install.sh"
)

khor_commit_mirror_release() (
    set -eu
    _root="$1" _stage="$2" _tag="$3"
    khor_valid_tag "$_tag" || exit 1
    for _entry in install.sh VERSION; do
        _destination="$_root/$_entry"
        if [ -L "$_destination" ] || { [ -e "$_destination" ] && [ ! -f "$_destination" ]; }; then
            echo "khor-mirror: publication target is not a regular file ($_entry)" >&2; exit 1
        fi
    done
    [ ! -L "$_root/releases" ] || { echo "khor-mirror: releases must not be a symbolic link" >&2; exit 1; }
    khor_validate_mirror_release "$_stage"
    mkdir -p "$_root/releases"
    _generation="$_root/releases/$_tag"
    if [ -e "$_generation" ] || [ -L "$_generation" ]; then
        [ -d "$_generation" ] && [ ! -L "$_generation" ] || exit 1
        for _file in install.sh khor-linux-x86_64.gz khor-linux-x86_64.gz.sha256 khor-linux-aarch64.gz khor-linux-aarch64.gz.sha256 khor-macos-arm64.gz khor-macos-arm64.gz.sha256 khor-macos-x86_64.gz khor-macos-x86_64.gz.sha256; do
            [ -f "$_generation/$_file" ] && [ ! -L "$_generation/$_file" ] && cmp -s "$_stage/$_file" "$_generation/$_file" || {
                echo "khor-mirror: refusing to replace immutable $_tag ($_file)" >&2; exit 1;
            }
        done
    else
        mv "$_stage" "$_generation"
    fi
    # Staging is private. Published static assets must also be readable by the
    # web server when it runs under a different account from the deploy user.
    for _file in install.sh khor-linux-x86_64.gz khor-linux-x86_64.gz.sha256 khor-linux-aarch64.gz khor-linux-aarch64.gz.sha256 khor-macos-arm64.gz khor-macos-arm64.gz.sha256 khor-macos-x86_64.gz khor-macos-x86_64.gz.sha256; do
        chmod 644 "$_generation/$_file"
    done
    chmod 755 "$_root" "$_root/releases" "$_generation"
    _bootstrap=$(mktemp "$_root/.installer.XXXXXXXX")
    _pointer=""
    trap 'rm -f "$_bootstrap"; [ -z "$_pointer" ] || rm -f "$_pointer"' EXIT
    cp "$_generation/install.sh" "$_bootstrap"
    chmod 644 "$_bootstrap"
    mv -f "$_bootstrap" "$_root/install.sh"
    _pointer=$(mktemp "$_root/.VERSION.XXXXXXXX")
    printf '%s\n' "$_tag" > "$_pointer"
    chmod 644 "$_pointer"
    mv -f "$_pointer" "$_root/VERSION"
)

khor_publish_mirror_release() (
    set -eu
    _root="$1" _stage="$2" _tag="$3"
    mkdir -p "$_root"
    mkdir "$_root/.sync.lock" 2>/dev/null || { echo "khor-mirror: another writer holds the publication lock" >&2; exit 1; }
    trap 'rmdir "$_root/.sync.lock"' EXIT
    khor_commit_mirror_release "$_root" "$_stage" "$_tag"
)
