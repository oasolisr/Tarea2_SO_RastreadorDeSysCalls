// Declaración de módulos
mod parser;
mod tracer;

use parser::Args;
use clap::Parser;

fn main() {
    // Parsear los argumentos de la línea de comandos
    let args = Args::parse();

    // Validar que se haya pasado un programa a ejecutar
    args.validate();

    // Ejecutar el rastreador con los flags y programa especificado
    tracer::run_tracer(args.v, args.step, args.prog_and_args);
}
