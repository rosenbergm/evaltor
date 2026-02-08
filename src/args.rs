use std::path::PathBuf;

use clap::Parser;

#[derive(Clone, Debug, Parser)]
pub struct EvaltorArgs {
    /// Path to the submissions directory
    #[clap(short, long, env = "EVALTOR_SUBMISSIONS")]
    pub submissions: PathBuf,

    /// Path to the tests directory
    #[clap(short, long, env = "EVALTOR_TESTS")]
    pub tests: PathBuf,

    /// Hostname
    #[clap(long, env = "EVALTOR_HOSTNAME")]
    pub hostname: String,

    /// Port
    #[clap(long, env = "EVALTOR_PORT")]
    pub port: u16,

    /// Google Client ID
    #[clap(long, env = "EVALTOR_GOOGLE_CLIENT_ID")]
    pub google_client_id: String,

    /// Google Client Secret
    #[clap(long, env = "EVALTOR_GOOGLE_CLIENT_SECRET")]
    pub google_client_secret: String,
}
