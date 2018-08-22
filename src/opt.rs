use std::path::PathBuf;

#[derive(StructOpt, Debug)]
pub enum Opt {
    /// Builds the Rom Hack
    #[structopt(name = "build")]
    Build {
        /// Compiles the Rom Hack in Rust's debug mode
        #[structopt(short = "d", long = "debug")]
        debug: bool,
        /// Compiles the Rom Hack as a patch
        #[structopt(short = "p", long = "patch")]
        patch: bool,
    },
    /// Applies a patch file to a game to create a Rom Hack
    #[structopt(name = "apply")]
    Apply {
        /// Input path to patch file
        #[structopt(name = "PATCH", parse(from_os_str))]
        patch: PathBuf,
        /// Input path to original game (GCM or ISO format)
        #[structopt(name = "ORIGINAL", parse(from_os_str))]
        original_game: PathBuf,
        /// Output path for Rom Hack
        #[structopt(name = "OUT", parse(from_os_str))]
        output: PathBuf,
    },
    /// Creates a new Rom Hack with the given name
    #[structopt(name = "new")]
    New { name: String },
}
