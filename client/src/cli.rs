use clap::Parser;

#[derive(Parser)]
pub struct Cli {
    /// Request an ID from the server
    #[arg(long, short = 'r')]
    pub request_id: Option<String>,

    /// The address of the Schmu server
    #[arg(long, short = 'S', default_value = shared::consts::SERVER_ADDRESS)]
    pub server_address: String,

    /// The port of the Schmu server
    #[arg(long, short = 'P', default_value_t = shared::consts::SERVER_PORT_PUBLIC)]
    pub server_port: u16,
}
