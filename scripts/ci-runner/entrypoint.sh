#!/bin/bash
# Registers the runner once (state in the volume /home/runner/state), then
# runs jobs one at a time until stopped. The registration token is read from
# /run/secrets/reg-token on the first start only; it is a one-hour token the
# user creates (docs/build-host.md "CI runner"), never a personal token.
set -euo pipefail
cd /home/runner/actions-runner
state=/home/runner/state
for f in .runner .credentials .credentials_rsaparams; do
	[ -e "$state/$f" ] && ln -sf "$state/$f" "$f"
done
if [ ! -e .runner ]; then
	[ -s /run/secrets/reg-token ] || { echo "not registered and no /run/secrets/reg-token" >&2; exit 1; }
	./config.sh --unattended --url https://github.com/redox-rpi/redox-rpi \
		--token "$(cat /run/secrets/reg-token)" --name homenode \
		--labels homenode --no-default-labels --work /home/runner/work --replace
	for f in .runner .credentials .credentials_rsaparams; do
		mv "$f" "$state/$f" && ln -s "$state/$f" "$f"
	done
fi
exec ./run.sh
