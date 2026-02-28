use clap::Parser;
use skills_sync::cli::Cli;
use skills_sync::manifest;

fn main() {
    // Install miette error handler for fancy diagnostics
    let _ = miette::set_hook(Box::new(|_| {
        Box::new(
            miette::MietteHandlerOpts::new()
                .terminal_links(true)
                .unicode(true)
                .context_lines(2)
                .build(),
        )
    }));

    let cli = Cli::parse();

    if let Err(err) = run(cli) {
        eprintln!("{:?}", err);
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> skills_sync::error::Result<()> {
    if cli.init {
        // Will be implemented in Plan 03
        eprintln!("--init not yet implemented");
        return Ok(());
    }

    // Discover manifest (placeholder — full discovery in Plan 03)
    let manifest_path = if let Some(ref config) = cli.config {
        config.clone()
    } else {
        std::path::PathBuf::from(".skills.toml")
    };

    if !manifest_path.exists() {
        return Err(skills_sync::error::SkillsSyncError::ManifestNotFound {
            searched: vec![manifest_path],
        }
        .into());
    }

    let content =
        std::fs::read_to_string(&manifest_path).map_err(skills_sync::error::SkillsSyncError::Io)?;
    let _manifest = manifest::parse_and_validate(&content, &manifest_path)?;

    Ok(())
}
