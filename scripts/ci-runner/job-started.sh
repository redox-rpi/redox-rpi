#!/bin/bash
# Job-started hook of our runner (ACTIONS_RUNNER_HOOK_JOB_STARTED in the
# Containerfile, #426). The runner runs it before the first step of every
# job, and a workflow cannot skip it. It fails the job when the event is a
# pull request from another repository (a fork): a fork PR can change
# runs-on in its own copy of the workflow, so runs-on alone does not keep
# forks off this runner. docs/build-host.md "CI runner".
set -euo pipefail
repo=redox-rpi/redox-rpi
event=${GITHUB_EVENT_PATH:-}
if [ -z "$event" ] || [ ! -r "$event" ]; then
	echo "job-started hook: no readable event file, refusing the job" >&2
	exit 1
fi
# Every pull_request* event carries .pull_request; other events pass.
head=$(jq -r 'if has("pull_request") then (.pull_request.head.repo.full_name // "unknown") else empty end' "$event")
if [ -n "$head" ] && [ "$head" != "$repo" ]; then
	echo "job-started hook: pull request from $head, not $repo; this runner takes no fork PRs (#426)" >&2
	exit 1
fi
