#!/usr/bin/env bash
# Issue #40: run prebuilt programs from Redox' package repository after
# `pkg install`. One line per check: T40 PASS|FAIL <name> (<exit code>).
# The curl and wget checks fetch from homenodellm's HTTP server.
cd /tmp || exit 1
# With arguments, only the checks with those names run.
SEL=" $* "
check() {
    local name="$1"; shift
    [ "$SEL" = "  " ] || [[ "$SEL" == *" $name "* ]] || return 0
    local out
    out="$("$@" 2>&1)"; local rc=$?
    if [ $rc -eq 0 ]; then echo "T40 PASS $name"; else echo "T40 FAIL $name ($rc): $(echo "$out" | tail -2 | tr '\n' ' ' | cut -c1-160)"; fi
}
printf 'alpha\nbeta\ngamma\n' > /tmp/t40.txt
check bash          bash -c 'a=(1 2 3); echo ${#a[@]}'
check dash          dash -c 'echo ok'
check nano          nano --version
check vim           vim --version
check kibi          kibi --version
check smith         bash -c 'command -v smith'
check ripgrep       rg -c beta /tmp/t40.txt
check gawk          gawk 'BEGIN { print 6*7 }'
check sed           sed -n 2p /tmp/t40.txt
check findutils     find /tmp -name t40.txt
check diffutils     bash -c 'diff /tmp/t40.txt /tmp/t40.txt'
check patch         patch --version
check file          file /usr/bin/ion
check python312     python3.12 -c 'import json, re; print(json.dumps({"n": sum(range(10))}))'
check rustpython    rustpython -c 'print(1+1)'
check sqlite3       sqlite3 :memory: 'create table t(x); insert into t values (1),(2); select sum(x) from t;'
check git           bash -c 'rm -rf /tmp/r && git init -q /tmp/r && cd /tmp/r && git -c user.name=t -c user.email=t@t commit -q --allow-empty -m t && git log --oneline'
check curl          curl -fsS -o /dev/null http://192.168.178.120:8080/
check wget          wget -q -O /dev/null http://192.168.178.120:8080/
check tmux          tmux -V
check xz            bash -c 'xz -k -c /tmp/t40.txt | xz -d | grep -q gamma'
check zstd          bash -c 'zstd -q -c /tmp/t40.txt | zstd -d -q | grep -q gamma'
check bzip2         bash -c 'bzip2 -c /tmp/t40.txt | bzip2 -d | grep -q gamma'
check lz4           bash -c 'lz4 -q -c /tmp/t40.txt | lz4 -d -q | grep -q gamma'
check gnu-make      make --version
check rsync         bash -c 'rsync -a /tmp/t40.txt /tmp/t40-copy.txt && cmp /tmp/t40.txt /tmp/t40-copy.txt'
check bottom        btm --version
check pls           pls --version
check miniserve     miniserve --version
check nasm          nasm -v
check patchelf      patchelf --version
check rldd          bash -c 'rldd /usr/bin/rg'
echo "T40 DONE"
