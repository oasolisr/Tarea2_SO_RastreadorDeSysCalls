use nix::sys::ptrace;
use nix::sys::signal::raise;
use nix::sys::signal::Signal;
use nix::sys::wait::{waitpid, WaitStatus};
use nix::unistd::{execv, fork, ForkResult};
use std::collections::HashMap;
use std::ffi::CString;
use std::io::{self, Write};
use prettytable::{Table, Row, Cell};

// Cambiar a false para desactivar mensajes internos de debug
const DEBUG: bool = false;

// Macro de debug centralizado: imprime solo si DEBUG = true
macro_rules! debug {
    ($($arg:tt)*) => {
        if DEBUG {
            eprintln!($($arg)*);
        }
    };
}

/// Crea el mapa syscall_number → nombre
pub fn build_syscall_map() -> HashMap<u64, &'static str> {
    use std::iter::FromIterator;
    HashMap::from_iter(vec![
        (0, "read"),
        (1, "write"),
        (2, "open"),
        (3, "close"),
        (9, "mmap"),
        (11, "munmap"),
        (39, "getpid"),
        (57, "fork"),
        (59, "execve"),
        (60, "exit"),
        (61, "wait4"),
        (231, "exit_group"),
        (257, "openat"),
        (17, "pread64"),
        (32, "dup"),
        (33, "pipe"),
        (41, "socket"),
        (42, "connect"),
        (44, "sendto"),
        (45, "recvfrom"),
        (63, "uname"),
        (158, "arch_prctl"),
        (202, "futex"),
        (262, "newfstatat"),
    ])
}

/// Devuelve el nombre de la syscall o "sys_<num>" si no está en el mapa
fn syscall_name(map: &HashMap<u64, &'static str>, num: u64) -> String {
    map.get(&num).map(|s| s.to_string()).unwrap_or_else(|| format!("sys_{}", num))
}

/// Pausa la ejecución hasta que el usuario presione Enter (modo step -V)
fn wait_enter() {
    print!("Presione Enter para continuar...");
    io::stdout().flush().unwrap();
    let mut buf = String::new();
    let _ = io::stdin().read_line(&mut buf);
}

/// Convierte argumentos de Rust a CStrings para execv
fn path_and_args_to_cstrings(args: &[String]) -> (CString, Vec<CString>) {
    let prog = CString::new(args[0].as_str()).expect("CString::new failed for prog");
    let cargs = args.iter()
        .map(|s| CString::new(s.as_str()).expect("CString::new failed"))
        .collect();
    (prog, cargs)
}

