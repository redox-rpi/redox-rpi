#!/bin/sh
# Test broker that wants a client certificate (Mosquitto require_certificate),
# for #115. Runs on the build host next to the TLS test broker of
# mqtt-tls-test and uses its CA (~/mqtt-broker/tls/ca.crt, ca.key).
#
# Makes, once, in ~/mqtt-broker/client/ (never in the repository):
#   client.crt, client.key  signed by the test CA, CN redox-rpi
#   other.crt, other.key    signed by another CA the broker does not trust
# then runs mosquitto on <address>:8884 with require_certificate and the
# certificate's CN as the user name, until stopped or for <seconds>. A
# subscriber with the client certificate logs every message with a time stamp
# to ~/mqtt-test/client-cert-broker.out.
#
# Usage: client-cert-broker.sh [address] [seconds]
#   QEMU: client-cert-broker.sh 127.0.0.1 &   (the guest uses 10.0.2.2:8884)
#   Pi:   client-cert-broker.sh 192.168.178.120 3600 &
set -eu
ADDR=${1:-127.0.0.1}
SECS=${2:-3600}
B=$HOME/mqtt-broker
C=$B/client
export LD_LIBRARY_PATH=$B/root/usr/lib/x86_64-linux-gnu

mkdir -p "$C" "$HOME/mqtt-test"
if [ ! -f "$C/client.crt" ]; then
    (
        cd "$C"
        umask 077
        printf 'extendedKeyUsage = clientAuth\n' > ext.cnf
        openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:P-256 -out client.key
        openssl req -new -key client.key -subj /CN=redox-rpi -out client.csr
        openssl x509 -req -in client.csr -CA "$B/tls/ca.crt" -CAkey "$B/tls/ca.key" \
            -CAcreateserial -days 3650 -extfile ext.cnf -out client.crt
        openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:P-256 -nodes -days 3650 \
            -subj "/CN=other test CA" -keyout other-ca.key -out other-ca.crt
        openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:P-256 -out other.key
        openssl req -new -key other.key -subj /CN=intruder -out other.csr
        openssl x509 -req -in other.csr -CA other-ca.crt -CAkey other-ca.key \
            -CAcreateserial -days 3650 -extfile ext.cnf -out other.crt
        rm -f ./*.csr
    )
fi

cat > "$C/mosquitto.conf" <<EOF
per_listener_settings false
allow_anonymous false
persistence false
log_dest file $C/mosquitto.log
listener 8884 $ADDR
cafile $B/tls/ca.crt
certfile $B/tls/server.crt
keyfile $B/tls/server.key
require_certificate true
use_identity_as_username true
EOF

timeout "$SECS" "$B/root/usr/sbin/mosquitto" -c "$C/mosquitto.conf" &
sleep 1
timeout "$SECS" "$B/root/usr/bin/mosquitto_sub" -h "$ADDR" -p 8884 --cafile "$B/tls/ca.crt" \
    --cert "$C/client.crt" --key "$C/client.key" -t "#" -v -F "%I %t %p" \
    > "$HOME/mqtt-test/client-cert-broker.out" 2>&1 &
wait
