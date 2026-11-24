/* gles-probe (#669): OpenGL ES 2.0 off-screen through EGL's surfaceless
 * platform, the way Mesa's v3d driver runs on Redox with v3dd.
 *
 * Each frame clears a 64x64 RGBA texture (bound to a framebuffer object)
 * to blue, draws a red triangle over its lower left half, reads the pixels
 * back and checks them: every pixel red or blue, the red pixels exactly the
 * half below the diagonal plus the diagonal, the corners right. The
 * triangle's long edge lies half a pixel beyond the diagonal's pixel
 * centres, so no centre is on an edge and every renderer must give the same
 * pixels, whatever its fill rule. The first frame's pixels are hashed
 * (FNV-1a): the hash must match Mesa's software renderer
 * (LIBGL_ALWAYS_SOFTWARE=1).
 *
 *   gles-probe [frames]       default 100
 *
 * Prints "gles-probe: PASS: ..." or "gles-probe: FAIL: ..." and exits 0 or 1.
 */

#include <EGL/egl.h>
#include <EGL/eglext.h>
#include <GLES2/gl2.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#define SIZE 64

static int fail(const char *what)
{
	printf("gles-probe: FAIL: %s (EGL error 0x%x, GL error 0x%x)\n", what,
	       eglGetError(), glGetError());
	return 1;
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
		printf("gles-probe: shader: %s\n", log);
		return 0;
	}
	return s;
}

static double now_ms(void)
{
	struct timespec ts;
	clock_gettime(CLOCK_MONOTONIC, &ts);
	return ts.tv_sec * 1e3 + ts.tv_nsec / 1e6;
}

int main(int argc, char **argv)
{
	int frames = argc > 1 ? atoi(argv[1]) : 100;
	if (frames < 1)
		frames = 1;
	setvbuf(stdout, NULL, _IONBF, 0);

	PFNEGLGETPLATFORMDISPLAYEXTPROC get_display =
		(PFNEGLGETPLATFORMDISPLAYEXTPROC)eglGetProcAddress("eglGetPlatformDisplayEXT");
	if (!get_display)
		return fail("no eglGetPlatformDisplayEXT");
	EGLDisplay dpy = get_display(EGL_PLATFORM_SURFACELESS_MESA, EGL_DEFAULT_DISPLAY, NULL);
	EGLint major = 0, minor = 0;
	if (dpy == EGL_NO_DISPLAY || !eglInitialize(dpy, &major, &minor))
		return fail("eglInitialize on the surfaceless platform");
	printf("gles-probe: EGL %d.%d, %s\n", major, minor, eglQueryString(dpy, EGL_VENDOR));
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
	printf("gles-probe: GL_RENDERER %s, GL_VERSION %s\n", renderer,
	       (const char *)glGetString(GL_VERSION));

	GLuint tex, fbo;
	glGenTextures(1, &tex);
	glBindTexture(GL_TEXTURE_2D, tex);
	glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA, SIZE, SIZE, 0, GL_RGBA, GL_UNSIGNED_BYTE, NULL);
	glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST);
	glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);
	glGenFramebuffers(1, &fbo);
	glBindFramebuffer(GL_FRAMEBUFFER, fbo);
	glFramebufferTexture2D(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D, tex, 0);
	if (glCheckFramebufferStatus(GL_FRAMEBUFFER) != GL_FRAMEBUFFER_COMPLETE)
		return fail("framebuffer incomplete");

	GLuint vs = shader(GL_VERTEX_SHADER,
			   "attribute vec2 pos;\n"
			   "void main() { gl_Position = vec4(pos, 0.0, 1.0); }\n");
	GLuint fs = shader(GL_FRAGMENT_SHADER,
			   "precision mediump float;\n"
			   "uniform vec4 color;\n"
			   "void main() { gl_FragColor = color; }\n");
	if (!vs || !fs)
		return fail("shader compile");
	GLuint prog = glCreateProgram();
	glAttachShader(prog, vs);
	glAttachShader(prog, fs);
	glBindAttribLocation(prog, 0, "pos");
	glLinkProgram(prog);
	GLint linked = 0;
	glGetProgramiv(prog, GL_LINK_STATUS, &linked);
	if (!linked)
		return fail("program link");
	glUseProgram(prog);
	glUniform4f(glGetUniformLocation(prog, "color"), 1.0f, 0.0f, 0.0f, 1.0f);
	/* The edge x + y = E (normalised coordinates) with E = 1/64 passes half a
	 * pixel beyond the centres on the diagonal x + y = 63 (pixels). */
	static const GLfloat tri[] = { -1, -1, 1 + 1.0f / 64, -1, -1, 1 + 1.0f / 64 };
	glVertexAttribPointer(0, 2, GL_FLOAT, GL_FALSE, 0, tri);
	glEnableVertexAttribArray(0);
	glViewport(0, 0, SIZE, SIZE);

	static uint8_t px[SIZE * SIZE * 4];
	uint32_t hash = 0;
	int red_first = 0;
	double start = now_ms();
	for (int f = 0; f < frames; f++) {
		glClearColor(0.0f, 0.0f, 1.0f, 1.0f);
		glClear(GL_COLOR_BUFFER_BIT);
		glDrawArrays(GL_TRIANGLES, 0, 3);
		memset(px, 0x55, sizeof(px));
		glReadPixels(0, 0, SIZE, SIZE, GL_RGBA, GL_UNSIGNED_BYTE, px);
		if (glGetError() != GL_NO_ERROR)
			return fail("glReadPixels");
		int red = 0, blue = 0;
		for (int i = 0; i < SIZE * SIZE; i++) {
			const uint8_t *p = &px[i * 4];
			if (p[0] == 255 && p[1] == 0 && p[2] == 0 && p[3] == 255)
				red++;
			else if (p[0] == 0 && p[1] == 0 && p[2] == 255 && p[3] == 255)
				blue++;
		}
		/* Row 0 is the bottom: (0,0) red, (63,63) blue. */
		int corners = px[0] == 255 && px[(SIZE * SIZE - 1) * 4 + 2] == 255;
		int want = (SIZE + 1) * SIZE / 2; /* centres on or below the diagonal */
		if (red + blue != SIZE * SIZE || red != want || !corners) {
			printf("gles-probe: FAIL: frame %d: %d red, %d blue, other %d, corners %s, first pixel %02x%02x%02x%02x\n",
			       f, red, blue, SIZE * SIZE - red - blue, corners ? "right" : "wrong",
			       px[0], px[1], px[2], px[3]);
			return 1;
		}
		if (f == 0) {
			hash = 2166136261u;
			for (size_t i = 0; i < sizeof(px); i++)
				hash = (hash ^ px[i]) * 16777619u;
			red_first = red;
		}
	}
	double ms = now_ms() - start;
	printf("gles-probe: PASS: %s, %d of %d frames right, %d red pixels, hash %08x, %.2f ms per frame\n",
	       renderer, frames, frames, red_first, hash, ms / frames);
	eglMakeCurrent(dpy, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT);
	eglDestroyContext(dpy, ctx);
	eglTerminate(dpy);
	return 0;
}
