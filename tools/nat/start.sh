#!/bin/sh
set -eu
role="$1"
if [ "$role" = relay ]; then
    exec ssk node --config /opt/ssk/relay.yaml
fi
# Rules apply inside this disposable container only. TCP relay and Docker DNS work.
iptables -A OUTPUT -o lo -j ACCEPT
iptables -A OUTPUT -p udp --dport 53 -j ACCEPT
iptables -A OUTPUT -p udp -j DROP
iptables -A INPUT -i lo -j ACCEPT
iptables -A INPUT -p udp -j DROP
cat > "/run/ssk/$role.yaml" <<EOF
identity_file: /run/ssk/$role.key
contact_cache: null
relays: [http://relay:3340]
EOF
if [ "$role" = a ]; then
    exec ssk bench --players 1 --seconds 45 --warmup 10 --config /run/ssk/a.yaml --invite-file /run/ssk/a.ticket --output /run/ssk/a.json
fi
count=0
while [ ! -s /run/ssk/a.ticket ]; do
    count=$((count+1))
    [ "$count" -lt 90 ] || exit 1
    sleep 1
done
exec ssk bench --players 1 --seconds 40 --warmup 10 --config /run/ssk/b.yaml --connect "$(cat /run/ssk/a.ticket)" --output /run/ssk/b.json
