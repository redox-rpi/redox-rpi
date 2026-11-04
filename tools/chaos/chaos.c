/*
 * chaos: fault injection on Redox (issue #196).
 *
 * Runs randomized rounds, each one fault of one kind, and checks after every
 * round that the system still answers. Every fault runs in a child process
 * with a deadline, so a round that hangs or crashes only loses that child;
 * a child that SIGKILL cannot remove is reported. Every round prints its
 * seed: `chaos --kind <kind> --round-seed <seed>` repeats it.
 *
 * Written in C on purpose: it is built against our relibc and exercises its
 * C side (stdio, pthread, fork, signals, fcntl, statvfs).
 *
 *   chaos [--rounds N] [--seed S] [--kind K[,K]...] [--round-seed S]
 *         [--timeout SEC] [--mem-reserve MB] [--fill-max MB] [--data DIR]
 *         [--net HOST:PORT] [--list] [--quiet]
 *
 * Exit code 0 when every round and health check passed, 1 otherwise.
 */

#include <errno.h>
#include <fcntl.h>
#include <pthread.h>
#include <signal.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/statvfs.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>
#include <dirent.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <sys/socket.h>

#define MAX_FDS 4096
#define MAX_KIDS 64
#define MAX_THREADS 64
#define CHUNK (1024 * 1024)

/* Options */
static unsigned rounds = 20;
static uint64_t master_seed;
static int have_round_seed;
static uint64_t round_seed_opt;
static unsigned timeout_s = 60;
static unsigned mem_reserve_mb = 48;
static unsigned fill_max_mb;            /* 0: until the disk is full */
static const char *data_dir = "/scheme/data";
static const char *self_path = "/usr/bin/chaos";
static const char *net_target;          /* host:port of scripts/chaos-host.py serve */
static int quiet;

/* ---- small helpers ------------------------------------------------------ */

static uint64_t rng_state;

static uint64_t rnd(void)
{
    /* xorshift64*: small, fast, and the same on every platform */
    uint64_t x = rng_state;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    rng_state = x;
    return x * 0x2545F4914F6CDD1DULL;
}

static void seed_rng(uint64_t seed)
{
    rng_state = seed ? seed : 0x9E3779B97F4A7C15ULL;
    for (int i = 0; i < 4; i++)
        rnd();
}

static unsigned rnd_below(unsigned n)
{
    return n ? (unsigned)(rnd() % n) : 0;
}

static uint64_t now_ms(void)
{
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (uint64_t)ts.tv_sec * 1000 + ts.tv_nsec / 1000000;
}

static void sleep_ms(unsigned ms)
{
    struct timespec ts = { ms / 1000, (long)(ms % 1000) * 1000000L };
    while (nanosleep(&ts, &ts) < 0 && errno == EINTR)
        ;
}

static void say(const char *fmt, ...)
{
    va_list ap;
    va_start(ap, fmt);
    printf("chaos: ");
    vprintf(fmt, ap);
    printf("\n");
    fflush(stdout);
    va_end(ap);
}

/* Detail lines from inside a fault; --quiet drops them. */
static void note(const char *fmt, ...)
{
    if (quiet)
        return;
    va_list ap;
    va_start(ap, fmt);
    printf("chaos:   ");
    vprintf(fmt, ap);
    printf("\n");
    fflush(stdout);
    va_end(ap);
}

static uint64_t free_memory(void)
{
    struct statvfs st;
    if (statvfs("/scheme/memory", &st) < 0)
        return 0;
    return (uint64_t)st.f_bfree * st.f_bsize;
}

/*
 * Run fn(arg) in a child and wait at most timeout_ms. Returns the child's
 * exit code, 128+signal when a signal ended it, -1 when it ran out of time
 * (it is killed then), -2 when even SIGKILL did not end it within 10 s
 * and -3 when fork failed.
 */
static int run_child(int (*fn)(void *), void *arg, unsigned timeout_ms)
{
    fflush(stdout);
    pid_t pid = fork();
    if (pid < 0)
        return -3;
    if (pid == 0)
        _exit(fn(arg));

    uint64_t end = now_ms() + timeout_ms;
    int status;
    for (;;) {
        pid_t r = waitpid(pid, &status, WNOHANG);
        if (r == pid)
            break;
        if (r < 0 && errno != EINTR)
            return -3;
        if (now_ms() >= end) {
            kill(pid, SIGKILL);
            uint64_t kill_end = now_ms() + 10000;
            while ((r = waitpid(pid, &status, WNOHANG)) == 0 && now_ms() < kill_end)
                sleep_ms(10);
            return r == pid ? -1 : -2;
        }
        sleep_ms(5);
    }
    if (WIFEXITED(status))
        return WEXITSTATUS(status);
    if (WIFSIGNALED(status))
        return 128 + WTERMSIG(status);
    return 255;
}

/* ---- health check ------------------------------------------------------- */

