#!/bin/sh
set -eu

# dpkg may replace a file from an earlier Khor package. An independently
# installed command at the same path retains its installation ownership.
if [ -e /usr/bin/khor ] || [ -L /usr/bin/khor ]; then
  owner=$(dpkg-query -S /usr/bin/khor 2>/dev/null || true)
  case "$owner" in
    'khor: /usr/bin/khor'|'khor:amd64: /usr/bin/khor'|'khor:arm64: /usr/bin/khor') ;;
    *)
      echo 'An independent command already occupies /usr/bin/khor. Move it using its original installer before installing this package.' >&2
      exit 1
      ;;
  esac
fi
