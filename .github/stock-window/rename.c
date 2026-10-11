#define _GNU_SOURCE
#include <sys/syscall.h>
#include <unistd.h>
// Emulator-only libc bridge: production arm64 already calls this kernel syscall.
// This links the unchanged Manager source on Android/x86_64; it is not an APK or
// release patch and does not replace any window/AM/binding implementation.
int renameat2(int olddir, const char *oldpath, int newdir, const char *newpath, unsigned int flags) {
    return (int)syscall(SYS_renameat2, olddir, oldpath, newdir, newpath, flags);
}
