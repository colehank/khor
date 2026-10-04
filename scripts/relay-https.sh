#!/usr/bin/env bash
# relay-https.sh — put the Beijing relay on HTTPS (round NET-7, option B).
#
# Run as root on the relay box, AFTER acme.sh has issued the certificate:
#
#   acme.sh --issue --dns dns_ali -d bj.relay.khor.cloud      # or dns_cf
#   bash relay-https.sh                                       # this script
#
# What it does, each step said as it goes, and each one a no-op when it is
# already so (run it twice and the second run changes nothing):
#
#   1. installs the certificate from acme.sh into /etc/khor-relay/tls,
#      readable by the relay's user, and registers the renewal hook with
#      acme.sh (--install-cert --reloadcmd), so every renewal is copied
#      there and the relay told to read it again;
#   2. writes the hook, /usr/local/lib/khor-relay/tls-renewed.sh;
#   3. checks the certificate and key the way the relay will read them
#      (khor relay --tls-check — an older khor refuses the flag, and is
#      too old to renew in place);
#   4. writes the relay's key file, /etc/khor-relay-https.env (0600);
#   5. writes khor-relay-https.service and starts or restarts it — a
#      SECOND relay beside khor-relay.service, which keeps serving
#      http://101.200.80.77:3340 untouched;
#   6. opens the port in ufw or firewalld when one of them is running;
#   7. asks the new relay for /ping over HTTPS on this machine.
#
# It never stops, edits or restarts khor-relay.service, never touches
# acme.sh's account or DNS key, and never issues a certificate itself.
#
#   bash relay-https.sh --show     print the unit and the hook, change nothing
#
# Overrides (the defaults are the Beijing relay's; the KHOR_RH_* ones exist
# so the script can be run against a scratch directory by its test):
#   KHOR_RH_DOMAIN   bj.relay.khor.cloud
#   KHOR_RH_PORT     3341
#   KHOR_RH_ACME     acme.sh (or ~/.acme.sh/acme.sh)
#   KHOR_RH_ROOT     ""        every path below is put under this prefix
#   KHOR_RH_USER     khor-relay   the user the relay runs as
#   KHOR_RH_GROUP    its group (the same name)
#   KHOR_RH_KHOR     /usr/local/bin/khor
#   KHOR_RH_BIND     [::]
#   KHOR_RH_CAPTIVE  127.0.0.1:3342
set -euo pipefail

DOMAIN="${KHOR_RH_DOMAIN:-bj.relay.khor.cloud}"
PORT="${KHOR_RH_PORT:-3341}"
ROOT="${KHOR_RH_ROOT:-}"
RELAY_USER="${KHOR_RH_USER:-khor-relay}"
RELAY_GROUP="${KHOR_RH_GROUP:-$RELAY_USER}"
BIND="${KHOR_RH_BIND:-[::]}"
KHOR="${KHOR_RH_KHOR:-/usr/local/bin/khor}"
# The key this relay admits by. It is compiled into khor
# (crates/net/src/endpoint.rs, KHOR_RELAYS beside BEIJING_TLS); the two must
# match, and tests/relay_https.sh checks that they do.
TOKEN="19ccb31e891ffa3ab66f95f5b743c7d4b419afe635b97e5e"
# The captive-portal port iroh-relay opens beside a TLS one. On loopback:
# plain HTTP to a domain on a mainland box is what the ICP rules stop.
CAPTIVE="${KHOR_RH_CAPTIVE:-127.0.0.1:3342}"

TLS_DIR="$ROOT/etc/khor-relay/tls"
CERT="$TLS_DIR/fullchain.pem"
KEY="$TLS_DIR/key.pem"
HOOK="$ROOT/usr/local/lib/khor-relay/tls-renewed.sh"
ENV_FILE="$ROOT/etc/khor-relay-https.env"
UNIT="$ROOT/etc/systemd/system/khor-relay-https.service"
STATE="$ROOT/var/lib/khor-relay/state-https"

said() { printf '%s\n' "$*"; }
changed=0
did() { changed=$((changed + 1)); said "  改了：$*"; }
same() { said "  没变：$*"; }

