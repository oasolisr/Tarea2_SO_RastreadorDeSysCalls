mod parser;
mod tracer;

use parser::Args;
use clap::Parser;

fn main() {
    let args = Args::parse();
    args.validate();

    tracer::run_tracer(args.v, args.step, args.prog_and_args);
}