static int child_true(void *arg)
{
    (void)arg;
    return 0;
}

/* Everything that must still work after a round; returns 0 or prints why. */
static int health(void)
{
    char buf[256];
    int ok = 1;

    int r = run_child(child_true, NULL, 10000);
    if (r != 0) {
        say("health FAIL: fork+exit gave %d", r);
        ok = 0;
    }

    pid_t pid = fork();
    if (pid == 0) {
        execl(self_path, self_path, "--noop", (char *)NULL);
        _exit(127);
    }
    if (pid < 0) {
        say("health FAIL: fork for exec: %s", strerror(errno));
        ok = 0;
    } else {
        uint64_t end = now_ms() + 15000;
        int status = 0;
        pid_t w;
        while ((w = waitpid(pid, &status, WNOHANG)) == 0 && now_ms() < end)
            sleep_ms(10);
        if (w != pid) {
            kill(pid, SIGKILL);
            waitpid(pid, &status, 0);
            say("health FAIL: exec of %s did not finish in 15 s", self_path);
            ok = 0;
        } else if (!WIFEXITED(status) || WEXITSTATUS(status) != 0) {
            say("health FAIL: exec of %s gave status 0x%x", self_path, status);
            ok = 0;
        }
    }

    int fd = open("/tmp/chaos-health", O_CREAT | O_TRUNC | O_RDWR, 0644);
    if (fd < 0) {
        say("health FAIL: open /tmp/chaos-health: %s", strerror(errno));
        ok = 0;
    } else {
        const char msg[] = "chaos health check\n";
        if (write(fd, msg, sizeof msg) != (ssize_t)sizeof msg
            || lseek(fd, 0, SEEK_SET) != 0
            || read(fd, buf, sizeof msg) != (ssize_t)sizeof msg
            || memcmp(buf, msg, sizeof msg) != 0) {
            say("health FAIL: /tmp write and read back: %s", strerror(errno));
            ok = 0;
        }
        close(fd);
        unlink("/tmp/chaos-health");
    }

    fd = open("/scheme/sys/uname", O_RDONLY);
    if (fd < 0 || read(fd, buf, sizeof buf) <= 0) {
        say("health FAIL: read /scheme/sys/uname: %s", strerror(errno));
        ok = 0;
    }
    if (fd >= 0)
        close(fd);

    return ok ? 0 : 1;
}

/* ---- faults ------------------------------------------------------------- */

/* Memory: allocate and touch 1 MiB chunks until malloc fails or only
 * --mem-reserve MiB are left, then free everything. */
static int f_memory(void *arg)
{
    (void)arg;
    uint64_t reserve = (uint64_t)mem_reserve_mb * CHUNK;
    size_t cap = 4096, n = 0;
    char **chunks = malloc(cap * sizeof *chunks);
    if (!chunks)
        return 2;
    uint64_t start_free = free_memory();
    const char *why = "cap";
    while (n < cap) {
        if (reserve && n % 8 == 0) {
            uint64_t f = free_memory();
            if (f && f < reserve + CHUNK) {
                why = "reserve";
                break;
            }
        }
        char *p = malloc(CHUNK);
        if (!p) {
            why = "malloc failed";
            break;
        }
        /* touch every page so it is really mapped */
        for (size_t i = 0; i < CHUNK; i += 4096)
            p[i] = (char)i;
        chunks[n++] = p;
    }
    note("memory: %zu MiB allocated (%s), free before %llu MiB, at peak %llu MiB",
         n, why, (unsigned long long)(start_free >> 20),
         (unsigned long long)(free_memory() >> 20));
    /* free in random order to churn the allocator */
    while (n) {
        size_t i = rnd_below(n);
        free(chunks[i]);
        chunks[i] = chunks[--n];
    }
    free(chunks);
    return 0;
}

/* Processes: waves of forks that exit at once, some of them nested. */
static int f_procs(void *arg)
{
    (void)arg;
    unsigned waves = 2 + rnd_below(4), forks = 0, fails = 0;
    for (unsigned w = 0; w < waves; w++) {
        pid_t kids[MAX_KIDS];
        unsigned n = 8 + rnd_below(MAX_KIDS - 8), got = 0;
        for (unsigned i = 0; i < n; i++) {
            pid_t p = fork();
            if (p == 0) {
                if (rnd_below(4) == 0) {
                    /* grandchild: exits with a code the child checks */
                    pid_t g = fork();
                    if (g == 0)
                        _exit(7);
                    int st;
                    if (g < 0 || waitpid(g, &st, 0) != g || !WIFEXITED(st)
                        || WEXITSTATUS(st) != 7)
                        _exit(3);
                }
                _exit(i & 0x3f);
            }
            if (p < 0) {
                fails++;
                break;
            }
            kids[got++] = p;
        }
        forks += got;
        for (unsigned i = 0; i < got; i++) {
            int st;
            if (waitpid(kids[i], &st, 0) != kids[i]) {
                note("procs: waitpid %d: %s", kids[i], strerror(errno));
                return 3;
            }
            if (!WIFEXITED(st) || WEXITSTATUS(st) != (int)(i & 0x3f)) {
                note("procs: child %u ended with status 0x%x", i, st);
                return 4;
            }
        }
    }
    note("procs: %u forks in %u waves, %u fork failures", forks, waves, fails);
    return 0;
}

