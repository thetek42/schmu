use clap::Parser;

#[derive(Parser)]
pub struct Cli {
    #[arg(long, short = 'r')]
    pub request_id: Option<String>,
}