unit_text() {
	cat <<EOF
# Written by scripts/relay-https.sh (khor, round NET-7). The Beijing relay
# over HTTPS, beside khor-relay.service (plain HTTP on 3340), which it
# does not replace.
[Unit]
Description=khor relay over HTTPS (${DOMAIN}:${PORT})
After=network-online.target
Wants=network-online.target

[Service]
User=${RELAY_USER}
EnvironmentFile=${ENV_FILE}
StateDirectory=khor-relay
ExecStart=${KHOR} relay --tls-cert ${CERT} --tls-key ${KEY} --bind ${BIND}:${PORT} --http ${CAPTIVE} --token \${KHOR_RELAY_TOKEN} --rate 1048576 --state ${STATE}
# A renewed certificate is read again in place: no client is dropped.
ExecReload=/bin/kill -HUP \$MAINPID
Restart=always
RestartSec=3

[Install]
WantedBy=multi-user.target
EOF
}

hook_text() {
	cat <<EOF
#!/bin/sh
# acme.sh's --reloadcmd for ${DOMAIN} (written by scripts/relay-https.sh).
# acme.sh has just copied the renewed certificate into ${TLS_DIR}:
# make it readable by the relay, check it the way the relay reads it, and
# tell a running relay to read it again. A relay that is not running is
# left alone (it reads the files when it starts).
set -e
chgrp ${RELAY_GROUP} ${CERT} ${KEY}
chmod 0640 ${CERT} ${KEY}
${KHOR} relay --tls-check --tls-cert ${CERT} --tls-key ${KEY}
if systemctl is-active --quiet khor-relay-https.service; then
	systemctl reload khor-relay-https.service
fi
EOF
}

# Writes $2 to $1 when it differs; says which. $3 is the mode.
put() {
	local path="$1" text="$2" mode="$3" what="$4"
	if [ -f "$path" ] && [ "$(cat "$path")" = "$text" ]; then
		same "$what ($path)"
		chmod "$mode" "$path"
		return 1
	fi
	mkdir -p "$(dirname "$path")"
	printf '%s\n' "$text" >"$path.new"
	chmod "$mode" "$path.new"
	mv "$path.new" "$path"
	did "$what ($path)"
	return 0
}

if [ "${1:-}" = "--show" ]; then
	said "# $UNIT"
	unit_text
	said
	said "# $HOOK"
	hook_text
	exit 0
fi

if [ -z "$ROOT" ] && [ "$(id -u)" != 0 ]; then
	said "要用 root 跑（sudo bash $0）。"
	exit 1
fi

said "khor 中继 HTTPS：${DOMAIN}:${PORT}"

# ── 0. what this needs ───────────────────────────────────────────────
ACME="${KHOR_RH_ACME:-}"
if [ -z "$ACME" ]; then
	if command -v acme.sh >/dev/null 2>&1; then ACME=acme.sh
	elif [ -x "$HOME/.acme.sh/acme.sh" ]; then ACME="$HOME/.acme.sh/acme.sh"
	else
		said "找不到 acme.sh。先装它并签出证书：acme.sh --issue --dns dns_ali -d ${DOMAIN}"
		exit 1
	fi
fi
if [ ! -x "$KHOR" ]; then
	said "找不到 ${KHOR}。先把这一轮构建的 khor 放到那里。"
	exit 1
fi
if ! id "$RELAY_USER" >/dev/null 2>&1; then
	said "没有用户 ${RELAY_USER}（khor-relay.service 用的那个）。先照 docs/NET.md「服务器上的样子」建好。"
	exit 1
fi
# acme.sh keeps an ECC certificate under <domain>_ecc and wants --ecc for it.
ECC=()
if [ -d "${LE_WORKING_DIR:-$HOME/.acme.sh}/${DOMAIN}_ecc" ]; then
	ECC=(--ecc)
fi

# ── 1-2. the certificate, and the hook that keeps it fresh ──────────
said "1. 证书"
mkdir -p "$TLS_DIR"
chgrp "$RELAY_GROUP" "$TLS_DIR"
chmod 0750 "$TLS_DIR"
put "$HOOK" "$(hook_text)" 0755 "续期钩子" || true
before="$( (cat "$CERT" "$KEY" 2>/dev/null || true) | cksum)"
if ! acme_said="$("$ACME" --install-cert -d "$DOMAIN" ${ECC[@]+"${ECC[@]}"} \
	--fullchain-file "$CERT" \
	--key-file "$KEY" \
	--reloadcmd "$HOOK" 2>&1)"; then
	said "acme.sh --install-cert 没成，它的原话："
	said "$acme_said"
	exit 1
fi
after="$( (cat "$CERT" "$KEY" 2>/dev/null || true) | cksum)"
if [ ! -s "$CERT" ] || [ ! -s "$KEY" ]; then
	said "acme.sh 没有把证书放到 $TLS_DIR。它签出 ${DOMAIN} 了吗？（acme.sh --list）"
	exit 1
