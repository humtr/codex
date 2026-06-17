#include <errno.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

static const char codex_managed_launcher_marker[] __attribute__((used)) =
    "codex termux managed launcher";

static int safe_join(char *out, size_t out_sz, const char *a, const char *b) {
    int n = snprintf(out, out_sz, "%s/%s", a, b);
    return (n >= 0 && (size_t)n < out_sz) ? 0 : -1;
}

int main(int argc, char **argv) {
    const char *home = getenv("HOME");
    const char *prefix = getenv("PREFIX");
    const char *managed = getenv("CODEX_TERMUX_MANAGED_SHELL");
    const char *bash = getenv("CODEX_TERMUX_BASH");
    char default_managed[PATH_MAX];
    char default_bash[PATH_MAX];
    char **outv;
    int i;

    if (!home || !*home) home = "/data/data/com.termux/files/home";
    if (!prefix || !*prefix) prefix = "/data/data/com.termux/files/usr";
    if (!managed || !*managed) {
        if (safe_join(default_managed, sizeof(default_managed), home,
                ".local/lib/codex/termux/manager/managed.sh") < 0) {
            return 125;
        }
        managed = default_managed;
    }
    if (!bash || !*bash) {
        if (safe_join(default_bash, sizeof(default_bash), prefix, "bin/bash") < 0) {
            return 125;
        }
        bash = default_bash;
    }

    outv = calloc((size_t)argc + 2, sizeof(char *));
    if (!outv) {
        fprintf(stderr, "codex-launcher: allocation failed: %s\n", strerror(errno));
        return 125;
    }
    outv[0] = (char *)bash;
    outv[1] = (char *)managed;
    for (i = 1; i < argc; i++) outv[i + 1] = argv[i];
    outv[argc + 1] = NULL;
    execv(bash, outv);
    fprintf(stderr, "codex-launcher: failed to exec %s %s: %s\n",
        bash, managed, strerror(errno));
    return 126;
}
