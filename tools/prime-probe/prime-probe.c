/* prime-probe (#670, G4 step 2 spike): the GPU renders straight into a
 * buffer of the display driver, with no copy by the CPU.
 *
 * The program opens a VT of its own on the display (through inputd, as
 * Orbital does), creates two dumb buffers at the screen's mode and exports
 * each as a dma-buf fd (PRIME_HANDLE_TO_FD). EGL imports the fds
 * (EGL_LINUX_DMA_BUF_EXT, linear): Mesa's v3d driver hands each fd to v3dd
 * (PRIME_FD_TO_HANDLE), which maps the buffer's pages into the client's GPU
 * address space. The GPU then draws a rotating cube into the buffers, and
 * SET_CRTC shows them in turn (double buffering, no vsync).
 *
 * Checks: before the first frame the CPU fills the buffer with a pattern;
 * after it, the CPU's view of the buffer (MAP_DUMB, the display driver's
 * memory) must match glReadPixels pixel for pixel and hold no pattern
 * pixel. Every frame: corners background, centre cube.
 *
 *   prime-probe [--frames N] [--show] [--lifetime] [--foreign]
 *       N frames (default 300). --show switches to the VT, so the frames
 *       appear on the screen; the VT closes at the end and inputd switches
 *       back. --lifetime destroys the dumb buffers and their framebuffers
 *       while the GPU still has them imported and draws one more frame into
 *       each (the export must keep the memory alive). --foreign offers v3dd
 *       two fds that are no display buffers, a regular file's and the
 *       display handle's own; v3dd must refuse both (FAIL otherwise).
 *
 * Prints "prime-probe: PASS: ..." or "prime-probe: FAIL: ..." and exits 0
 * or 1.
 */

#include <EGL/egl.h>
#include <EGL/eglext.h>
#include <GLES2/gl2.h>
#include <GLES2/gl2ext.h>
#include <drm_fourcc.h>
#include <errno.h>
#include <fcntl.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <time.h>
#include <unistd.h>
#include <xf86drm.h>
#include <xf86drmMode.h>

#define BG_R 16
#define BG_G 24
#define BG_B 48
#define BG_XRGB (0x00000000u | BG_R << 16 | BG_G << 8 | BG_B)
#define PATTERN 0x55aa55aau
#define NBUF 2

extern ssize_t redox_fpath(int fd, void *buf, size_t count);

static int fail(const char *what)
{
	int err = errno;
	/* eglGetError first: every EGL call resets it. glGetError only with a
	 * current context: without one the call can go through an empty
	 * dispatch table. */
	EGLint egl = eglGetError();
	GLenum gl = eglGetCurrentContext() != EGL_NO_CONTEXT ? glGetError() : 0;
	printf("prime-probe: FAIL: %s (errno %d, EGL error 0x%x, GL error 0x%x)\n", what, err, egl, gl);
	return 1;
}

static double now_ms(void)
{
	struct timespec ts;
	clock_gettime(CLOCK_MONOTONIC, &ts);
	return ts.tv_sec * 1e3 + ts.tv_nsec / 1e6;
}

static GLuint shader(GLenum type, const char *src)
{
	GLuint s = glCreateShader(type);
	GLint ok = 0;
	glShaderSource(s, 1, &src, NULL);
	glCompileShader(s);
	glGetShaderiv(s, GL_COMPILE_STATUS, &ok);
	if (!ok) {
		char log[512] = "";
		glGetShaderInfoLog(s, sizeof(log), NULL, log);
		printf("prime-probe: shader: %s\n", log);
		return 0;
	}
	return s;
}

/* Column-major 4x4 matrices. */
static void mat_mul(float *out, const float *a, const float *b)
{
	float r[16];
	for (int c = 0; c < 4; c++)
		for (int i = 0; i < 4; i++) {
			r[c * 4 + i] = 0;
			for (int k = 0; k < 4; k++)
				r[c * 4 + i] += a[k * 4 + i] * b[c * 4 + k];
		}
	memcpy(out, r, sizeof(r));
}

