/* sdl-gles-probe (#669): OpenGL ES 2.0 through SDL2 in an Orbital window.
 *
 *   sdl-gles-probe [frames]              default 300
 *       SDL_GL: an OpenGL ES 2.0 context for an SDL window (EGL on Mesa,
 *       the V3D on the Pi 4/400), GL functions through
 *       SDL_GL_GetProcAddress. Each frame clears the window to blue and
 *       draws a triangle over its lower left half in a colour that cycles
 *       with the frame, reads the pixels back, checks them exactly and
 *       shows the frame with SDL_GL_SwapWindow.
 *   sdl-gles-probe --renderer [frames]
 *       SDL's 2D renderer (SDL_CreateRenderer), OpenGL ES 2.0 unless
 *       SDL_RENDER_DRIVER names another one (Orbital's default is the
 *       software renderer, #769): a red square on blue each frame,
 *       read back with SDL_RenderReadPixels and checked.
 *
 * No pixel centre lies on an edge, so every renderer gives the same pixels
 * (the hash of frame 0 must match LIBGL_ALWAYS_SOFTWARE=1). Prints
 * "sdl-gles-probe: PASS: ..." or "sdl-gles-probe: FAIL: ..." and exits 0
 * or 1.
 */

#include <SDL.h>
#include <SDL_opengles2.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define SIZE 256

static int fail(const char *what)
{
	printf("sdl-gles-probe: FAIL: %s: %s\n", what, SDL_GetError());
	return 1;
}

static int pump(void)
{
	SDL_Event e;
	while (SDL_PollEvent(&e))
		if (e.type == SDL_QUIT)
			return 0;
	return 1;
}

static uint32_t fnv(const uint8_t *p, size_t n)
{
	uint32_t h = 2166136261u;
	while (n--)
		h = (h ^ *p++) * 16777619u;
	return h;
}

/* GL functions, all through SDL_GL_GetProcAddress. */
#define GLFUNCS(X)                                                                       \
	X(const GLubyte *, glGetString, (GLenum))                                       \
	X(GLenum, glGetError, (void))                                                   \
	X(void, glClearColor, (GLfloat, GLfloat, GLfloat, GLfloat))                     \
	X(void, glClear, (GLbitfield))                                                  \
	X(void, glViewport, (GLint, GLint, GLsizei, GLsizei))                           \
	X(GLuint, glCreateShader, (GLenum))                                             \
	X(void, glShaderSource, (GLuint, GLsizei, const GLchar *const *, const GLint *)) \
	X(void, glCompileShader, (GLuint))                                              \
	X(void, glGetShaderiv, (GLuint, GLenum, GLint *))                               \
	X(GLuint, glCreateProgram, (void))                                              \
	X(void, glAttachShader, (GLuint, GLuint))                                       \
	X(void, glBindAttribLocation, (GLuint, GLuint, const GLchar *))                 \
	X(void, glLinkProgram, (GLuint))                                                \
	X(void, glGetProgramiv, (GLuint, GLenum, GLint *))                              \
	X(void, glUseProgram, (GLuint))                                                 \
	X(GLint, glGetUniformLocation, (GLuint, const GLchar *))                        \
	X(void, glUniform4f, (GLint, GLfloat, GLfloat, GLfloat, GLfloat))               \
	X(void, glVertexAttribPointer, (GLuint, GLint, GLenum, GLboolean, GLsizei, const void *)) \
	X(void, glEnableVertexAttribArray, (GLuint))                                    \
	X(void, glDrawArrays, (GLenum, GLint, GLsizei))                                 \
	X(void, glReadPixels, (GLint, GLint, GLsizei, GLsizei, GLenum, GLenum, void *))
#define DECL(ret, name, args) static ret(*p_##name) args;
GLFUNCS(DECL)

