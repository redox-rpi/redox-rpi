#!/usr/bin/tcc -run -DGREETING="Hallo"
/* A C file run as a script: chmod +x hallo.c; echo hi | ./hallo.c a b
 * Prints its arguments and the bytes read from stdin, exits with 42.
 * The second shebang option shows that options after -run reach tcc. */
#include <stdio.h>

int main(int argc, char **argv)
{
    int n = 0;

    for (int i = 0; i < argc; i++)
        printf("argv[%d]=%s\n", i, argv[i]);
    while (getchar() != EOF)
        n++;
    printf("%s, stdin %d bytes\n", GREETING, n);
    return 42;
}