static void mat_rot(float *m, float deg, int axis)
{
	float a = deg * (float)M_PI / 180.0f, c = cosf(a), s = sinf(a);
	memset(m, 0, 16 * sizeof(float));
	m[0] = m[5] = m[10] = m[15] = 1;
	int i = (axis + 1) % 3, j = (axis + 2) % 3;
	m[i * 4 + i] = c;
	m[j * 4 + j] = c;
	m[i * 4 + j] = s;
	m[j * 4 + i] = -s;
}

static void cube(GLfloat *v)
{
	static const float corner[6][2] = { { -1, -1 }, { 1, -1 }, { 1, 1 },
					    { -1, -1 }, { 1, 1 }, { -1, 1 } };
	for (int f = 0; f < 6; f++) {
		int n = f / 2, u = (n + 1) % 3, w = (n + 2) % 3;
		float sign = (f % 2) ? -1.0f : 1.0f;
		for (int k = 0; k < 6; k++) {
			GLfloat *p = &v[(f * 6 + k) * 6];
			p[n] = sign;
			p[u] = corner[k][0];
			p[w] = corner[k][1];
			p[3] = p[4] = p[5] = 0;
			p[3 + n] = sign;
		}
	}
}

struct buf {
	uint32_t handle, pitch, fb;
	uint64_t size;
	int dmafd;
	uint32_t *cpu;
	EGLImageKHR image;
	GLuint color, depth, fbo;
};

/* Where a buffer holds pixels the GPU did not draw: the pattern the CPU
 * wrote, or zero (never written). Prints counts, the rows and the number
 * of 4 KiB pages concerned, to tell cache lines from whole pages. */
static void stale_report(const char *what, const uint32_t *px, uint32_t pitch, int w, int h)
{
	uint32_t stride = pitch / 4;
	int pattern = 0, zero = 0, bg = 0, first = -1, last = -1, pages = 0;
	size_t last_page = (size_t)-1;
	for (int y = 0; y < h; y++)
		for (int x = 0; x < w; x++) {
			uint32_t v = px[y * stride + x];
			int stale = v == PATTERN || v == 0;
			pattern += v == PATTERN;
			zero += v == 0;
			bg += (v & 0xffffff) == BG_XRGB;
			if (stale) {
				size_t page = ((size_t)y * pitch + (size_t)x * 4) / 4096;
				if (page != last_page) {
					pages++;
					last_page = page;
				}
				if (first < 0)
					first = y;
				last = y;
			}
		}
	printf("prime-probe: %s: %d pattern, %d zero, %d background pixels of %d; stale rows %d to %d, in %d pages of %llu\n",
	       what, pattern, zero, bg, w * h, first, last, pages,
	       (unsigned long long)(((size_t)pitch * h + 4095) / 4096));
}

/* The VT number is the last number in the consumer handle's path. */
static int path_number(int fd)
{
	char path[256];
	ssize_t n = redox_fpath(fd, path, sizeof(path) - 1);
	if (n <= 0)
		return -1;
	path[n] = 0;
	char *p = path + n;
	while (p > path && p[-1] >= '0' && p[-1] <= '9')
		p--;
	return *p ? atoi(p) : -1;
}

/* Offers v3dd an fd that is no display buffer; 0 if v3dd refuses it. */
static int foreign_refused(int render, int fd, const char *what)
{
	uint32_t handle = 0;
	if (drmPrimeFDToHandle(render, fd, &handle) == 0) {
		printf("prime-probe: foreign: v3dd ACCEPTED %s as BO %u\n", what, handle);
		struct drm_gem_close gc = { .handle = handle };
		drmIoctl(render, DRM_IOCTL_GEM_CLOSE, &gc);
		return -1;
	}
	printf("prime-probe: foreign: v3dd refused %s (errno %d)\n", what, errno);
	return 0;
}

static int foreign_test(int dfd)
{
	char path[] = "/tmp/prime-probe-XXXXXX";
	int f = mkstemp(path);
	if (f < 0)
		return fail("--foreign: mkstemp");
	unlink(path);
	static char page[4096];
	memset(page, 0x5a, sizeof(page));
	if (write(f, page, sizeof(page)) != sizeof(page))
		return fail("--foreign: write");
	int render = open("/dev/dri/renderD128", O_RDWR | O_CLOEXEC);
	if (render < 0)
		return fail("--foreign: open renderD128");
	int bad = foreign_refused(render, f, "a regular file's fd");
	bad |= foreign_refused(render, dfd, "the display handle (no buffer)");
	close(render);
	close(f);
	if (bad) {
		printf("prime-probe: FAIL: v3dd imported an fd that is no display buffer\n");
		return 1;
	}
	return 0;
}