static volatile unsigned long thread_work;
static pthread_mutex_t work_lock = PTHREAD_MUTEX_INITIALIZER;

static void *thread_body(void *arg)
{
    unsigned spins = (unsigned)(uintptr_t)arg;
    for (unsigned i = 0; i < spins; i++) {
        pthread_mutex_lock(&work_lock);
        thread_work++;
        pthread_mutex_unlock(&work_lock);
        if (i % 64 == 0)
            sched_yield();
    }
    return arg;
}

/* Threads: waves of threads that share a mutex; the total must add up. */
static int f_threads(void *arg)
{
    (void)arg;
    unsigned waves = 2 + rnd_below(4), made = 0, fails = 0;
    unsigned long expect = 0;
    thread_work = 0;
    for (unsigned w = 0; w < waves; w++) {
        pthread_t t[MAX_THREADS];
        unsigned n = 4 + rnd_below(MAX_THREADS - 4), got = 0;
        for (unsigned i = 0; i < n; i++) {
            unsigned spins = 100 + rnd_below(2000);
            if (pthread_create(&t[got], NULL, thread_body, (void *)(uintptr_t)spins) != 0) {
                fails++;
                break;
            }
            expect += spins;
            got++;
        }
        made += got;
        for (unsigned i = 0; i < got; i++)
            pthread_join(t[i], NULL);
    }
    note("threads: %u threads in %u waves, %u create failures", made, waves, fails);
    if (thread_work != expect) {
        note("threads: counter %lu, expected %lu", thread_work, expect);
        return 4;
    }
    return 0;
}

static volatile sig_atomic_t sig_count;

static void on_signal(int sig)
{
    (void)sig;
    sig_count++;
}

/* Signals: one or more children send SIGUSR1/SIGUSR2 as fast as they can
 * while this process reads and writes a pipe; interrupted calls must give
 * EINTR or finish, never wrong data. */
static int f_signals(void *arg)
{
    (void)arg;
    struct sigaction sa;
    memset(&sa, 0, sizeof sa);
    sa.sa_handler = on_signal;
    sa.sa_flags = rnd_below(2) ? SA_RESTART : 0;
    sigaction(SIGUSR1, &sa, NULL);
    sigaction(SIGUSR2, &sa, NULL);

    int p[2];
    if (pipe(p) < 0)
        return 2;
    pid_t parent = getpid();
    unsigned senders = 1 + rnd_below(3), ms = 500 + rnd_below(2500);
    pid_t kids[4];
    for (unsigned i = 0; i < senders; i++) {
        kids[i] = fork();
        if (kids[i] == 0) {
            uint64_t end = now_ms() + ms;
            int sig = i % 2 ? SIGUSR2 : SIGUSR1;
            while (now_ms() < end)
                if (kill(parent, sig) < 0)
                    _exit(5);
            _exit(0);
        }
    }

    unsigned long rounds_ok = 0, eintr = 0;
    uint64_t end = now_ms() + ms;
    while (now_ms() < end) {
        unsigned long v = rounds_ok, back = 0;
        ssize_t w = write(p[1], &v, sizeof v);
        if (w < 0) {
            if (errno == EINTR) {
                eintr++;
                continue;
            }
            note("signals: write: %s", strerror(errno));
            return 3;
        }
        ssize_t r;
        while ((r = read(p[0], &back, sizeof back)) < 0 && errno == EINTR)
            eintr++;
        if (r != sizeof back || back != v) {
            note("signals: pipe gave %zd bytes, value %lu instead of %lu", r, back, v);
            return 4;
        }
        rounds_ok++;
    }
    for (unsigned i = 0; i < senders; i++) {
        int st;
        waitpid(kids[i], &st, 0);
    }
    note("signals: %u senders for %u ms, %ld signals, %lu pipe rounds, %lu EINTR (%s)",
         senders, ms, (long)sig_count, rounds_ok, eintr,
         sa.sa_flags ? "SA_RESTART" : "no SA_RESTART");
    return sig_count ? 0 : 6;
}

/* fd exhaustion: open (and pipe) until EMFILE or MAX_FDS, close them in
 * random order, and check a new open works. */
