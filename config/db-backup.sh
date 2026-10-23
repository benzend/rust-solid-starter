#!/bin/sh
# Runs in the `backup` Kamal accessory: dump the database once a day and keep
# BACKUP_KEEP_DAYS days of dumps in /backups. Restore one with:
#   pg_restore --clean --if-exists -d app /backups/<file>.dump
set -eu

while true; do
  file="/backups/$(date -u +%Y-%m-%dT%H%M%SZ).dump"
  if pg_dump --format=custom --file="$file.partial"; then
    mv "$file.partial" "$file"
    echo "backup: wrote $file"
  else
    rm -f "$file.partial"
    echo "backup: pg_dump failed, will retry next run" >&2
  fi
  find /backups -name '*.dump' -mtime +"${BACKUP_KEEP_DAYS:-7}" -delete
  sleep 86400
done
