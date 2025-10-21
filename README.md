# Rastreador de System Calls en Rust

## Descripción

Rastreador de llamadas al sistema en Rust que permite ejecutar cualquier programa en Linux, registrar todas sus syscalls y mostrar un conteo acumulativo. Incluye modos verbose y paso a paso para análisis detallado.

## Características

* Rastreo de syscalls de cualquier programa ejecutado.
* Modo `-v`: muestra cada syscall durante la ejecución.
* Modo `-V`: pausa la ejecución en cada syscall hasta que el usuario presione Enter.
* Tabla final acumulativa con frecuencia de cada syscall.

## Requisitos

* Sistema operativo: Linux (Ubuntu recomendado)
* Rust 1.71 o superior
* Dependencias de Rust: `nix`, `clap`, `prettytable`, `libc`

## Instalación

Clonar el repositorio y compilar:

```bash
git clone https://github.com/oasolisr/Tarea2_SO_RastreadorDeSysCalls.git
cd rastreador
cargo build --release
```

El binario se generará en `target/release/rastreador`.

## Uso

Ejecutar un programa y mostrar solo la tabla final:

```bash
sudo ./target/release/rastreador /bin/ls -l /usr
```

Ejecutar en modo verbose:

```bash
sudo ./target/release/rastreador -v /bin/ls -l /usr
```

Ejecutar en modo paso a paso (pause en cada syscall):

```bash
sudo ./target/release/rastreador -V /bin/ls -l /usr
```

Ejemplo con un programa más complejo (`tar`):

```bash
sudo ./target/release/rastreador -v /bin/tar -czf /tmp/test.tar.gz -C /usr/bin .
```