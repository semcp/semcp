use anyhow::Result;
use clap::Parser;
use snpx::NpxRunner;
use std::env;

#[derive(Parser)]
#[command(
    name = "snpx",
    about = "A containerized replacement for npx",
    version = env!("CARGO_PKG_VERSION")
)]
struct Args {
    #[arg(long, help = "Use verbose output")]
    verbose: bool,

    #[arg(long = "image", help = "Docker image to use (default: node:24)")]
    image: Option<String>,

    #[arg(short = 'y', help = "Automatically answer yes when prompted")]
    yes: bool,

    #[arg(short = 'p', long = "package", help = "Package to execute from")]
    package: Option<String>,

    #[arg(short = 'c', long = "call", help = "Execute the command in a shell")]
    call: Option<String>,

    #[arg(long = "no-install", help = "Skip package installation")]
    no_install: bool,

    #[arg(long = "ignore-existing", help = "Ignore existing commands")]
    ignore_existing: bool,

    #[arg(short = 'q', long = "quiet", help = "Suppress npm logs")]
    quiet: bool,

    #[arg(long = "shell", help = "Use custom shell")]
    shell: Option<String>,

    #[arg(help = "The package and arguments to execute")]
    package_args: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    if args.package_args.is_empty() {
        eprintln!("Error: No package specified");
        std::process::exit(1);
    }

    let docker_image = args.image.unwrap_or_else(|| "node:24".to_string());

    let runner = NpxRunner::new(docker_image, args.verbose);

    let mut npx_flags = Vec::new();

    if args.yes {
        npx_flags.push("-y".to_string());
    } else if !args.no_install {
        npx_flags.push("-y".to_string());
    }

    if let Some(package) = &args.package {
        npx_flags.push("-p".to_string());
        npx_flags.push(package.clone());
    }

    if let Some(call) = &args.call {
        npx_flags.push("-c".to_string());
        npx_flags.push(call.clone());
    }

    if args.no_install {
        npx_flags.push("--no-install".to_string());
    }

    if args.ignore_existing {
        npx_flags.push("--ignore-existing".to_string());
    }

    if args.quiet {
        npx_flags.push("-q".to_string());
    }

    if let Some(shell) = &args.shell {
        npx_flags.push("--shell".to_string());
        npx_flags.push(shell.clone());
    }

    let result = if runner.check_docker_available()? {
        if args.verbose {
            eprintln!("Docker is available, using containerized execution");
        }
        runner
            .run_containerized_npx_with_flags(&npx_flags, &args.package_args)
            .await
    } else {
        runner.run_fallback_npx_with_flags(&npx_flags, &args.package_args)
    };

    match result {
        Ok(status) => {
            if let Some(code) = status.code() {
                std::process::exit(code);
            } else {
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
