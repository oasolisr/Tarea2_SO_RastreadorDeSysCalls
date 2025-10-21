use clap::Parser;

/// Estructura de argumentos del rastreador.
/// Uso:
///   rastreador [-v | -V] -- prog arg1 arg2 ...
#[derive(Parser, Debug)]
#[command(author, version, about, disable_version_flag = true)]
pub struct Args {
    /// Verbose: imprimir info por syscall
    #[arg(short = 'v', long = "verbose", conflicts_with = "step")]
    pub v: bool,

    /// Verbose + step: imprimir info y pausar hasta Enter en cada syscall
    #[arg(short = 'V', long = "step")]
    pub step: bool,

    /// Programa y sus argumentos (el primer argumento después de opciones)
    #[arg(required = true, trailing_var_arg = true)]
    pub prog_and_args: Vec<String>,
}

impl Args {
    /// Muestra uso amigable si no se pasan argumentos
    pub fn validate(&self) {
        if self.prog_and_args.is_empty() {
            eprintln!("Debe especificar el programa a ejecutar y sus argumentos");
            std::process::exit(1);
        }
    }
}