static int f_fds(void *arg)
{
    (void)arg;
    static int fds[MAX_FDS];
    unsigned n = 0;
    int err = 0;
    const char *paths[] = { "/scheme/null", "/scheme/zero", "/scheme/rand", "/tmp" };
    while (n < MAX_FDS) {
        int fd;
        if (rnd_below(5) == 0 && n + 2 <= MAX_FDS) {
            int p[2];
            if (pipe(p) < 0) {
                err = errno;
                break;
            }
            fds[n++] = p[0];
            fd = p[1];
        } else if (rnd_below(5) == 0 && n > 0) {
            fd = dup(fds[rnd_below(n)]);
        } else {
            const char *path = paths[rnd_below(4)];
            fd = open(path, path[1] == 't' ? O_RDONLY | O_DIRECTORY : O_RDONLY);
        }
        if (fd < 0) {
            err = errno;
            break;
        }
        fds[n++] = fd;
    }
    note("fds: %u open when stopped (%s)", n, err ? strerror(err) : "limit");
    while (n) {
        unsigned i = rnd_below(n);
        if (close(fds[i]) < 0) {
            note("fds: close %d: %s", fds[i], strerror(errno));
            return 3;
        }
        fds[i] = fds[--n];
    }
    int fd = open("/scheme/null", O_RDWR);
    if (fd < 0) {
        note("fds: open after closing all: %s", strerror(errno));
        return 4;
    }
    close(fd);
    return 0;
}

/*
 * Scheme fuzzer. Opens on every scheme in /scheme except those on the deny
 * list; reads only where it cannot steal input or block forever; writes
 * only on the allow list. Calls with invalid fds, flags, offsets and
 * lengths go everywhere it opens.
 */
static const char *deny_open[] = {
    "disk", "memory", "irq", "proc", "thisproc", "debug", "pty", "input",
    "display", "serio", "acpi", "pci", "usb", "xhci", "dwc2", "gpio", "i2c",
    "spi", "firmware", "network", "kernel", "audio", "orbital", "initfs",
    "sudo", "escalate", "log", "tcp", "udp", "ip", "icmp", "chan", "uds",
    "shm", "event", "rpi-", "fbcon", "logging", "netcfg", NULL
};
static const char *allow_write[] = { "null", "zero", "pipe", NULL };
/* /scheme/sys entries that change the system or block */
static const char *deny_sys[] = { "kstop", "exit", "reboot", "update", NULL };

static int in_list(const char *name, const char **list)
{
    for (; *list; list++)
        if (strncmp(name, *list, strlen(*list)) == 0)
            return 1;
    return 0;
}

#define MAX_SCHEMES 64
static char schemes[MAX_SCHEMES][64];
static unsigned n_schemes;

static void list_schemes(void)
{
    n_schemes = 0;
    DIR *d = opendir("/scheme");
    if (!d)
        return;
    struct dirent *e;
    while ((e = readdir(d)) && n_schemes < MAX_SCHEMES) {
        if (e->d_name[0] == '.' || in_list(e->d_name, deny_open))
            continue;
        snprintf(schemes[n_schemes++], sizeof schemes[0], "%s", e->d_name);
    }
    closedir(d);
}

static void random_path(char *out, size_t len, const char *scheme)
{
    static const char *tails[] = {
        "", "/", "//", "/..", "/../..", "/.", "/nonexistent", "/a/b/c/d/e",
        "/\x01\x7f", "/%s%n", "/ ", "/context", "/uname", "/log", "/1", "/0",
        "/4294967296", "/-1", "/cpu", "/scheme",
    };
    unsigned kind = rnd_below(10);
    if (strcmp(scheme, "sys") == 0 && kind < 4) {
        /* a real sys entry, except the dangerous ones */
        static const char *sys[] = { "uname", "context", "cpu", "log", "scheme", "stat", "iostat", "syscall", "block" };
        snprintf(out, len, "/scheme/sys/%s", sys[rnd_below(9)]);
    } else if (kind == 9) {
        /* long path */
        int n = snprintf(out, len, "/scheme/%s/", scheme);
        while ((size_t)n < len - 1)
            out[n++] = 'a' + rnd_below(26);
        out[len - 1] = 0;
    } else {
        snprintf(out, len, "/scheme/%s%s", scheme, tails[rnd_below(sizeof tails / sizeof *tails)]);
    }
    if (strncmp(out, "/scheme/sys/", 12) == 0 && in_list(out + 12, deny_sys))
        snprintf(out, len, "/scheme/sys/uname");
}

