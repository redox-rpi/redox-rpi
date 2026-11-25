/*
 * vtrun: run a program on a virtual terminal of its own (redox-rpi#728).
 *
 * Opening /scheme/input/consumer makes inputd create a new VT; vtrun
 * activates it, so the display shows what is set on that VT, and runs the
 * program. The program finds the VT's display node (for example
 * /scheme/display.vc4/4) in place of an argument "{}" and in the
 * environment as DRM_DISPLAY (VT holds the number). When the program ends,
 * or vtrun itself dies, the consumer handle closes and inputd switches to
 * another VT (the console).
 *
 * Usage: vtrun program [args...]
 * Example: vtrun modetest -D {} -s 27:640x480
 */
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>

/* relibc */
ssize_t redox_fpath(int fd, void *buf, size_t count);

/* inputd's ControlEvent: kind 1 activates the VT in data. */
struct control_event {
	size_t kind;
	size_t data;
};

int main(int argc, char **argv)
{
	if (argc < 2) {
		fprintf(stderr, "usage: vtrun program [args...] (\"{}\" is the VT's display node)\n");
		return 2;
	}

	int consumer = open("/scheme/input/consumer", O_RDONLY | O_NONBLOCK | O_CLOEXEC);
	if (consumer < 0) {
		perror("vtrun: open /scheme/input/consumer");
		return 1;
	}
	char node[256];
	ssize_t n = redox_fpath(consumer, node, sizeof(node) - 1);
	if (n <= 0) {
		perror("vtrun: fpath of the consumer handle");
		return 1;
	}
	node[n] = '\0';
	const char *slash = strrchr(node, '/');
	long vt = slash ? strtol(slash + 1, NULL, 10) : 0;
	if (vt <= 0) {
		fprintf(stderr, "vtrun: no VT in '%s'\n", node);
		return 1;
	}

	int control = open("/scheme/input/control", O_WRONLY | O_CLOEXEC);
	struct control_event ev = { 1, (size_t)vt };
	if (control < 0 || write(control, &ev, sizeof(ev)) != (ssize_t)sizeof(ev)) {
		perror("vtrun: activate the VT");
		return 1;
	}
	close(control);
	fprintf(stderr, "vtrun: VT %ld active, display node %s\n", vt, node);

	char vtnum[24];
	snprintf(vtnum, sizeof(vtnum), "%ld", vt);
	setenv("VT", vtnum, 1);
	setenv("DRM_DISPLAY", node, 1);
	for (int i = 1; i < argc; i++)
		if (strcmp(argv[i], "{}") == 0)
			argv[i] = node;

	pid_t pid = fork();
	if (pid < 0) {
		perror("vtrun: fork");
		return 1;
	}
	if (pid == 0) {
		execvp(argv[1], argv + 1);
		perror("vtrun: exec");
		_exit(127);
	}
	int status = 0;
	while (waitpid(pid, &status, 0) < 0 && errno == EINTR)
		;
	/* Closing the handle gives the display back. */
	close(consumer);
	if (WIFEXITED(status)) {
		fprintf(stderr, "vtrun: VT %ld closed, program exit %d\n", vt, WEXITSTATUS(status));
		return WEXITSTATUS(status);
	}
	fprintf(stderr, "vtrun: VT %ld closed, program ended by signal %d\n", vt,
		WIFSIGNALED(status) ? WTERMSIG(status) : 0);
	return 128 + (WIFSIGNALED(status) ? WTERMSIG(status) : 0);
}
