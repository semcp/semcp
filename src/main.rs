use anyhow::Result;
use clap::Parser;
use snpx::{ImageVariants, PolicyConfig, SnpxRunner};
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

    #[arg(long = "image", help = "Docker image to use (default: node:24-alpine)")]
    image: Option<String>,

    #[arg(long = "alpine", help = "Use Alpine image (~180MB)")]
    alpine: bool,

    #[arg(long = "slim", help = "Use slim image (~250MB)")]
    slim: bool,

    #[arg(long = "standard", help = "Use standard image (~1.1GB)")]
    standard: bool,

    #[arg(long = "distroless", help = "Use distroless image (~200MB)")]
    distroless: bool,

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

    #[arg(long = "policy", help = "Path to policy file")]
    policy: Option<String>,

    #[arg(help = "The package and arguments to execute")]
    package_args: Vec<String>,
}

fn determine_image(args: &Args) -> String {
    if let Some(ref custom_image) = args.image {
        custom_image.clone()
    } else if args.alpine {
        ImageVariants::ALPINE.to_string()
    } else if args.slim {
        ImageVariants::SLIM.to_string()
    } else if args.standard {
        ImageVariants::STANDARD.to_string()
    } else if args.distroless {
        ImageVariants::DISTROLESS.to_string()
    } else {
        ImageVariants::get_recommended().to_string()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    if args.package_args.is_empty() {
        eprintln!("Error: No package specified");
        std::process::exit(1);
    }

    let docker_image = determine_image(&args);

    if args.verbose {
        eprintln!("Using Docker image: {}", docker_image);
    }

    let policy_config = if let Some(ref policy_path) = args.policy {
        if args.verbose {
            eprintln!("Loading policy from: {}", policy_path);
        }
        PolicyConfig::from_file(policy_path)?
    } else {
        PolicyConfig::new()
    };

    let runner = SnpxRunner::with_policy(docker_image, args.verbose, policy_config);

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
