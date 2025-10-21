// example_syscalls.c
// Genera una variedad de syscalls para testing de rastreadores.
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <fcntl.h>
#include <sys/stat.h>
#include <sys/mman.h>
#include <sys/types.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <string.h>
#include <errno.h>
#include <sys/ioctl.h>
#include <sys/wait.h>
#include <time.h>
#include <sys/time.h>
#include <sys/utsname.h>
#include <termios.h>

int main(void) {
    printf("[example_syscalls] start\n");

    // 1) pid info
    pid_t pid = getpid(), ppid = getppid();
    printf("PID=%d PPID=%d\n", pid, ppid);

    // 2) time syscalls
    struct timeval tv;
    gettimeofday(&tv, NULL);
    struct timespec ts;
    clock_gettime(CLOCK_REALTIME, &ts);
    printf("time: tv=%ld.%06ld ts=%ld.%09ld\n",
           (long)tv.tv_sec, (long)tv.tv_usec,
           (long)ts.tv_sec, (long)ts.tv_nsec);

    // 3) uname (syscall uname)
    struct utsname u;
    uname(&u);
    printf("sysname: %s nodename: %s\n", u.sysname, u.nodename);

    // 4) mkdir, chmod
    const char *dir = "/tmp/rastro_test";
    mkdir(dir, 0755);
    chmod(dir, 0750);

    // 5) open / write / fsync / lseek / read / close
    char path[256];
    snprintf(path, sizeof(path), "%s/file.txt", dir);
    int fd = open(path, O_CREAT | O_RDWR, 0644);
    if (fd < 0) { perror("open"); return 1; }
    const char *text = "Linea de prueba para syscalls\n";
    write(fd, text, strlen(text));
    fsync(fd);
    lseek(fd, 0, SEEK_SET);
    char buf[128];
    ssize_t n = read(fd, buf, sizeof(buf)-1);
    if (n > 0) { buf[n] = 0; printf("read: %s", buf); }
    // 6) mmap / munmap
    off_t flen = lseek(fd, 0, SEEK_END);
    if (flen > 0) {
        void *m = mmap(NULL, flen, PROT_READ, MAP_PRIVATE, fd, 0);
        if (m != MAP_FAILED) {
            // acceder al mmap provoca read en el kernel si no está en cache
            write(STDOUT_FILENO, m, flen);
            munmap(m, flen);
        }
    }
    close(fd);

    // 7) pipe, dup2, write/read through pipe
    int p[2];
    pipe(p); // pipe -> read/write fds
    if (fork() == 0) {
        // child: escribe en la tuberia
        close(p[0]);
        const char *msg = "mensaje por pipe\n";
        write(p[1], msg, strlen(msg));
        close(p[1]);
        _exit(0);
    } else {
        // parent: lee
        close(p[1]);
        char pbuf[64];
        ssize_t pr = read(p[0], pbuf, sizeof(pbuf)-1);
        if (pr > 0) { pbuf[pr] = 0; printf("padre leyó pipe: %s", pbuf); }
        close(p[0]);
        wait(NULL); // waitpid
    }

    // 8) ioctl (obtiene tamaño de terminal) -> TIOCGWINSZ
    struct winsize ws;
    if (ioctl(STDOUT_FILENO, TIOCGWINSZ, &ws) == 0) {
        printf("tty rows=%d cols=%d\n", ws.ws_row, ws.ws_col);
    }

    // 9) socket: crea un servidor TCP y un cliente que se conecta (loopback)
    int srv = socket(AF_INET, SOCK_STREAM, 0);
    if (srv >= 0) {
        struct sockaddr_in sa;
        memset(&sa, 0, sizeof(sa));
        sa.sin_family = AF_INET;
        sa.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
        sa.sin_port = 0; // puerto 0 => kernel asigna
        bind(srv, (struct sockaddr*)&sa, sizeof(sa));
        // obtener puerto asignado
        socklen_t len = sizeof(sa);
        getsockname(srv, (struct sockaddr*)&sa, &len);
        int port = ntohs(sa.sin_port);
        listen(srv, 1);
        pid_t forked = fork();
        if (forked == 0) {
            // cliente
            int cli = socket(AF_INET, SOCK_STREAM, 0);
            struct sockaddr_in csa;
            memset(&csa,0,sizeof(csa));
            csa.sin_family = AF_INET;
            csa.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
            csa.sin_port = htons(port);
            // un poco de espera para que el servidor esté listo
            struct timespec t = {0, 200 * 1000 * 1000}; // 200ms
            nanosleep(&t, NULL);
            if (connect(cli, (struct sockaddr*)&csa, sizeof(csa)) == 0) {
                const char *hello = "hola desde cliente\n";
                send(cli, hello, strlen(hello), 0);
                char rcv[128];
                ssize_t rr = recv(cli, rcv, sizeof(rcv)-1, 0);
                if (rr > 0) { rcv[rr]=0; write(STDOUT_FILENO, rcv, rr); }
            }
            close(cli);
            _exit(0);
        } else {
            // servidor acepta
            int acc = accept(srv, NULL, NULL);
            char rcv2[128];
            ssize_t rr2 = recv(acc, rcv2, sizeof(rcv2)-1, 0);
            if (rr2 > 0) {
                rcv2[rr2] = 0;
                // responde
                const char *reply = "respuesta del servidor\n";
                send(acc, reply, strlen(reply), 0);
                write(STDOUT_FILENO, rcv2, rr2);
            }
            close(acc);
            wait(NULL); // espera al cliente
        }
        close(srv);
    }

    // 10) fork + execve (lanzar /bin/true) -> execve syscall
    pid_t p2 = fork();
    if (p2 == 0) {
        char *argv[] = {"/bin/true", NULL};
        execve("/bin/true", argv, NULL);
        // si falla:
        _exit(127);
    } else {
        waitpid(p2, NULL, 0);
    }

    // 11) unlink y rmdir
    unlink(path);
    rmdir(dir);

    printf("[example_syscalls] end\n");
    return 0;
}