static int f_schemes(void *arg)
{
    (void)arg;
    list_schemes();
    if (!n_schemes)
        return 2;
    static char buf[65536];
    char path[1200];
    unsigned calls = 200 + rnd_below(800), opened = 0, errors = 0;
    int fds[16];
    unsigned nfd = 0;
    for (unsigned c = 0; c < calls; c++) {
        unsigned op = rnd_below(10);
        if (op < 3 || nfd == 0) {
            const char *s = schemes[rnd_below(n_schemes)];
            random_path(path, rnd_below(4) ? 256 : sizeof path, s);
            int flags = O_NONBLOCK | O_CLOEXEC;
            int rw = in_list(s, allow_write) ? rnd_below(3) : O_RDONLY;
            flags |= rw == 0 ? O_RDONLY : rw == 1 ? O_WRONLY : O_RDWR;
            if (rnd_below(4) == 0)
                flags |= O_DIRECTORY;
            int fd = open(path, flags);
            if (fd < 0) {
                errors++;
                continue;
            }
            opened++;
            if (nfd == 16) {
                close(fds[0]);
                fds[0] = fds[--nfd];
            }
            fds[nfd++] = fd;
            continue;
        }
        unsigned which = rnd_below(nfd + 1);
        int fd = which == nfd ? (int)(rnd_below(3) == 0 ? -1 : 9000 + rnd_below(1000)) : fds[which];
        size_t lens[] = { 0, 1, 7, 4096, 4097, sizeof buf };
        size_t len = lens[rnd_below(6)];
        long r = 0;
        switch (op) {
        case 3: case 4:
            /* the fd was opened O_NONBLOCK, and schemes that could block
             * or steal input are not opened at all */
            r = read(fd, buf, len);
            break;
        case 5:
            /* write only to fds of allow-listed schemes: opened writable */
            if (fd >= 0 && (fcntl(fd, F_GETFL) & O_ACCMODE) != O_RDONLY) {
                for (size_t i = 0; i < len; i++)
                    buf[i] = (char)rnd();
                r = write(fd, buf, len);
            }
            break;
        case 6: {
            int cmds[] = { F_GETFL, F_SETFL, F_GETFD, F_SETFD, F_DUPFD, 9999, -1 };
            int cmd = cmds[rnd_below(7)];
            long v = rnd_below(3) == 0 ? (long)rnd() : (long)rnd_below(8);
            if (cmd == F_SETFL)
                v = (v & ~(long)O_ACCMODE) | O_NONBLOCK;
            r = fcntl(fd, cmd, v);
            if (cmd == F_DUPFD && r >= 0)
                close((int)r);
            break;
        }
        case 7: {
            off_t offs[] = { 0, 1, -1, 4096, (off_t)1 << 40, -((off_t)1 << 40) };
            r = lseek(fd, offs[rnd_below(6)], (int)rnd_below(5) - 1);
            break;
        }
        case 8: {
            struct stat st;
            r = fstat(fd, &st);
            break;
        }
        default:
            if (which < nfd) {
                r = close(fd);
                fds[which] = fds[--nfd];
            } else {
                r = close(fd);
            }
            break;
        }
        if (r < 0)
            errors++;
    }
    while (nfd)
        close(fds[--nfd]);
    note("schemes: %u calls on %u schemes, %u opens, %u errors", calls, n_schemes, opened, errors);
    return 0;
}

/* Full disk: fill data_dir with one file until ENOSPC (or --fill-max),
 * check small writes then fail cleanly, delete and check space is back. */
static int f_fill(void *arg)
{
    (void)arg;
    struct statvfs st;
    if (statvfs(data_dir, &st) < 0) {
        note("fill: no %s (%s), skipped", data_dir, strerror(errno));
        return 0;
    }
    uint64_t free0 = (uint64_t)st.f_bavail * st.f_bsize;
    char path[256], small[256];
    snprintf(path, sizeof path, "%s/chaos-fill", data_dir);
    snprintf(small, sizeof small, "%s/chaos-small", data_dir);
    int fd = open(path, O_CREAT | O_TRUNC | O_WRONLY, 0644);
    if (fd < 0) {
        note("fill: open %s: %s", path, strerror(errno));
        return 2;
    }
    static char buf[CHUNK];
    for (size_t i = 0; i < sizeof buf; i++)
        buf[i] = (char)rnd();
    uint64_t written = 0, cap = (uint64_t)fill_max_mb * CHUNK;
    int err = 0, out_of_time = 0;
    uint64_t start = now_ms();
    /* stop on our own before the round's deadline kills us: a slow card
     * (QEMU: about 3 MB/s) may not fill up within one round */
    uint64_t end = start + (timeout_s > 20 ? timeout_s - 15 : timeout_s / 2) * 1000ULL;
    while (!cap || written < cap) {
        if (now_ms() >= end) {
            out_of_time = 1;
            break;
        }
        ssize_t w = write(fd, buf, sizeof buf);
        if (w < 0) {
            err = errno;
            break;
        }
        written += w;
        if (w < (ssize_t)sizeof buf) {
            /* short write: the next one says why */
            continue;
        }
    }
    fsync(fd);
    note("fill: wrote %llu MiB in %llu ms of %llu MiB free (%s)",
         (unsigned long long)(written >> 20), (unsigned long long)(now_ms() - start),
         (unsigned long long)(free0 >> 20),
         err ? strerror(err) : out_of_time ? "out of time, not full" : "cap");
    int rc = 0;
    if (err && err != ENOSPC && err != EDQUOT) {
        note("fill: write failed with %s instead of ENOSPC", strerror(err));
        rc = 3;
    }
    if (err) {
        /* while full: small writes must fail cleanly or succeed, and the
         * file system must keep answering */
        int sfd = open(small, O_CREAT | O_TRUNC | O_WRONLY, 0644);
        if (sfd >= 0) {
            ssize_t w = write(sfd, "x", 1);
            note("fill: small write when full: %zd (%s)", w, w < 0 ? strerror(errno) : "ok");
            close(sfd);
            unlink(small);
        } else {
            note("fill: small open when full: %s", strerror(errno));
        }
        DIR *d = opendir(data_dir);
        if (!d) {
            note("fill: opendir when full: %s", strerror(errno));
            rc = 4;
        } else {
            closedir(d);
        }
    }
    close(fd);
    if (unlink(path) < 0) {
        note("fill: unlink: %s", strerror(errno));
        return 5;
    }
    if (statvfs(data_dir, &st) == 0) {
        uint64_t free1 = (uint64_t)st.f_bavail * st.f_bsize;
        note("fill: free after delete %llu MiB", (unsigned long long)(free1 >> 20));
        if (free1 + 16 * (uint64_t)CHUNK < free0) {
            note("fill: %llu MiB not given back", (unsigned long long)((free0 - free1) >> 20));
            rc = 6;
        }
    }
    return rc;
}