int main(int argc, char **argv)
{
	int frames = 300, show = 0, lifetime = 0, foreign = 0, pattern_fill = 1;
	setvbuf(stdout, NULL, _IONBF, 0);
	for (int i = 1; i < argc; i++) {
		if (!strcmp(argv[i], "--frames") && i + 1 < argc)
			frames = atoi(argv[++i]);
		else if (!strcmp(argv[i], "--show"))
			show = 1;
		else if (!strcmp(argv[i], "--lifetime"))
			lifetime = 1;
		else if (!strcmp(argv[i], "--foreign"))
			foreign = 1;
		else if (!strcmp(argv[i], "--no-pattern"))
			pattern_fill = 0;
		else {
			printf("usage: prime-probe [--frames N] [--show] [--lifetime] [--foreign] [--no-pattern]\n");
			return 2;
		}
	}
	if (frames < 1)
		frames = 1;

	/* A VT of our own, and the display's handle for it. */
	int consumer = open("/scheme/input/consumer", O_RDWR | O_CLOEXEC);
	if (consumer < 0)
		return fail("open /scheme/input/consumer");
	int vt = path_number(consumer);
	int dfd = openat(consumer, "display", O_RDWR | O_CLOEXEC);
	if (dfd < 0)
		return fail("open the display through the consumer handle");
	printf("prime-probe: VT %d\n", vt);

	drmModeRes *res = drmModeGetResources(dfd);
	if (!res || res->count_connectors < 1 || res->count_crtcs < 1)
		return fail("drmModeGetResources");
	drmModeConnector *conn = NULL;
	for (int i = 0; i < res->count_connectors && !conn; i++) {
		drmModeConnector *c = drmModeGetConnector(dfd, res->connectors[i]);
		if (c && c->connection == DRM_MODE_CONNECTED && c->count_modes > 0)
			conn = c;
		else if (c)
			drmModeFreeConnector(c);
	}
	if (!conn)
		return fail("no connected connector with a mode");
	uint32_t crtc_id = res->crtcs[0];
	drmModeModeInfo mode = conn->modes[0];
	drmModeCrtc *crtc = drmModeGetCrtc(dfd, crtc_id);
	if (crtc && crtc->mode_valid)
		mode = crtc->mode;
	int w = mode.hdisplay, h = mode.vdisplay;
	printf("prime-probe: connector %u, CRTC %u, mode %dx%d\n", conn->connector_id, crtc_id, w, h);

	/* EGL first: the extension check needs the display. */
	PFNEGLGETPLATFORMDISPLAYEXTPROC get_display =
		(PFNEGLGETPLATFORMDISPLAYEXTPROC)eglGetProcAddress("eglGetPlatformDisplayEXT");
	if (!get_display)
		return fail("no eglGetPlatformDisplayEXT");
	EGLDisplay dpy = get_display(EGL_PLATFORM_SURFACELESS_MESA, EGL_DEFAULT_DISPLAY, NULL);
	if (dpy == EGL_NO_DISPLAY || !eglInitialize(dpy, NULL, NULL))
		return fail("eglInitialize on the surfaceless platform");
	const char *exts = eglQueryString(dpy, EGL_EXTENSIONS);
	int has_import = exts && strstr(exts, "EGL_EXT_image_dma_buf_import");
	int has_mods = exts && strstr(exts, "EGL_EXT_image_dma_buf_import_modifiers");
	printf("prime-probe: EGL_EXT_image_dma_buf_import %s, _modifiers %s\n", has_import ? "yes" : "no",
	       has_mods ? "yes" : "no");
	if (!has_import)
		return fail("no EGL_EXT_image_dma_buf_import (does v3dd report DRM_CAP_PRIME import?)");
	/* Without a modifier v3d takes an imported buffer as tiled (UIF). */
	if (!has_mods)
		return fail("no EGL_EXT_image_dma_buf_import_modifiers, cannot ask for a linear import");
	if (!eglBindAPI(EGL_OPENGL_ES_API))
		return fail("eglBindAPI");
	static const EGLint config_attribs[] = {
		EGL_SURFACE_TYPE, EGL_PBUFFER_BIT,
		EGL_RENDERABLE_TYPE, EGL_OPENGL_ES2_BIT,
		EGL_RED_SIZE, 8, EGL_GREEN_SIZE, 8, EGL_BLUE_SIZE, 8,
		EGL_NONE,
	};
	EGLConfig config;
	EGLint n = 0;
	if (!eglChooseConfig(dpy, config_attribs, &config, 1, &n) || n < 1)
		return fail("eglChooseConfig");
	static const EGLint context_attribs[] = { EGL_CONTEXT_CLIENT_VERSION, 2, EGL_NONE };
	EGLContext ctx = eglCreateContext(dpy, config, EGL_NO_CONTEXT, context_attribs);
	if (ctx == EGL_NO_CONTEXT || !eglMakeCurrent(dpy, EGL_NO_SURFACE, EGL_NO_SURFACE, ctx))
		return fail("eglCreateContext/eglMakeCurrent");
	const char *renderer = (const char *)glGetString(GL_RENDERER);
	printf("prime-probe: GL_RENDERER %s\n", renderer);
	PFNEGLCREATEIMAGEKHRPROC create_image =
		(PFNEGLCREATEIMAGEKHRPROC)eglGetProcAddress("eglCreateImageKHR");
	PFNEGLDESTROYIMAGEKHRPROC destroy_image =
		(PFNEGLDESTROYIMAGEKHRPROC)eglGetProcAddress("eglDestroyImageKHR");
	PFNGLEGLIMAGETARGETRENDERBUFFERSTORAGEOESPROC image_storage =
		(PFNGLEGLIMAGETARGETRENDERBUFFERSTORAGEOESPROC)eglGetProcAddress(
			"glEGLImageTargetRenderbufferStorageOES");
	if (!create_image || !destroy_image || !image_storage)
		return fail("eglCreateImageKHR or glEGLImageTargetRenderbufferStorageOES missing");

	/* Two dumb buffers of the display driver, each exported and imported. */
	struct buf b[NBUF];
	double t_import = now_ms();
	for (int i = 0; i < NBUF; i++) {
		struct drm_mode_create_dumb cd = { .width = w, .height = h, .bpp = 32 };
		if (drmIoctl(dfd, DRM_IOCTL_MODE_CREATE_DUMB, &cd))
			return fail("CREATE_DUMB");
		b[i].handle = cd.handle;
		b[i].pitch = cd.pitch;
		b[i].size = cd.size;
		uint32_t handles[4] = { cd.handle }, pitches[4] = { cd.pitch }, offsets[4] = { 0 };
		if (drmModeAddFB2(dfd, w, h, DRM_FORMAT_XRGB8888, handles, pitches, offsets, &b[i].fb, 0))
			return fail("ADD_FB2");
		struct drm_mode_map_dumb md = { .handle = cd.handle };
		if (drmIoctl(dfd, DRM_IOCTL_MODE_MAP_DUMB, &md))
			return fail("MAP_DUMB");
		b[i].cpu = mmap(NULL, cd.size, PROT_READ | PROT_WRITE, MAP_SHARED, dfd, md.offset);
		if (b[i].cpu == MAP_FAILED)
			return fail("mmap of the dumb buffer");
		if (drmPrimeHandleToFD(dfd, cd.handle, DRM_CLOEXEC | DRM_RDWR, &b[i].dmafd))
			return fail("PRIME_HANDLE_TO_FD");
		if (i == 0) {
			/* The import Mesa does, by hand first, for a clear errno. */
			int render = open("/dev/dri/renderD128", O_RDWR | O_CLOEXEC);
			uint32_t h = 0;
			if (render < 0)
				return fail("open /dev/dri/renderD128");
			if (drmPrimeFDToHandle(render, b[i].dmafd, &h))
				return fail("PRIME_FD_TO_HANDLE of vc4d's buffer on the render node");
			printf("prime-probe: direct import: vc4d's buffer is BO %u on the render node\n", h);
			struct drm_gem_close gc = { .handle = h };
			drmIoctl(render, DRM_IOCTL_GEM_CLOSE, &gc);
			close(render);
		}
		EGLint attrs[] = {
			EGL_WIDTH, w,
			EGL_HEIGHT, h,
			EGL_LINUX_DRM_FOURCC_EXT, DRM_FORMAT_XRGB8888,
			EGL_DMA_BUF_PLANE0_FD_EXT, b[i].dmafd,
			EGL_DMA_BUF_PLANE0_OFFSET_EXT, 0,
			EGL_DMA_BUF_PLANE0_PITCH_EXT, (EGLint)cd.pitch,
			EGL_DMA_BUF_PLANE0_MODIFIER_LO_EXT, (EGLint)(DRM_FORMAT_MOD_LINEAR & 0xffffffff),
			EGL_DMA_BUF_PLANE0_MODIFIER_HI_EXT, (EGLint)(DRM_FORMAT_MOD_LINEAR >> 32),
			EGL_NONE,
		};
		b[i].image = create_image(dpy, EGL_NO_CONTEXT, EGL_LINUX_DMA_BUF_EXT, NULL, attrs);
		if (b[i].image == EGL_NO_IMAGE_KHR)
			return fail("eglCreateImageKHR(EGL_LINUX_DMA_BUF_EXT)");
		glGenRenderbuffers(1, &b[i].color);
		glBindRenderbuffer(GL_RENDERBUFFER, b[i].color);
		image_storage(GL_RENDERBUFFER, b[i].image);
		glGenRenderbuffers(1, &b[i].depth);
		glBindRenderbuffer(GL_RENDERBUFFER, b[i].depth);
		glRenderbufferStorage(GL_RENDERBUFFER, GL_DEPTH_COMPONENT16, w, h);
		glGenFramebuffers(1, &b[i].fbo);
		glBindFramebuffer(GL_FRAMEBUFFER, b[i].fbo);
		glFramebufferRenderbuffer(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_RENDERBUFFER, b[i].color);
		glFramebufferRenderbuffer(GL_FRAMEBUFFER, GL_DEPTH_ATTACHMENT, GL_RENDERBUFFER, b[i].depth);
		if (glCheckFramebufferStatus(GL_FRAMEBUFFER) != GL_FRAMEBUFFER_COMPLETE)
			return fail("framebuffer with the imported buffer incomplete");
		printf("prime-probe: buffer %d: handle %u, pitch %u, %llu bytes, fb %u, dma-buf fd %d, imported\n", i,
		       b[i].handle, b[i].pitch, (unsigned long long)b[i].size, b[i].fb, b[i].dmafd);
	}
	t_import = now_ms() - t_import;

	GLuint vs = shader(GL_VERTEX_SHADER,
			   "uniform mat4 mvp;\n"
			   "uniform mat4 rot;\n"
			   "attribute vec3 pos;\n"
			   "attribute vec3 normal;\n"
			   "varying vec3 col;\n"
			   "void main() {\n"
			   "  vec3 base = abs(normal) * 0.7 + 0.3 * vec3(normal.x < 0.0 || normal.y < 0.0 || normal.z < 0.0);\n"
			   "  float light = 0.35 + 0.65 * max((rot * vec4(normal, 0.0)).z, 0.0);\n"
			   "  col = base * light;\n"
			   "  gl_Position = mvp * vec4(pos, 1.0);\n"
			   "}\n");
	GLuint fs = shader(GL_FRAGMENT_SHADER,
			   "precision mediump float;\n"
			   "varying vec3 col;\n"
			   "void main() { gl_FragColor = vec4(col, 1.0); }\n");
	if (!vs || !fs)
		return fail("shader compile");
	GLuint prog = glCreateProgram();
	glAttachShader(prog, vs);
	glAttachShader(prog, fs);
	glBindAttribLocation(prog, 0, "pos");
	glBindAttribLocation(prog, 1, "normal");
	glLinkProgram(prog);
	GLint linked = 0;
	glGetProgramiv(prog, GL_LINK_STATUS, &linked);
	if (!linked)
		return fail("program link");
	glUseProgram(prog);
	GLint u_mvp = glGetUniformLocation(prog, "mvp"), u_rot = glGetUniformLocation(prog, "rot");
	static GLfloat verts[36 * 6];
	cube(verts);
	glVertexAttribPointer(0, 3, GL_FLOAT, GL_FALSE, 6 * sizeof(GLfloat), verts);
	glVertexAttribPointer(1, 3, GL_FLOAT, GL_FALSE, 6 * sizeof(GLfloat), verts + 3);
	glEnableVertexAttribArray(0);
	glEnableVertexAttribArray(1);
	glViewport(0, 0, w, h);
	glEnable(GL_DEPTH_TEST);
	glClearColor(BG_R / 255.0f, BG_G / 255.0f, BG_B / 255.0f, 1.0f);

	/* Perspective (fov 45 degrees, near 1, far 10), camera 6 back. GL's
	 * row 0 is the buffer's first row, the top of the screen: flip y. */
	float proj[16] = { 0 }, f = 1.0f / tanf(22.5f * (float)M_PI / 180.0f);
	proj[0] = f * h / w;
	proj[5] = -f;
	proj[10] = -11.0f / 9.0f;
	proj[11] = -1;
	proj[14] = -20.0f / 9.0f;
	float view[16] = { 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, -6, 1 };

	if (show) {
		int ctl = open("/scheme/input/control", O_WRONLY | O_CLOEXEC);
		size_t ev[2] = { 1 /* VT activate */, (size_t)vt };
		if (vt < 0 || ctl < 0 || write(ctl, ev, sizeof(ev)) != sizeof(ev))
			return fail("activate the VT through /scheme/input/control");
		close(ctl);
	}

	/* Frame 0 into buffer 0: the CPU's view must match GL's. */
	/* --no-pattern: the CPU does not write the buffer before the GPU (a
	 * never-written pixel then reads as zero). */
	if (pattern_fill)
		for (uint64_t i = 0; i < b[0].size / 4; i++)
			b[0].cpu[i] = PATTERN;
	double t_render = 0, t_flip = 0, start = now_ms();
	int done = 0;
	uint32_t hash_first = 0;
	for (int fr = 0; fr < frames; fr++) {
		struct buf *cur = &b[fr % NBUF];
		double t0 = now_ms();
		float rx[16], ry[16], rot[16], mvp[16];
		mat_rot(rx, fr * 1.5f + 20.0f, 0);
		mat_rot(ry, fr * 2.0f + 30.0f, 1);
		mat_mul(rot, ry, rx);
		mat_mul(mvp, view, rot);
		mat_mul(mvp, proj, mvp);
		glBindFramebuffer(GL_FRAMEBUFFER, cur->fbo);
		glUniformMatrix4fv(u_mvp, 1, GL_FALSE, mvp);
		glUniformMatrix4fv(u_rot, 1, GL_FALSE, rot);
		glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);
		glDrawArrays(GL_TRIANGLES, 0, 36);
		glFinish();
		if (glGetError() != GL_NO_ERROR)
			return fail("drawing");
		double t1 = now_ms();
		t_render += t1 - t0;

		const uint32_t *px = cur->cpu;
		uint32_t stride = cur->pitch / 4;
		uint32_t c[4] = { px[0], px[w - 1], px[(h - 1) * stride], px[(h - 1) * stride + w - 1] };
		int corners = 1;
		for (int k = 0; k < 4; k++)
			corners &= (c[k] & 0xffffff) == BG_XRGB;
		int centre = (px[(h / 2) * stride + w / 2] & 0xffffff) != BG_XRGB;
		if (!corners || !centre) {
			stale_report("stale", px, cur->pitch, w, h);
			/* Late GPU writes or lost ones: look again later. */
			usleep(200000);
			stale_report("stale after 200 ms", px, cur->pitch, w, h);
			uint8_t g[4] = { 0 };
			glReadPixels(0, 0, 1, 1, GL_RGBA, GL_UNSIGNED_BYTE, g);
			printf("prime-probe: glReadPixels of the first pixel: %02x%02x%02x%02x, CPU now %08x\n", g[0],
			       g[1], g[2], g[3], px[0]);
			printf("prime-probe: FAIL: frame %d: corners %s (%08x %08x %08x %08x), centre %s\n", fr,
			       corners ? "right" : "wrong", c[0], c[1], c[2], c[3], centre ? "right" : "wrong");
			return 1;
		}
		if (fr == 0) {
			uint8_t *gl = malloc((size_t)w * h * 4);
			if (!gl)
				return fail("malloc");
			glReadPixels(0, 0, w, h, GL_RGBA, GL_UNSIGNED_BYTE, gl);
			int differ = 0, pattern = 0, cube_px = 0;
			uint32_t hsh = 2166136261u;
			for (int y = 0; y < h; y++)
				for (int x = 0; x < w; x++) {
					uint32_t v = px[y * stride + x];
					const uint8_t *g = &gl[((size_t)y * w + x) * 4];
					uint32_t want = (uint32_t)g[0] << 16 | (uint32_t)g[1] << 8 | g[2];
					if (v == PATTERN)
						pattern++;
					if ((v & 0xffffff) != want)
						differ++;
					if ((v & 0xffffff) != BG_XRGB)
						cube_px++;
					hsh = (hsh ^ (v & 0xffffff)) * 16777619u;
				}
			free(gl);
			hash_first = hsh;
			printf("prime-probe: frame 0: %d pattern pixels left, %d pixels differ from glReadPixels, cube %d of %d pixels, hash %08x\n",
			       pattern, differ, cube_px, w * h, hsh);
			if (pattern || differ || (!pattern_fill && cube_px == w * h))
				return fail("the display buffer does not hold what the GPU drew");
			if (cube_px * 100 < 2 * w * h || cube_px * 100 > 60 * w * h)
				return fail("the cube covers less than 2 % or more than 60 % of the screen");
		}
		if (drmModeSetCrtc(dfd, crtc_id, cur->fb, 0, 0, &conn->connector_id, 1, &mode))
			return fail("SET_CRTC");
		t_flip += now_ms() - t1;
		done++;
	}
	double ms = now_ms() - start;
	printf("prime-probe: %d frames at %dx%d, %.2f ms per frame (render+glFinish %.2f, SET_CRTC %.2f), import of %d buffers %.1f ms\n",
	       done, w, h, ms / done, t_render / done, t_flip / done, NBUF, t_import);

	if (lifetime) {
		/* The display forgets the buffers; the GPU still has them. */
		for (int i = 0; i < NBUF; i++) {
			munmap(b[i].cpu, b[i].size);
			drmModeRmFB(dfd, b[i].fb);
			struct drm_mode_destroy_dumb dd = { .handle = b[i].handle };
			if (drmIoctl(dfd, DRM_IOCTL_MODE_DESTROY_DUMB, &dd))
				return fail("DESTROY_DUMB");
			close(b[i].dmafd);
		}
		for (int i = 0; i < NBUF; i++) {
			glBindFramebuffer(GL_FRAMEBUFFER, b[i].fbo);
			glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);
			glDrawArrays(GL_TRIANGLES, 0, 36);
			glFinish();
			uint8_t p[4] = { 0 };
			glReadPixels(w / 2, h / 2, 1, 1, GL_RGBA, GL_UNSIGNED_BYTE, p);
			if (glGetError() != GL_NO_ERROR)
				return fail("--lifetime: drawing after DESTROY_DUMB");
		}
		printf("prime-probe: lifetime: drew into both buffers after RM_FB, DESTROY_DUMB and close of the dma-buf fds\n");
	}
	for (int i = 0; i < NBUF; i++) {
		glDeleteFramebuffers(1, &b[i].fbo);
		glDeleteRenderbuffers(1, &b[i].color);
		glDeleteRenderbuffers(1, &b[i].depth);
		destroy_image(dpy, b[i].image);
	}
	eglMakeCurrent(dpy, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT);
	eglDestroyContext(dpy, ctx);
	eglTerminate(dpy);
	if (foreign && foreign_test(dfd))
		return 1;
	printf("prime-probe: PASS: %s, %dx%d, %d frames into display buffers, frame 0 hash %08x, %.2f ms per frame\n",
	       renderer, w, h, done, hash_first, ms / done);
	close(dfd);
	close(consumer);
	return 0;
}
