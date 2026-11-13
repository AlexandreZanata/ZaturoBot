#!/usr/bin/env bash
# Read-only inventory. Load the workstation's private .env explicitly before running.
set -euo pipefail

: "${ZATURO_DEPLOY_SSH_HOST:?Set the confirmed Oracle host}"
: "${ZATURO_DEPLOY_SSH_USER:?Set the existing administrative user}"
: "${ZATURO_DEPLOY_SSH_PORT:?Set the confirmed SSH port}"
: "${ZATURO_DEPLOY_SSH_KEY:?Set the existing private-key file path; never key contents}"

[[ "$ZATURO_DEPLOY_SSH_HOST" =~ ^[a-zA-Z0-9.-]+$ ]] || { echo 'Invalid host' >&2; exit 1; }
[[ "$ZATURO_DEPLOY_SSH_USER" =~ ^[a-z_][a-z0-9_-]*$ ]] || { echo 'Invalid user' >&2; exit 1; }
[[ "$ZATURO_DEPLOY_SSH_PORT" =~ ^[0-9]{1,5}$ ]] || { echo 'Invalid port' >&2; exit 1; }
(( 10#$ZATURO_DEPLOY_SSH_PORT >= 1 && 10#$ZATURO_DEPLOY_SSH_PORT <= 65535 )) || { echo 'Invalid port' >&2; exit 1; }
[[ "$ZATURO_DEPLOY_SSH_KEY" = /* && -f "$ZATURO_DEPLOY_SSH_KEY" && -r "$ZATURO_DEPLOY_SSH_KEY" ]] || { echo 'Private-key file is unavailable' >&2; exit 1; }

exec ssh -p "$ZATURO_DEPLOY_SSH_PORT" -i "$ZATURO_DEPLOY_SSH_KEY" \
    -o BatchMode=yes -o IdentitiesOnly=yes -o StrictHostKeyChecking=yes \
    -o UpdateHostKeys=no -o ConnectTimeout=10 \
    "$ZATURO_DEPLOY_SSH_USER@$ZATURO_DEPLOY_SSH_HOST" 'bash -s' <<'REMOTE'
set -eu
id
hostname
uname -a
free -m
df -h /
systemctl list-units --type=service --state=running --no-pager
systemctl list-timers --all --no-pager
sudo -n ss -lntup
for path in /opt/ppg /var/lib/ppg /var/backups/ppg /etc/ppg /var/log/ppg /home/ubuntu/ppg-src; do
    if sudo -n test -e "$path"; then sudo -n du -sh -- "$path"; fi
done
if command -v ufw >/dev/null; then sudo -n ufw status verbose; fi
REMOTE
