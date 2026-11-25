/* gles-demo (#669): a rotating cube in OpenGL ES 2.0, drawn off-screen
 * through EGL's surfaceless platform (Mesa's v3d driver with v3dd, or
 * softpipe) and copied into an Orbital window each frame.
 *
 * The cube's angle depends only on the frame number, so a frame looks the
 * same on every renderer, apart from edge pixels and rounding in the
 * shading. --dump writes the first and the last frame as PPM files and
 * --compare counts the pixels where two such files differ, for a check of
 * the V3D against the software renderer (LIBGL_ALWAYS_SOFTWARE=1).
 *
 *   gles-demo [--frames N] [--window] [--dump PREFIX]
 *       N frames (default 100; 0 with --window: until the window is
 *       closed). --dump writes PREFIX-first.ppm and PREFIX-last.ppm.
 *   gles-demo --compare A.ppm B.ppm
 *       PASS if at most 1 % of the pixels differ by more than 8 in a
 *       channel.
 *
 * Each frame is checked: the corners show the background, the centre the
 * cube, and the cube covers 10 to 60 % of the picture. Prints
 * "gles-demo: PASS: ..." or "gles-demo: FAIL: ..." and exits 0 or 1.
 */

#include <EGL/egl.h>
#include <EGL/eglext.h>
#include <GLES2/gl2.h>
#include <math.h>
#include <orbital.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#define SIZE 256
#define BG_R 16
#define BG_G 24
#define BG_B 48

