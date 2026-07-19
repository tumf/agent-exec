#!/usr/bin/env bash
set -euo pipefail

usage() {
  printf 'Usage: %s --tag vX.Y.Z [--upload] [--dist-dir DIR]\n' "$0" >&2
}

tag=''
upload=false
dist_dir=dist

while (($#)); do
  case "$1" in
    --tag)
      (($# >= 2)) || { usage; exit 2; }
      tag=$2
      shift 2
      ;;
    --upload)
      upload=true
      shift
      ;;
    --dist-dir)
      (($# >= 2)) || { usage; exit 2; }
      dist_dir=$2
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      usage
      exit 2
      ;;
  esac
done

[[ $tag =~ ^v[0-9]+\.[0-9]+\.[0-9]+([-.][0-9A-Za-z.-]+)?$ ]] || {
  printf 'tag must be a version tag such as v0.2.25\n' >&2
  exit 2
}
git rev-parse -q --verify "refs/tags/$tag" >/dev/null || {
  printf 'tag does not exist locally: %s\n' "$tag" >&2
  exit 2
}
[[ $(git describe --exact-match --tags HEAD 2>/dev/null || true) == "$tag" ]] || {
  printf 'HEAD must be checked out at %s\n' "$tag" >&2
  exit 2
}

case $(uname -m) in
  arm64) target=aarch64-apple-darwin ;;
  x86_64) target=x86_64-apple-darwin ;;
  *) printf 'unsupported macOS architecture: %s\n' "$(uname -m)" >&2; exit 2 ;;
esac

archive="agent-exec-${tag}-${target}.tar.gz"
mkdir -p "$dist_dir"

cargo build --locked --release
target_dir=$(cargo metadata --format-version 1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')
binary="$target_dir/release/agent-exec"
"$binary" --version
AGENT_EXEC_ROOT=$(mktemp -d) "$binary" run -- echo release-smoke | python3 -c 'import json, sys; response = json.load(sys.stdin); assert response["ok"] is True; assert response["state"] == "exited"; assert response["exit_code"] == 0'

tar -C "$(dirname "$binary")" -czf "$dist_dir/$archive" agent-exec
(
  cd "$dist_dir"
  shasum -a 256 "$archive" > "$archive.sha256"
)

if "$upload"; then
  gh release view "$tag" >/dev/null
  gh release upload "$tag" "$dist_dir/$archive" "$dist_dir/$archive.sha256"
fi

printf '%s\n%s\n' "$dist_dir/$archive" "$dist_dir/$archive.sha256"
