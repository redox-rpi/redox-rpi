/* CPU benchmark for comparing tcc's code with gcc -O2 on the Pi:
 * recursive calls, integer loops over memory, floating point.
 * Prints the time of each part in milliseconds. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

static double now_ms(void)
{
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec * 1000.0 + ts.tv_nsec / 1e6;
}

static int fib(int n)
{
    return n < 2 ? n : fib(n - 1) + fib(n - 2);
}

static int sieve(char *flags, int n)
{
    int count = 0;
    memset(flags, 1, n);
    for (int i = 2; i < n; i++) {
        if (flags[i]) {
            count++;
            for (int j = 2 * i; j < n; j += i)
                flags[j] = 0;
        }
    }
    return count;
}

static int mandel(int size, int iters)
{
    int inside = 0;
    for (int y = 0; y < size; y++) {
        for (int x = 0; x < size; x++) {
            double cr = 2.5 * x / size - 2.0, ci = 2.5 * y / size - 1.25;
            double zr = 0, zi = 0;
            int i;
            for (i = 0; i < iters && zr * zr + zi * zi < 4.0; i++) {
                double t = zr * zr - zi * zi + cr;
                zi = 2 * zr * zi + ci;
                zr = t;
            }
            inside += i == iters;
        }
    }
    return inside;
}

int main(void)
{
    double t0, total = now_ms();
    int n = 2000000;
    char *flags = malloc(n);

    t0 = now_ms();
    int f = fib(27);
    printf("fib(27)    = %d  %8.1f ms\n", f, now_ms() - t0);

    t0 = now_ms();
    int p = 0;
    for (int r = 0; r < 5; r++)
        p = sieve(flags, n);
    printf("sieve 5x2M = %d  %8.1f ms\n", p, now_ms() - t0);

    t0 = now_ms();
    int m = mandel(200, 200);
    printf("mandel 200 = %d  %8.1f ms\n", m, now_ms() - t0);

    printf("total             %8.1f ms\n", now_ms() - total);
    free(flags);
    return 0;
}
