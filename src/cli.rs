use std::path::PathBuf;

use clap::Clap;


#[derive(Clap)]
pub(crate) struct Opts {
    pub parse_server_port: u16,
    pub wiki_xml: PathBuf,
    pub output_file: Option<PathBuf>,
    pub title: Option<String>,
    #[clap(long, short)]
    pub and_after: bool,
    #[clap(long, short)]
    pub xhtml_output: bool,
    #[clap(long, short)]
    pub no_plain_output: bool,
}