static int fail(const char *what)
{
	printf("gles-demo: FAIL: %s (EGL error 0x%x, GL error 0x%x)\n", what,
	       eglGetError(), glGetError());
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
		printf("gles-demo: shader: %s\n", log);
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

static int write_ppm(const char *path, const uint8_t *px)
{
	FILE *f = fopen(path, "wb");
	if (!f)
		return -1;
	fprintf(f, "P6\n%d %d\n255\n", SIZE, SIZE);
	for (int y = SIZE - 1; y >= 0; y--) /* GL row 0 is the bottom */
		for (int x = 0; x < SIZE; x++)
			fwrite(&px[(y * SIZE + x) * 4], 1, 3, f);
	return fclose(f);
}

static uint8_t *read_ppm(const char *path)
{
	FILE *f = fopen(path, "rb");
	int w = 0, h = 0, max = 0;
	if (!f)
		return NULL;
	uint8_t *data = NULL;
	if (fscanf(f, "P6 %d %d %d", &w, &h, &max) == 3 && w == SIZE && h == SIZE && max == 255 &&
	    fgetc(f) != EOF) {
		data = malloc(SIZE * SIZE * 3);
		if (data && fread(data, 1, SIZE * SIZE * 3, f) != SIZE * SIZE * 3) {
			free(data);
			data = NULL;
		}
	}
	fclose(f);
	return data;
}

static int compare(const char *pa, const char *pb)
{
	uint8_t *a = read_ppm(pa), *b = read_ppm(pb);
	if (!a || !b) {
		printf("gles-demo: FAIL: compare: cannot read %s as a %dx%d PPM\n", a ? pb : pa, SIZE, SIZE);
		return 1;
	}
	int differ = 0, maxd = 0;
	for (int i = 0; i < SIZE * SIZE; i++) {
		int d = 0;
		for (int c = 0; c < 3; c++) {
			int x = abs(a[i * 3 + c] - b[i * 3 + c]);
			if (x > d)
				d = x;
		}
		if (d > maxd)
			maxd = d;
		if (d > 8)
			differ++;
	}
	int ok = differ * 100 <= SIZE * SIZE;
	printf("gles-demo: %s: compare %s %s: %d of %d pixels differ by more than 8 (%.2f %%), largest difference %d\n",
	       ok ? "PASS" : "FAIL", pa, pb, differ, SIZE * SIZE, differ * 100.0 / (SIZE * SIZE), maxd);
	return ok ? 0 : 1;
}

/* 6 faces, 2 triangles each: position (3) and normal (3) per vertex. The
 * depth test sorts the faces, so the winding does not matter. */
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

int main(int argc, char **argv)
{
	int frames = -1, window = 0;
	const char *dump = NULL;
	setvbuf(stdout, NULL, _IONBF, 0);
	for (int i = 1; i < argc; i++) {
		if (!strcmp(argv[i], "--compare") && i + 2 < argc)
			return compare(argv[i + 1], argv[i + 2]);
		else if (!strcmp(argv[i], "--frames") && i + 1 < argc)
			frames = atoi(argv[++i]);
		else if (!strcmp(argv[i], "--window"))
			window = 1;
		else if (!strcmp(argv[i], "--dump") && i + 1 < argc)
			dump = argv[++i];
		else {
			printf("usage: gles-demo [--frames N] [--window] [--dump PREFIX] | --compare A.ppm B.ppm\n");
			return 2;
		}
	}
	if (frames < 0 || (frames == 0 && !window))
		frames = 100;

	PFNEGLGETPLATFORMDISPLAYEXTPROC get_display =
		(PFNEGLGETPLATFORMDISPLAYEXTPROC)eglGetProcAddress("eglGetPlatformDisplayEXT");
	if (!get_display)
		return fail("no eglGetPlatformDisplayEXT");
	EGLDisplay dpy = get_display(EGL_PLATFORM_SURFACELESS_MESA, EGL_DEFAULT_DISPLAY, NULL);
	if (dpy == EGL_NO_DISPLAY || !eglInitialize(dpy, NULL, NULL))
		return fail("eglInitialize on the surfaceless platform");
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
	if (ctx == EGL_NO_CONTEXT)
		return fail("eglCreateContext");
	if (!eglMakeCurrent(dpy, EGL_NO_SURFACE, EGL_NO_SURFACE, ctx))
		return fail("eglMakeCurrent without a surface");
	const char *renderer = (const char *)glGetString(GL_RENDERER);
	printf("gles-demo: GL_RENDERER %s\n", renderer);

	GLuint tex, depth, fbo;
	glGenTextures(1, &tex);
	glBindTexture(GL_TEXTURE_2D, tex);
	glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA, SIZE, SIZE, 0, GL_RGBA, GL_UNSIGNED_BYTE, NULL);
	glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST);
	glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);
	glGenRenderbuffers(1, &depth);
	glBindRenderbuffer(GL_RENDERBUFFER, depth);
	glRenderbufferStorage(GL_RENDERBUFFER, GL_DEPTH_COMPONENT16, SIZE, SIZE);
	glGenFramebuffers(1, &fbo);
	glBindFramebuffer(GL_FRAMEBUFFER, fbo);
	glFramebufferTexture2D(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D, tex, 0);
	glFramebufferRenderbuffer(GL_FRAMEBUFFER, GL_DEPTH_ATTACHMENT, GL_RENDERBUFFER, depth);
	if (glCheckFramebufferStatus(GL_FRAMEBUFFER) != GL_FRAMEBUFFER_COMPLETE)
		return fail("framebuffer incomplete");

	/* Each face gets its colour from its normal; light from the viewer. */
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
	glViewport(0, 0, SIZE, SIZE);
	glEnable(GL_DEPTH_TEST);
	glClearColor(BG_R / 255.0f, BG_G / 255.0f, BG_B / 255.0f, 1.0f);

	/* Perspective (fov 45 degrees, near 1, far 10), camera 6 back. */
	float proj[16] = { 0 }, f = 1.0f / tanf(22.5f * (float)M_PI / 180.0f);
	proj[0] = f;
	proj[5] = f;
	proj[10] = -11.0f / 9.0f;
	proj[11] = -1;
	proj[14] = -20.0f / 9.0f;
	float view[16] = { 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, -6, 1 };

	void *win = NULL;
	if (window) {
		win = orb_window_new_flags(-1, -1, SIZE, SIZE, "gles-demo", ORB_WINDOW_ASYNC);
		if (!win || !orb_window_data(win))
			return fail("no Orbital window (is Orbital running?)");
	}

	static uint8_t px[SIZE * SIZE * 4];
	uint32_t hash_first = 0, hash_last = 0;
	double t_gl = 0, t_win = 0, start = now_ms();
	int done = 0, running = 1;
	for (int fr = 0; running && (frames == 0 || fr < frames); fr++) {
		double t0 = now_ms();
		float rx[16], ry[16], rot[16], mvp[16];
		mat_rot(rx, fr * 1.5f + 20.0f, 0);
		mat_rot(ry, fr * 2.0f + 30.0f, 1);
		mat_mul(rot, ry, rx);
		mat_mul(mvp, view, rot);
		mat_mul(mvp, proj, mvp);
		glUniformMatrix4fv(u_mvp, 1, GL_FALSE, mvp);
		glUniformMatrix4fv(u_rot, 1, GL_FALSE, rot);
		glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);
		glDrawArrays(GL_TRIANGLES, 0, 36);
		memset(px, 0x55, sizeof(px));
		glReadPixels(0, 0, SIZE, SIZE, GL_RGBA, GL_UNSIGNED_BYTE, px);
		if (glGetError() != GL_NO_ERROR)
			return fail("drawing or glReadPixels");
		double t1 = now_ms();
		t_gl += t1 - t0;

		int bg = 0;
		for (int i = 0; i < SIZE * SIZE; i++) {
			const uint8_t *p = &px[i * 4];
			if (p[0] == BG_R && p[1] == BG_G && p[2] == BG_B)
				bg++;
		}
		const uint8_t *c[4] = { px, &px[(SIZE - 1) * 4], &px[(SIZE * SIZE - SIZE) * 4],
					&px[(SIZE * SIZE - 1) * 4] };
		const uint8_t *mid = &px[((SIZE / 2) * SIZE + SIZE / 2) * 4];
		int corners = 1;
		for (int k = 0; k < 4; k++)
			corners &= c[k][0] == BG_R && c[k][1] == BG_G && c[k][2] == BG_B;
		int centre = !(mid[0] == BG_R && mid[1] == BG_G && mid[2] == BG_B);
		int cube_px = SIZE * SIZE - bg;
		if (!corners || !centre || cube_px * 100 < 10 * SIZE * SIZE || cube_px * 100 > 60 * SIZE * SIZE) {
			printf("gles-demo: FAIL: frame %d: corners %s, centre %s, cube %d of %d pixels\n", fr,
			       corners ? "right" : "wrong", centre ? "right" : "wrong", cube_px, SIZE * SIZE);
			return 1;
		}
		uint32_t h = 2166136261u;
		for (size_t i = 0; i < sizeof(px); i++)
			h = (h ^ px[i]) * 16777619u;
		if (fr == 0) {
			hash_first = h;
			if (dump) {
				char path[256];
				snprintf(path, sizeof(path), "%s-first.ppm", dump);
				if (write_ppm(path, px))
					return fail("cannot write the first frame");
			}
		}
		hash_last = h;

		if (win) {
			uint32_t *dst = orb_window_data(win);
			for (int y = 0; y < SIZE; y++) {
				const uint8_t *s = &px[(SIZE - 1 - y) * SIZE * 4];
				uint32_t *d = &dst[y * SIZE];
				for (int x = 0; x < SIZE; x++, s += 4)
					d[x] = 0xFF000000u | (uint32_t)s[0] << 16 | (uint32_t)s[1] << 8 | s[2];
			}
			orb_window_sync(win);
			void *it = orb_window_events(win);
			for (OrbEventOption e = orb_events_next(it); e.tag != OrbEventOption_None;
			     e = orb_events_next(it))
				if (e.tag == OrbEventOption_Quit)
					running = 0;
			orb_events_destroy(it);
			t_win += now_ms() - t1;
		}
		done++;
	}
	double ms = now_ms() - start;
	if (dump) {
		char path[256];
		snprintf(path, sizeof(path), "%s-last.ppm", dump);
		if (write_ppm(path, px))
			return fail("cannot write the last frame");
	}
	printf("gles-demo: PASS: %s, %d frames right, window %s, first frame hash %08x, last %08x, "
	       "%.2f ms per frame (GL %.2f, window %.2f)\n",
	       renderer, done, win ? "yes" : "no", hash_first, hash_last, ms / done, t_gl / done,
	       t_win / done);
	if (win)
		orb_window_destroy(win);
	eglMakeCurrent(dpy, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT);
	eglDestroyContext(dpy, ctx);
	eglTerminate(dpy);
	return 0;
}
