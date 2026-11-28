/* Boot-time probe: the "app" of a minimal Redox image (#47 timing). */
#include <stdio.h>
#include <time.h>
#include <unistd.h>

int main(void) {
	struct timespec ts;
	clock_gettime(CLOCK_MONOTONIC, &ts);
	printf("APP READY monotonic %ld.%03ld s\n", (long)ts.tv_sec, ts.tv_nsec / 1000000);
	fflush(stdout);
	for (;;)
		sleep(3600);
}