fi
if [ "$before" = "$after" ]; then same "证书文件 ($CERT)"; else did "证书文件 ($CERT)"; fi
chgrp "$RELAY_GROUP" "$CERT" "$KEY"
chmod 0640 "$CERT" "$KEY"
said "  acme.sh 每次续期后会跑 $HOOK"

# ── 3. the certificate as the relay will read it ─────────────────────
said "2. 检查证书"
if ! "$KHOR" relay --tls-check --tls-cert "$CERT" --tls-key "$KEY"; then
	said "上面是 khor 读证书时的原话。若是「不认识 --tls-check」，这台的 khor 太旧：先换成这一轮构建的。"
	exit 1
fi
if command -v openssl >/dev/null 2>&1; then
	said "  $(openssl x509 -in "$CERT" -noout -enddate)"
	# OpenSSL says "does match" / "does NOT match"; an openssl without
	# -checkhost says neither, and the relay's own check above stands.
	host="$(openssl x509 -in "$CERT" -noout -checkhost "$DOMAIN" 2>/dev/null || true)"
	case "$host" in
	*"does NOT match"*)
		said "证书不是给 ${DOMAIN} 的："
		openssl x509 -in "$CERT" -noout -subject || true
		exit 1
		;;
	*"does match"*) said "  证书是给 ${DOMAIN} 的" ;;
	esac
fi

# ── 4-5. the key and the service ─────────────────────────────────────
said "3. 服务"
env_changed=0
put "$ENV_FILE" "KHOR_RELAY_TOKEN=${TOKEN}" 0600 "密钥文件" && env_changed=1
unit_changed=0
put "$UNIT" "$(unit_text)" 0644 "khor-relay-https.service" && unit_changed=1
systemctl daemon-reload
systemctl enable khor-relay-https.service >/dev/null 2>&1
if [ "$env_changed$unit_changed" != "00" ] || ! systemctl is-active --quiet khor-relay-https.service; then
	systemctl restart khor-relay-https.service
	did "khor-relay-https 已（重新）启动"
else
	same "khor-relay-https 在跑"
fi
if systemctl is-active --quiet khor-relay.service; then
	said "  khor-relay.service（http://…:3340）照常在跑，没动它"
fi

# ── 6. the port ───────────────────────────────────────────────────────
said "4. 端口"
if command -v ufw >/dev/null 2>&1 && ufw status 2>/dev/null | grep -q "Status: active"; then
	if ufw status | grep -qE "^${PORT}/tcp +ALLOW"; then same "ufw 已放行 ${PORT}/tcp"; else ufw allow "${PORT}/tcp" >/dev/null; did "ufw 放行 ${PORT}/tcp"; fi
elif command -v firewall-cmd >/dev/null 2>&1 && firewall-cmd --state >/dev/null 2>&1; then
	if firewall-cmd --query-port="${PORT}/tcp" >/dev/null 2>&1; then same "firewalld 已放行 ${PORT}/tcp"; else
		firewall-cmd --permanent --add-port="${PORT}/tcp" >/dev/null && firewall-cmd --reload >/dev/null
		did "firewalld 放行 ${PORT}/tcp"
	fi
else
	said "  本机没有在跑的 ufw / firewalld"
fi
said "  阿里云控制台的防火墙要另外放行 TCP ${PORT}（这个脚本碰不到）"

# ── 7. does it answer ─────────────────────────────────────────────────
said "5. 本机试连"
ok=0
for _ in $(seq 1 20); do
	if curl -fsS --max-time 3 --resolve "${DOMAIN}:${PORT}:127.0.0.1" "https://${DOMAIN}:${PORT}/ping" >/dev/null 2>&1; then
		ok=1
		break
	fi
	sleep 0.5
done
if [ "$ok" = 1 ]; then
	said "  https://${DOMAIN}:${PORT}/ping 在本机答了"
else
	said "  https://${DOMAIN}:${PORT}/ping 在本机没有答。看日志：journalctl -u khor-relay-https -n 50"
	exit 1
fi

said
if [ "$changed" = 0 ]; then said "完成，什么都没改。"; else said "完成，改了 ${changed} 处。"; fi
said "接下来：从一台实验室机器试 curl -sS https://${DOMAIN}:${PORT}/ping"
said "（不通又确定端口放行了，就是 443 以外的端口也被备案拦了，见 audit/net-7.md「拦了怎么办」）。"
said "所有机器都换成这一轮的 khor 之后，才可以停 khor-relay.service。"