/* I/O worker for f_killio: loops on one kind of I/O until killed. */
static int killio_worker(void *arg)
{
    unsigned kind = (unsigned)(uintptr_t)arg;
    static char buf[65536];
    char path[256];
    for (size_t i = 0; i < sizeof buf; i++)
        buf[i] = (char)i;
    if (kind == 0 || kind == 1) {
        snprintf(path, sizeof path, "%s/chaos-killio-%d",
                 kind == 0 ? "/tmp" : data_dir, (int)getpid());
        int fd = open(path, O_CREAT | O_TRUNC | O_RDWR, 0644);
        if (fd < 0)
            return 2;
        for (;;) {
            if (write(fd, buf, sizeof buf) < 0 || fsync(fd) < 0)
                return 3;
            if (lseek(fd, 0, SEEK_END) > 16 * CHUNK)
                ftruncate(fd, 0), lseek(fd, 0, SEEK_SET);
        }
    }
    if (kind == 2) {
        /* blocked in a pipe read nobody writes to */
        int p[2];
        if (pipe(p) < 0)
            return 2;
        read(p[0], buf, 1);
        return 3;
    }
    /* reads of /scheme/sys/context, the kernel's process list */
    for (;;) {
        int fd = open("/scheme/sys/context", O_RDONLY);
        if (fd < 0)
            return 2;
        while (read(fd, buf, sizeof buf) > 0)
            ;
        close(fd);
    }
}

/* Kill during I/O: start workers, SIGKILL each after a random time, and
 * check they are gone and their files can be removed. */
static int f_killio(void *arg)
{
    (void)arg;
    unsigned n = 2 + rnd_below(6), killed = 0;
    int have_data = access(data_dir, W_OK) == 0;
    for (unsigned i = 0; i < n; i++) {
        unsigned kind = rnd_below(4);
        if (kind == 1 && !have_data)
            kind = 0;
        fflush(stdout);
        pid_t p = fork();
        if (p == 0)
            _exit(killio_worker((void *)(uintptr_t)kind));
        if (p < 0)
            return 2;
        sleep_ms(rnd_below(300));
        kill(p, SIGKILL);
        int st;
        uint64_t end = now_ms() + 10000;
        pid_t w;
        while ((w = waitpid(p, &st, WNOHANG)) == 0 && now_ms() < end)
            sleep_ms(5);
        if (w != p) {
            note("killio: worker %d (kind %u) still there 10 s after SIGKILL", p, kind);
            return 3;
        }
        if (!WIFSIGNALED(st)) {
            note("killio: worker %d (kind %u) ended with status 0x%x before the kill", p, kind, st);
        } else {
            killed++;
        }
        char path[256];
        snprintf(path, sizeof path, "%s/chaos-killio-%d", kind == 0 ? "/tmp" : data_dir, (int)p);
        if ((kind == 0 || kind == 1) && unlink(path) < 0 && errno != ENOENT) {
            note("killio: unlink %s: %s", path, strerror(errno));
            return 4;
        }
    }
    note("killio: %u workers, %u killed mid-I/O", n, killed);
    return 0;
}

/* Network client against a misbehaving server (scripts/chaos-host.py
 * serve): connections that send garbage, half-close, close early or read
 * slowly. Whatever the server does, netstack must keep answering; a call
 * that hangs is cut off by the round timeout, which also kills a process
 * in the middle of a TCP read. */
