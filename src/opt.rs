#[derive(StructOpt, Debug)]
pub enum Opt {
    /// Builds the Rom Hack
    #[structopt(name = "build")]
    Build {
        /// Compiles the Rom Hack in Rust's debug mode
        #[structopt(short = "d", long = "debug")]
        debug: bool,
    },
    /// Creates a new Rom Hack with the given name
    #[structopt(name = "new")]
    New { name: String },
}
