#!/bin/sh
# Runs in the `backup` Kamal accessory: dump the database once a day and keep
# BACKUP_KEEP_DAYS days of dumps in /backups on the server.
#
# Optional, configured in config/deploy.yml:
#   BACKUP_REMOTE         rclone destination (e.g. offsite:my-bucket/app). Each dump
#                         is also encrypted and uploaded there, so losing the
#                         server doesn't lose the backups.
#   BACKUP_AGE_RECIPIENT  age public key the off-site copies are encrypted to.
#                         Required with BACKUP_REMOTE. Keep the private key off
#                         the server.
#   BACKUP_PING_URL       URL fetched after each fully successful run, for a
#                         dead-man's-switch monitor (e.g. healthchecks.io) that
#                         alerts when backups stop.
#   BACKUP_ONCE=1         run one backup and exit (for testing and restore drills).
#
# Restore a local dump with:
#   pg_restore --clean --if-exists -d app /backups/<file>.dump
# and an off-site one with:
#   age -d -i key.txt <file>.dump.age | pg_restore --clean --if-exists -d app
set -eu

dir="${BACKUP_DIR:-/backups}"

if [ -n "${BACKUP_REMOTE:-}" ] && [ -z "${BACKUP_AGE_RECIPIENT:-}" ]; then
  echo "backup: BACKUP_REMOTE is set but BACKUP_AGE_RECIPIENT isn't; refusing to upload unencrypted dumps" >&2
  exit 1
fi

# The postgres image doesn't include these; install only what's configured.
need=""
[ -z "${BACKUP_REMOTE:-}" ] || need="rclone age"
[ -z "${BACKUP_PING_URL:-}" ] || need="$need curl"
for tool in $need; do
  command -v "$tool" >/dev/null || apk add --no-cache "$tool" >/dev/null
done

backup() {
  file="$dir/$(date -u +%Y-%m-%dT%H%M%SZ).dump"
  if ! pg_dump --format=custom --file="$file.partial"; then
    rm -f "$file.partial"
    echo "backup: pg_dump failed" >&2
    return 1
  fi
  mv "$file.partial" "$file"
  echo "backup: wrote $file"

  if [ -n "${BACKUP_REMOTE:-}" ]; then
    remote="$BACKUP_REMOTE/$(basename "$file").age"
    # Encrypt to a file first: piping into the upload would hide an age failure
    # and upload an empty file.
    if ! age -r "$BACKUP_AGE_RECIPIENT" -o "$file.age" "$file" ||
      ! rclone copyto "$file.age" "$remote"; then
      rm -f "$file.age"
      echo "backup: upload to $remote failed" >&2
      return 1
    fi
    rm -f "$file.age"
    echo "backup: uploaded $remote"
  fi
}

while true; do
  status=0
  if backup; then
    [ -z "${BACKUP_PING_URL:-}" ] || curl -fsS -m 10 --retry 3 -o /dev/null "$BACKUP_PING_URL" ||
      echo "backup: ping failed" >&2
  else
    status=1
    echo "backup: will retry next run" >&2
  fi
  # Off-site copies expire through the bucket's lifecycle rules, not here.
  find "$dir" -name '*.dump' -mtime +"${BACKUP_KEEP_DAYS:-7}" -delete
  [ "${BACKUP_ONCE:-}" != 1 ] || exit "$status"
  sleep 86400
done
