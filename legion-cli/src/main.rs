use clap::Parser;

fn main() {
    let args = legion_cli::cli::Cli::parse();
    legion_cli::app::run(args);
}
