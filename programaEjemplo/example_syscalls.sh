#!/usr/bin/env bash
set -euo pipefail

echo "[example_syscalls_simple.sh] start"
echo "PID $$ PPID $PPID"

# Use a safe temp dir
DIR="$(mktemp -d /tmp/rastro_test_bash.XXXXXX)"
FILE="$DIR/file.txt"

# 1) uname -> syscall uname (80)
uname -a

# 2) crear directorio y archivo -> mkdir (90), open/write/close (2/1/3), chmod (83)
# (mkdir ya hecho por mktemp; creamos el archivo)
echo "Linea de prueba para syscalls desde bash" > "$FILE"
chmod 0644 "$FILE"

# 3) stat / fstat / newfstatat -> usar stat
stat "$FILE" >/dev/null

# 4) lseek -> usar dd para mover el offset en lectura (lseek)
# dd lee del archivo; si algo falla no rompe el script por el || true
dd if="$FILE" of=/dev/null bs=1 count=1 skip=10 status=none || true

# 5) pipe -> tuberia simple entre procesos (22 pipe)
# Usamos una tubería simple garantizada
printf "mensaje por pipe\n" | { read -r LINE; echo "padre leyó pipe: $LINE"; }

# 6) FIFO (mkfifo) -> open/read/write/close
FIFO="$DIR/myfifo"
rm -f "$FIFO"
mkfifo "$FIFO"
# lector en background
{ read -r FMSG < "$FIFO"; echo "leí FIFO: $FMSG"; } &
# escritor
echo "fifo mensaje" > "$FIFO"
# esperar al lector
wait

# 7) mmap/munmap y brk -> usar python para mapear/leer/munmap (syscalls 9/11/12)
export EXAMPLE_FILE="$FILE"

python3 - <<'PY'
import mmap, os

fpath = os.environ.get("EXAMPLE_FILE")
if fpath and os.path.exists(fpath):
    fd = os.open(fpath, os.O_RDONLY)
    st = os.fstat(fd)
    if st.st_size > 0:
        m = mmap.mmap(fd, st.st_size, prot=mmap.PROT_READ)
        print(m[:min(50, st.st_size)].decode(errors='ignore'))
        m.close()
    os.close(fd)
else:
    print("python: no se encontró EXAMPLE_FILE o archivo vacío")
PY
# Pasamos la ruta mediante variable de entorno para python
export EXAMPLE_FILE="$FILE"
# Ejecutamos el bloque python anterior (nota: ya ejecutado con heredado EXAMPLE_FILE)

# 8) access -> test -r (syscall access)
if [ -r "$FILE" ]; then
  echo "archivo legible"
fi

# 9) fork + wait -> subshells y background jobs (57 fork, 61 wait4)
( sleep 0.05; echo "hijo corto" ) &
CHILD=$!
wait "$CHILD" || true

# 10) trap -> instalar handler -> rt_sigaction (133)
trap 'echo "signal handler triggered"' SIGUSR1
# enviar señal a mí mismo para activar handler
kill -USR1 $$

# 11) chmod/chown/unlink -> chmod (83), chown (84), unlink (87)
touch "$DIR/temp_for_chown"
chmod 0600 "$DIR/temp_for_chown"
chown "$(id -u):$(id -g)" "$DIR/temp_for_chown" || true
rm -f "$DIR/temp_for_chown"

# 12) unlink -> rm (87) y rmdir (91)
rm -f "$FILE" || true
rm -f "$FIFO" || true
rmdir "$DIR" || true

echo "[example_syscalls_simple.sh] about to exec /bin/true (execve)"
exec /bin/true