static int load_gl(void)
{
#define LOAD(ret, name, args)                                                     \
	if (!(p_##name = (ret(*) args)SDL_GL_GetProcAddress(#name))) {            \
		printf("sdl-gles-probe: FAIL: SDL_GL_GetProcAddress(\"%s\")\n", #name); \
		return -1;                                                        \
	}
	GLFUNCS(LOAD)
	return 0;
}

static GLuint shader(GLenum type, const char *src)
{
	GLuint s = p_glCreateShader(type);
	GLint ok = 0;
	p_glShaderSource(s, 1, &src, NULL);
	p_glCompileShader(s);
	p_glGetShaderiv(s, GL_COMPILE_STATUS, &ok);
	return ok ? s : 0;
}

static int run_gl(int frames)
{
	SDL_GL_SetAttribute(SDL_GL_CONTEXT_PROFILE_MASK, SDL_GL_CONTEXT_PROFILE_ES);
	SDL_GL_SetAttribute(SDL_GL_CONTEXT_MAJOR_VERSION, 2);
	SDL_GL_SetAttribute(SDL_GL_CONTEXT_MINOR_VERSION, 0);
	SDL_Window *win = SDL_CreateWindow("sdl-gles-probe", SDL_WINDOWPOS_CENTERED, SDL_WINDOWPOS_CENTERED,
					   SIZE, SIZE, SDL_WINDOW_OPENGL);
	if (!win)
		return fail("SDL_CreateWindow with SDL_WINDOW_OPENGL");
	SDL_GLContext ctx = SDL_GL_CreateContext(win);
	if (!ctx)
		return fail("SDL_GL_CreateContext (OpenGL ES 2.0)");
	if (load_gl())
		return 1;
	const char *renderer = (const char *)p_glGetString(GL_RENDERER);
	printf("sdl-gles-probe: SDL video driver %s, GL_RENDERER %s, GL_VERSION %s\n", SDL_GetCurrentVideoDriver(),
	       renderer, (const char *)p_glGetString(GL_VERSION));
	int dw = 0, dh = 0;
	SDL_GL_GetDrawableSize(win, &dw, &dh);
	if (dw != SIZE || dh != SIZE) {
		printf("sdl-gles-probe: FAIL: drawable %dx%d, not %dx%d\n", dw, dh, SIZE, SIZE);
		return 1;
	}

	GLuint vs = shader(GL_VERTEX_SHADER, "attribute vec2 pos;\n"
					     "void main() { gl_Position = vec4(pos, 0.0, 1.0); }\n");
	GLuint fs = shader(GL_FRAGMENT_SHADER, "precision mediump float;\n"
					       "uniform vec4 color;\n"
					       "void main() { gl_FragColor = color; }\n");
	if (!vs || !fs)
		return fail("shader compile");
	GLuint prog = p_glCreateProgram();
	p_glAttachShader(prog, vs);
	p_glAttachShader(prog, fs);
	p_glBindAttribLocation(prog, 0, "pos");
	p_glLinkProgram(prog);
	GLint linked = 0;
	p_glGetProgramiv(prog, GL_LINK_STATUS, &linked);
	if (!linked)
		return fail("program link");
	p_glUseProgram(prog);
	GLint u_color = p_glGetUniformLocation(prog, "color");
	/* The long edge passes half a pixel beyond the centres on the diagonal. */
	static const GLfloat tri[] = { -1, -1, 1 + 1.0f / SIZE, -1, -1, 1 + 1.0f / SIZE };
	p_glVertexAttribPointer(0, 2, GL_FLOAT, GL_FALSE, 0, tri);
	p_glEnableVertexAttribArray(0);
	p_glViewport(0, 0, SIZE, SIZE);

	static const uint8_t colours[3][3] = { { 255, 0, 0 }, { 0, 255, 0 }, { 255, 255, 0 } };
	static uint8_t px[SIZE * SIZE * 4];
	uint32_t hash = 0;
	int want = (SIZE + 1) * SIZE / 2, done = 0;
	Uint64 start = SDL_GetPerformanceCounter();
	for (int f = 0; f < frames && pump(); f++) {
		const uint8_t *c = colours[(f / 30) % 3];
		p_glClearColor(0.0f, 0.0f, 1.0f, 1.0f);
		p_glClear(GL_COLOR_BUFFER_BIT);
		p_glUniform4f(u_color, c[0] / 255.0f, c[1] / 255.0f, c[2] / 255.0f, 1.0f);
		p_glDrawArrays(GL_TRIANGLES, 0, 3);
		memset(px, 0x55, sizeof(px));
		p_glReadPixels(0, 0, SIZE, SIZE, GL_RGBA, GL_UNSIGNED_BYTE, px);
		if (p_glGetError() != GL_NO_ERROR)
			return fail("drawing or glReadPixels");
		int fg = 0, bg = 0;
		for (int i = 0; i < SIZE * SIZE; i++) {
			const uint8_t *p = &px[i * 4];
			if (p[0] == c[0] && p[1] == c[1] && p[2] == c[2] && p[3] == 255)
				fg++;
			else if (p[0] == 0 && p[1] == 0 && p[2] == 255 && p[3] == 255)
				bg++;
		}
		if (fg != want || fg + bg != SIZE * SIZE) {
			printf("sdl-gles-probe: FAIL: frame %d: %d triangle pixels (want %d), %d blue, %d other\n", f,
			       fg, want, bg, SIZE * SIZE - fg - bg);
			return 1;
		}
		if (f == 0)
			hash = fnv(px, sizeof(px));
		SDL_GL_SwapWindow(win);
		done++;
	}
	double ms = (SDL_GetPerformanceCounter() - start) * 1000.0 / SDL_GetPerformanceFrequency() / (done ? done : 1);
	printf("sdl-gles-probe: PASS: SDL_GL, %s, %d frames right, %d triangle pixels, hash %08x, %.2f ms per frame\n",
	       renderer, done, want, hash, ms);
	SDL_GL_DeleteContext(ctx);
	SDL_DestroyWindow(win);
	return 0;
}

static int run_renderer(int frames)
{
	if (!getenv("SDL_RENDER_DRIVER"))
		SDL_SetHint(SDL_HINT_RENDER_DRIVER, "opengles2");
	SDL_Window *win = SDL_CreateWindow("sdl-gles-probe renderer", SDL_WINDOWPOS_CENTERED,
					   SDL_WINDOWPOS_CENTERED, SIZE, SIZE, 0);
	if (!win)
		return fail("SDL_CreateWindow");
	SDL_Renderer *r = SDL_CreateRenderer(win, -1, 0);
	if (!r)
		return fail("SDL_CreateRenderer");
	SDL_RendererInfo info;
	SDL_GetRendererInfo(r, &info);
	printf("sdl-gles-probe: SDL video driver %s, renderer %s\n", SDL_GetCurrentVideoDriver(), info.name);
	static uint32_t px[SIZE * SIZE];
	SDL_Rect sq = { 64, 64, 128, 128 };
	int done = 0;
	uint32_t hash = 0;
	Uint64 start = SDL_GetPerformanceCounter();
	for (int f = 0; f < frames && pump(); f++) {
		SDL_SetRenderDrawColor(r, 0, 0, 255, 255);
		SDL_RenderClear(r);
		SDL_SetRenderDrawColor(r, 255, 0, 0, 255);
		SDL_RenderFillRect(r, &sq);
		if (SDL_RenderReadPixels(r, NULL, SDL_PIXELFORMAT_ARGB8888, px, SIZE * 4))
			return fail("SDL_RenderReadPixels");
		int red = 0, blue = 0, inside_ok = 1;
		for (int y = 0; y < SIZE; y++)
			for (int x = 0; x < SIZE; x++) {
				uint32_t p = px[y * SIZE + x] | 0xFF000000u;
				int inside = x >= 64 && x < 192 && y >= 64 && y < 192;
				if (p == 0xFFFF0000u)
					red++;
				else if (p == 0xFF0000FFu)
					blue++;
				if (inside != (p == 0xFFFF0000u))
					inside_ok = 0;
			}
		if (!inside_ok || red != 128 * 128 || red + blue != SIZE * SIZE) {
			printf("sdl-gles-probe: FAIL: renderer frame %d: %d red (want %d), %d blue, square %s\n", f,
			       red, 128 * 128, blue, inside_ok ? "right" : "wrong");
			return 1;
		}
		if (f == 0)
			hash = fnv((const uint8_t *)px, sizeof(px));
		SDL_RenderPresent(r);
		done++;
	}
	double ms = (SDL_GetPerformanceCounter() - start) * 1000.0 / SDL_GetPerformanceFrequency() / (done ? done : 1);
	printf("sdl-gles-probe: PASS: renderer %s, %d frames right, hash %08x, %.2f ms per frame\n", info.name, done,
	       hash, ms);
	SDL_DestroyRenderer(r);
	SDL_DestroyWindow(win);
	return 0;
}

int main(int argc, char **argv)
{
	int renderer = 0, frames = 300;
	setvbuf(stdout, NULL, _IONBF, 0);
	for (int i = 1; i < argc; i++) {
		if (!strcmp(argv[i], "--renderer"))
			renderer = 1;
		else
			frames = atoi(argv[i]);
	}
	if (frames < 1)
		frames = 1;
	if (SDL_Init(SDL_INIT_VIDEO))
		return fail("SDL_Init");
	int rc = renderer ? run_renderer(frames) : run_gl(frames);
	SDL_Quit();
	return rc;
}
