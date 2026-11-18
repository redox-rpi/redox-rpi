#include <stdio.h>

int main(int argc, char **argv)
{
    printf("Hello from tcc on Redox (%s)\n", argc > 0 ? argv[0] : "?");
    return 0;
}