/// Función principal del rastreador
pub fn run_tracer(verbose: bool, step: bool, prog_and_args: Vec<String>) {
    // Construir el mapa de syscalls y preparar contadores
    let syscall_map = build_syscall_map();
    let mut counts: HashMap<u64, u64> = HashMap::new();
    let mut last_syscall: Option<u64> = None;

    // Convertir args de Rust a CStrings
    let (_prog_c, args_c) = path_and_args_to_cstrings(&prog_and_args);

    // --- Fork: crear proceso hijo ---
    match unsafe { fork() } {
        Ok(ForkResult::Child) => {
            // --- Código del hijo ---
            debug!("Hijo iniciado, PID: {}", nix::unistd::getpid());

            // Permitir que el padre trace este proceso
            let _ = ptrace::traceme();
            debug!("Hijo: ptrace::traceme llamado");

            // Enviar SIGSTOP al padre para que pueda configurar ptrace
            let _ = raise(Signal::SIGSTOP).expect("failed to raise SIGSTOP");
            debug!("Hijo: SIGSTOP enviado al padre");

            // Ejecutar el programa deseado
            let cprog = &args_c[0];
            let cargs_refs: Vec<&std::ffi::CStr> = args_c.iter().map(|s| s.as_c_str()).collect();
            debug!("Hijo: ejecutando {:?}", args_c[0]);
            let res = execv(cprog.as_c_str(), &cargs_refs);
            eprintln!("Hijo: error ejecutando {}: {:?}", prog_and_args[0], res);
            std::process::exit(1);
        }
        Ok(ForkResult::Parent { child }) => {
            // --- Código del padre ---
            debug!("Padre: PID {}, child PID {}", nix::unistd::getpid(), child);

            // Esperar primer SIGSTOP del hijo
            let status = waitpid(child, None).expect("waitpid failed");
            debug!("Padre: waitpid retornó {:?}", status);

            if let WaitStatus::Stopped(_, Signal::SIGSTOP) = status {
                // Configurar ptrace para rastrear syscalls
                ptrace::setoptions(child, ptrace::Options::PTRACE_O_TRACESYSGOOD)
                    .expect("failed to set ptrace options");
                debug!("Padre: PTRACE_O_TRACESYSGOOD set");

                // Iniciar rastreo de syscalls
                ptrace::syscall(child, None).expect("ptrace syscall failed");
                debug!("Padre: inicio rastreo de syscalls");
            } else {
                panic!("Hijo no se detuvo como se esperaba");
            }

            let mut in_syscall = false;

            // --- Loop principal de rastreo ---
            loop {
                match waitpid(child, None) {
                    Ok(WaitStatus::Exited(_, code)) => {
                        // Hijo terminó normalmente
                        debug!("Hijo finalizó con código {}", code);
                        break;
                    }
                    Ok(WaitStatus::Signaled(_, sig, _)) => {
                        // Hijo terminó por señal
                        debug!("Hijo terminó por señal {:?}", sig);
                        break;
                    }
                    Ok(WaitStatus::Stopped(pid, sig)) => {
                        // Hijo detenido, revisar si es syscall
                        let signum = sig as i32;
                        debug!("Padre: hijo detenido PID {} señal {:?}", pid, sig);

                        if signum & 0x80 != 0 || signum == Signal::SIGTRAP as i32 {
                            // Obtener registros del hijo
                            let regs = match ptrace::getregs(pid) {
                                Ok(r) => r,
                                Err(_) => {
                                    debug!("Padre: error obteniendo registros, continuando");
                                    let _ = ptrace::syscall(pid, None);
                                    continue;
                                }
                            };
                            let num = regs.orig_rax as u64; // número de syscall
                            debug!("Padre: syscall detectada, num={}", num);

                            if !in_syscall {
                                // Entrada a syscall
                                *counts.entry(num).or_insert(0) += 1;
                                last_syscall = Some(num);
                                if verbose || step {
                                    println!("[ENTRY] {}({})", syscall_name(&syscall_map, num), num);
                                }
                                in_syscall = true;
                            } else {
                                // Salida de syscall
                                if verbose || step {
                                    println!(
                                        "[EXIT ] {}({}) => {}",
                                        last_syscall
                                            .map(|n| syscall_name(&syscall_map, n))
                                            .unwrap_or_else(|| format!("sys_{}", num)),
                                        num,
                                        regs.rax as i64
                                    );
                                }
                                in_syscall = false;
                            }

                            if step {
                                wait_enter();
                            }
                        }

                        // Continuar al siguiente evento/syscall
                        ptrace::syscall(pid, None).ok();
                    }
                    Ok(WaitStatus::PtraceEvent(pid, sig, ev)) => {
                        // Evento ptrace adicional
                        debug!("Padre: PtraceEvent PID {} sig {:?} evento {:?}", pid, sig, ev);
                        ptrace::syscall(child, None).ok();
                    }
                    Ok(WaitStatus::PtraceSyscall(pid)) => {
                        // Syscall interceptada (solo debug)
                        static mut COUNTER: usize = 0;
                        unsafe {
                            COUNTER += 1;
                            if COUNTER % 50 == 0 {
                                debug!("Padre: PtraceSyscall PID {}", pid);
                            }
                        }
                        ptrace::syscall(child, None).ok();
                    }
                    Err(e) => {
                        // Error en waitpid
                        debug!("waitpid error: {:?}", e);
                        break;
                    }
                    _ => {
                        debug!("Padre: waitpid returned unhandled estado");
                        ptrace::syscall(child, None).ok();
                    }
                }
            }

            // --- Imprimir tabla de syscalls ---
            let mut vec_counts: Vec<(u64, u64)> = counts.into_iter().collect();
            vec_counts.sort_by(|a, b| b.1.cmp(&a.1)); // ordenar por cantidad descendente

            let mut table = Table::new();
            table.add_row(Row::new(vec![Cell::new("System Call"), Cell::new("Veces")]));
            for (num, cnt) in vec_counts {
                table.add_row(Row::new(vec![
                    Cell::new(&syscall_name(&syscall_map, num)),
                    Cell::new(&cnt.to_string()),
                ]));
            }
            table.printstd();
        }
        Err(e) => eprintln!("Error en fork: {}", e),
    }
}