static int f_net(void *arg)
{
    (void)arg;
    if (!net_target) {
        note("net: no --net HOST:PORT, skipped");
        return 0;
    }
    char host[64];
    snprintf(host, sizeof host, "%s", net_target);
    char *colon = strrchr(host, ':');
    if (!colon)
        return 2;
    *colon = 0;
    struct sockaddr_in sa;
    memset(&sa, 0, sizeof sa);
    sa.sin_family = AF_INET;
    sa.sin_port = htons((uint16_t)atoi(colon + 1));
    if (inet_pton(AF_INET, host, &sa.sin_addr) != 1)
        return 2;
    static char buf[16384];
    unsigned conns = 5 + rnd_below(25), ok = 0, refused = 0, errors = 0;
    unsigned long got = 0;
    for (unsigned c = 0; c < conns; c++) {
        int s = socket(AF_INET, SOCK_STREAM, 0);
        if (s < 0) {
            note("net: socket: %s", strerror(errno));
            return 3;
        }
        if (connect(s, (struct sockaddr *)&sa, sizeof sa) < 0) {
            refused++;
            close(s);
            continue;
        }
        unsigned how = rnd_below(6);
        int len;
        if (how == 0) {
            /* garbage request */
            len = 1 + (int)rnd_below(sizeof buf);
            for (int i = 0; i < len; i++)
                buf[i] = (char)rnd();
        } else {
            len = snprintf(buf, sizeof buf, "GET /chaos/%u HTTP/1.1\r\nHost: %s\r\n\r\n", c, host);
        }
        if (write(s, buf, (size_t)len) < 0)
            errors++;
        if (how == 1)
            shutdown(s, SHUT_WR);
        if (how == 2) {
            /* close at once, with data unread */
            close(s);
            continue;
        }
        unsigned limit = how == 3 ? 1 + rnd_below(4096) : sizeof buf;
        ssize_t r;
        unsigned long this_conn = 0;
        while ((r = read(s, buf, limit)) > 0) {
            this_conn += (unsigned long)r;
            if (how == 4)
                sleep_ms(rnd_below(50));
            if (this_conn > 64UL * CHUNK)
                break;
        }
        if (r < 0)
            errors++;
        else
            ok++;
        got += this_conn;
        close(s);
    }
    note("net: %u connections to %s, %u closed cleanly, %u refused, %u errors, %lu bytes",
         conns, net_target, ok, refused, errors, got);
    return 0;
}

/* Files a fault leaves behind when it was killed: a full data partition
 * would fail every later round. */
static void cleanup_files(void)
{
    const char *dirs[] = { "/tmp", data_dir };
    for (int i = 0; i < 2; i++) {
        DIR *d = opendir(dirs[i]);
        if (!d)
            continue;
        struct dirent *e;
        char path[512];
        while ((e = readdir(d))) {
            if (strncmp(e->d_name, "chaos-", 6) != 0)
                continue;
            snprintf(path, sizeof path, "%s/%s", dirs[i], e->d_name);
            if (unlink(path) == 0)
                note("removed %s, left behind", path);
        }
        closedir(d);
    }
}

/* ---- rounds ------------------------------------------------------------- */

struct fault {
    const char *name;
    int (*fn)(void *);
    const char *what;
};

static const struct fault faults[] = {
    { "memory",  f_memory,  "allocate until malloc fails or --mem-reserve is reached" },
    { "procs",   f_procs,   "waves of fork + exit, some nested" },
    { "threads", f_threads, "waves of threads on one mutex" },
    { "signals", f_signals, "signal storm from children during pipe I/O" },
    { "fds",     f_fds,     "open, pipe and dup until EMFILE" },
    { "schemes", f_schemes, "random and invalid calls on /scheme/*" },
    { "fill",    f_fill,    "fill the data partition until ENOSPC" },
    { "killio",  f_killio,  "SIGKILL processes in the middle of I/O" },
    { "net",     f_net,     "TCP client against --net HOST:PORT (only with --net)" },
};
#define N_FAULTS (sizeof faults / sizeof *faults)

static int enabled[N_FAULTS];
static int kinds_all = 1;          /* no --kind given */

static int parse_kinds(const char *s)
{
    memset(enabled, 0, sizeof enabled);
    char tmp[256];
    snprintf(tmp, sizeof tmp, "%s", s);
    for (char *tok = strtok(tmp, ","); tok; tok = strtok(NULL, ",")) {
        if (strcmp(tok, "all") == 0) {
            for (size_t i = 0; i < N_FAULTS; i++)
                enabled[i] = strcmp(faults[i].name, "net") != 0;
            continue;
        }
        size_t i;
        for (i = 0; i < N_FAULTS; i++)
            if (strcmp(tok, faults[i].name) == 0)
                break;
        if (i == N_FAULTS) {
            fprintf(stderr, "chaos: unknown kind %s (see --list)\n", tok);
            return -1;
        }
        enabled[i] = 1;
    }
    return 0;
}

static const char *result_text(int r, char *buf, size_t len)
{
    if (r == 0)
        return "ok";
    if (r == -1)
        snprintf(buf, len, "timeout, killed");
    else if (r == -2)
        snprintf(buf, len, "UNKILLABLE child");
    else if (r == -3)
        snprintf(buf, len, "fork failed");
    else if (r > 128)
        snprintf(buf, len, "killed by signal %d", r - 128);
    else
        snprintf(buf, len, "exit %d", r);
    return buf;
}

