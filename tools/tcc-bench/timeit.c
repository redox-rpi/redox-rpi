/* timeit CMD [ARGS...]: run CMD and print its wall time in milliseconds
 * (Ion has no `time`). */
#include <stdio.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

int main(int argc, char **argv)
{
    struct timespec a, b;
    int status = 0;
    pid_t pid;

    if (argc < 2) {
        fprintf(stderr, "usage: timeit CMD [ARGS...]\n");
        return 2;
    }
    clock_gettime(CLOCK_MONOTONIC, &a);
    pid = fork();
    if (pid == 0) {
        execvp(argv[1], argv + 1);
        perror(argv[1]);
        _exit(127);
    }
    if (pid < 0 || waitpid(pid, &status, 0) < 0) {
        perror("timeit");
        return 1;
    }
    clock_gettime(CLOCK_MONOTONIC, &b);
    fprintf(stderr, "[timeit] %.1f ms, status %d: %s\n",
            (b.tv_sec - a.tv_sec) * 1000.0 + (b.tv_nsec - a.tv_nsec) / 1e6,
            WIFEXITED(status) ? WEXITSTATUS(status) : 128 + WTERMSIG(status),
            argv[1]);
    return WIFEXITED(status) ? WEXITSTATUS(status) : 1;
}
