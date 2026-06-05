#[allow(dead_code)]
pub fn init_cli() -> anyhow::Result<clap::Command> {
    Ok(clap::Command::new("spotify-player-gui"))
}

#[allow(dead_code)]
pub fn handle_cli_subcommand(_cmd: &str, _args: &clap::ArgMatches) -> anyhow::Result<()> {
    Ok(())
}
