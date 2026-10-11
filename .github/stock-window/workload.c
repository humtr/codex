#define _GNU_SOURCE
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/wait.h>
#include <unistd.h>

// Only the model frontend is synthetic. Manager, the AM client, Termux service
// and Android PTYs are real, inside an empty disposable emulator installation.
#define BASE "/data/data/com.termux/files/home/.stock-proof"
#define MANAGER BASE "/manager"
#define CORE BASE "/core"
#define NATIVE "/data/data/com.termux/files/home/.local/lib/codex/core/generations/stock-proof/helpers/2"
int main(int argc, char **argv) {
    setenv("CODEX_TERMUX_CORE_API", "codex-manager-core-v1", 1);
    setenv("CODEX_TERMUX_CORE_ENTRYPOINT", CORE, 1);
    if (argc > 2 && !strcmp(argv[1], "termux")) {
        char **args = calloc(argc, sizeof(char *));
        args[0] = MANAGER;
        for (int i=2; i<argc; i++) args[i-1]=argv[i];
        execv(MANAGER,args);return 11;
    }
    if (argc == 3 && !strcmp(argv[1], "launcher")) {
        pid_t child=fork();if(child<0)return 12;
        if(!child){execl(MANAGER,"manager","launch","--profile","default","--",argv[2],NULL);_exit(13);}
        int status;if(waitpid(child,&status,0)<0||!WIFEXITED(status)||WEXITSTATUS(status))return 14;
        // The invoking fallback shell is deliberately kept; it is not a workload.
        for(;;)pause();
    }
    if (strcmp(argv[0], NATIVE)) {
        char *args[]={NATIVE,argc>1?argv[1]:NULL,NULL};execv(NATIVE,args);return 15;
    }
    if(argc!=2||strlen(argv[1])!=36)return 16;
    char path[1024],descriptor[32];
    snprintf(path,sizeof(path),"%s/.codex-terminal-thread-v1-%d-stock-proof",getenv("TMPDIR"),getpid());
    int slot=open(path,O_CREAT|O_EXCL|O_RDWR,0600);if(slot<0)return 17;
    if(unlink(path)||write(slot,argv[1],36)!=36)return 18;
    snprintf(descriptor,sizeof(descriptor),"%d",slot);
    pid_t child=fork();if(child<0)return 19;
    if(!child){execl(MANAGER,"manager","__terminal-bind-v1",descriptor,NULL);_exit(20);}
    int status;if(waitpid(child,&status,0)<0||!WIFEXITED(status)||WEXITSTATUS(status))return 21;
    snprintf(path,sizeof(path),BASE "/%s.ready",argv[1]);FILE *f=fopen(path,"w");if(!f)return 22;
    fprintf(f,"%d",getpid());fclose(f);
    f=fopen(BASE "/workloads","a");if(!f)return 23;fprintf(f,"%s\n",argv[1]);fclose(f);
    for(;;)pause();
}
