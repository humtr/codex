#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/wait.h>
#include <unistd.h>

// Core dispatch, saved-session lookup and the model frontend are test fixtures.
// Manager, the AM client, Termux service and Android PTYs are real, inside an
// empty disposable emulator installation; no real Native inference is claimed.
#ifndef BASE
#define BASE "/data/data/com.termux/files/home/.stock-proof"
#endif
#define MANAGER BASE "/manager"
#define CORE BASE "/core"
#ifndef NATIVE
#define NATIVE "/data/data/com.termux/files/home/.local/lib/codex/core/generations/stock-proof/helpers/2"
#endif
// Fixed stage names only: never record environment values, commands or tokens.
// Diagnostics must not alter inherited PTYs, child status or errno.
static void stage(const char *name, int detail) {
    int saved_errno=errno;
    int fd=open(BASE "/stages.log",O_CREAT|O_APPEND|O_WRONLY|O_CLOEXEC|O_NOFOLLOW,0600);
    if(fd>=0) {
        struct stat st;
        if(!fstat(fd,&st)&&S_ISREG(st.st_mode)&&st.st_size<65536)
            dprintf(fd,"pid=%d ppid=%d stage=%s detail=%d errno=%d\n",
                    getpid(),getppid(),name,detail,saved_errno);
        close(fd);
    }
    errno=saved_errno;
}
static int failed(const char *name, int code) { stage(name,code);return code; }
static int saved_thread(const char *id) {
    if(!id||strlen(id)!=36)return 0;
    for(size_t i=0;i<36;i++) {
        if(i==8||i==13||i==18||i==23) { if(id[i]!='-')return 0; }
        else if(!((id[i]>='0'&&id[i]<='9')||(id[i]>='a'&&id[i]<='f')))return 0;
    }
    char saved[1024];const char *home=getenv("CODEX_HOME");
    if(!home)return 0;
    int n=snprintf(saved,sizeof(saved),"%s/%s.saved",home,id);
    return n>=0&&(size_t)n<sizeof(saved)&&!access(saved,R_OK);
}
int main(int argc, char **argv) {
    stage("entry",argc);
    char cwd[1024];const char *home=getenv("HOME");
    stage("cwd-is-home",home&&getcwd(cwd,sizeof(cwd))&&!strcmp(cwd,home));
    setenv("CODEX_TERMUX_CORE_API", "codex-manager-core-v1", 1);
    setenv("CODEX_TERMUX_CORE_ENTRYPOINT", CORE, 1);
    if (argc > 2 && !strcmp(argv[1], "termux") && !strcmp(argv[2], "__terminal-capabilities-v1")) {
        // Explicit synthetic Core admission for the fixture's real FD publisher.
        // This does not qualify a product Core, Native Owner or model inference.
        struct stat st;
        if(argc!=6||strcmp(argv[3],"--")||strcmp(argv[4],"resume")||!saved_thread(argv[5])
                ||lstat(NATIVE,&st)||!S_ISREG(st.st_mode)||access(NATIVE,X_OK))
            return failed("core-capability-unavailable",26);
        stage("core-capability-native",0);
        fputs("codex-terminal-capabilities-v1\nidentity=native\n",stdout);
        return 0;
    }
    if (argc > 2 && !strcmp(argv[1], "termux")) {
        stage("core-manager-dispatch",0);
        char **args = calloc(argc, sizeof(char *));
        if(!args)return failed("dispatch-allocation",10);
        args[0] = MANAGER;
        for (int i=2; i<argc; i++) args[i-1]=argv[i];
        execv(MANAGER,args);return failed("dispatch-exec",11);
    }
    if (argc == 3 && !strcmp(argv[1], "launcher")) {
        stage("launcher",0);
        pid_t child=fork();if(child<0)return failed("launcher-fork",12);
        if(!child){execl(MANAGER,"manager","launch","--profile","default","--",argv[2],NULL);_exit(failed("launcher-manager-exec",13));}
        int status;if(waitpid(child,&status,0)<0)return failed("launcher-wait",14);
        stage("launcher-manager-status",status);
        if(!WIFEXITED(status)||WEXITSTATUS(status))return failed("launcher-manager-failed",14);
        // The invoking fallback shell is deliberately kept; it is not a workload.
        for(;;)pause();
    }
    if (strcmp(argv[0], NATIVE)) {
        const char *id=argc>1?argv[1]:NULL;
        if(argc==3&&!strcmp(argv[1],"resume")) {
            // Test-only saved-thread oracle. Real Owner/writer and real model
            // continuity are qualified separately; do not create a fresh ID.
            if(!saved_thread(argv[2]))return failed("resume-saved-thread",25);
            id=argv[2];
        }
        stage("core-native-exec",0);
        char *args[]={NATIVE,(char *)id,NULL};execv(NATIVE,args);return failed("native-exec",15);
    }
    stage("native-entry",argc);
    if(argc!=2||strlen(argv[1])!=36)return failed("native-id",16);
    char path[1024],descriptor[32];
    snprintf(path,sizeof(path),"%s/.codex-terminal-thread-v1-%d-stock-proof",getenv("TMPDIR"),getpid());
    int slot=open(path,O_CREAT|O_EXCL|O_RDWR,0600);if(slot<0)return failed("slot-open",17);
    if(unlink(path)||write(slot,argv[1],36)!=36)return failed("slot-content",18);
    stage("slot-ready",slot);
    snprintf(descriptor,sizeof(descriptor),"%d",slot);
    pid_t child=fork();if(child<0)return failed("bind-fork",19);
    if(!child){execl(MANAGER,"manager","__terminal-bind-v1",descriptor,NULL);_exit(failed("bind-manager-exec",20));}
    int status;if(waitpid(child,&status,0)<0)return failed("bind-wait",21);
    stage("bind-manager-status",status);
    if(!WIFEXITED(status)||WEXITSTATUS(status))return failed("bind-manager-failed",21);
    snprintf(path,sizeof(path),BASE "/%s.ready",argv[1]);FILE *f=fopen(path,"w");if(!f)return failed("ready-open",22);
    fprintf(f,"%d",getpid());fclose(f);
    f=fopen(BASE "/workloads","a");if(!f)return failed("workload-open",23);fprintf(f,"%s\n",argv[1]);fclose(f);
    stage("workload-ready",0);
    for(;;)pause();
}