static void usage(void)
{
    fprintf(stderr,
            "usage: chaos [--rounds N] [--seed S] [--kind K[,K]...] [--round-seed S]\n"
            "             [--timeout SEC] [--mem-reserve MB] [--fill-max MB]\n"
            "             [--data DIR] [--self PATH] [--net HOST:PORT] [--list] [--quiet]\n");
    exit(2);
}

int main(int argc, char **argv)
{
    master_seed = (uint64_t)time(NULL) ^ ((uint64_t)getpid() << 32);
    parse_kinds("all");
    for (int i = 1; i < argc; i++) {
        const char *a = argv[i];
        const char *v = i + 1 < argc ? argv[i + 1] : NULL;
        if (strcmp(a, "--noop") == 0)
            return 0;
        if (strcmp(a, "--list") == 0) {
            for (size_t k = 0; k < N_FAULTS; k++)
                printf("%-8s %s\n", faults[k].name, faults[k].what);
            return 0;
        }
        if (strcmp(a, "--quiet") == 0) {
            quiet = 1;
            continue;
        }
        if (!v)
            usage();
        if (strcmp(a, "--rounds") == 0)
            rounds = (unsigned)strtoul(v, NULL, 0);
        else if (strcmp(a, "--seed") == 0)
            master_seed = strtoull(v, NULL, 0);
        else if (strcmp(a, "--round-seed") == 0)
            have_round_seed = 1, round_seed_opt = strtoull(v, NULL, 0);
        else if (strcmp(a, "--kind") == 0) {
            if (parse_kinds(v) < 0)
                return 2;
            kinds_all = 0;
        } else if (strcmp(a, "--timeout") == 0)
            timeout_s = (unsigned)strtoul(v, NULL, 0);
        else if (strcmp(a, "--mem-reserve") == 0)
            mem_reserve_mb = (unsigned)strtoul(v, NULL, 0);
        else if (strcmp(a, "--fill-max") == 0)
            fill_max_mb = (unsigned)strtoul(v, NULL, 0);
        else if (strcmp(a, "--data") == 0)
            data_dir = v;
        else if (strcmp(a, "--self") == 0)
            self_path = v;
        else if (strcmp(a, "--net") == 0)
            net_target = v;
        else
            usage();
        i++;
    }

    if (net_target && kinds_all)
        enabled[N_FAULTS - 1] = 1;
    unsigned kinds[N_FAULTS], n_kinds = 0;
    for (size_t k = 0; k < N_FAULTS; k++)
        if (enabled[k])
            kinds[n_kinds++] = (unsigned)k;
    if (!n_kinds)
        usage();
    if (have_round_seed)
        rounds = 1;

    say("start seed=0x%llx rounds=%u timeout=%us mem-reserve=%uMiB free-mem=%lluMiB",
        (unsigned long long)master_seed, rounds, timeout_s, mem_reserve_mb,
        (unsigned long long)(free_memory() >> 20));

    if (health() != 0) {
        say("health check fails before the first round, stopping");
        return 1;
    }

    seed_rng(master_seed);
    unsigned failed = 0;
    uint64_t t0 = now_ms();
    for (unsigned r = 1; r <= rounds; r++) {
        uint64_t rs = have_round_seed ? round_seed_opt : rnd();
        uint64_t saved = rng_state;
        /* the kind comes from the round seed, so --round-seed alone repeats it */
        seed_rng(rs);
        const struct fault *f = &faults[kinds[rnd_below(n_kinds)]];
        say("round %u/%u kind=%s seed=0x%llx", r, rounds, f->name, (unsigned long long)rs);
        uint64_t start = now_ms();
        int res = run_child(f->fn, NULL, timeout_s * 1000);
        char why[64];
        if (res != 0)
            cleanup_files();
        int hres = health();
        uint64_t ms = now_ms() - start;
        if (res == 0 && hres == 0) {
            say("round %u ok (%llu ms)", r, (unsigned long long)ms);
        } else {
            failed++;
            say("round %u FAIL kind=%s: %s%s (%llu ms); repeat: chaos --kind %s --round-seed 0x%llx",
                r, f->name, result_text(res, why, sizeof why), hres ? ", health check failed" : "",
                (unsigned long long)ms, f->name, (unsigned long long)rs);
            if (res == -2 || hres) {
                /* the system may be in a bad state; give it a moment */
                sleep_ms(2000);
            }
        }
        rng_state = saved;
    }
    say("done: %u rounds, %u failed, %llu s, free-mem=%lluMiB", rounds, failed,
        (unsigned long long)((now_ms() - t0) / 1000), (unsigned long long)(free_memory() >> 20));
    return failed ? 1 : 0;
}
