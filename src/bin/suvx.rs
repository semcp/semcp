use anyhow::Result;
use clap::Parser;
use snpx::{ImageVariants, SuvRunner};
use std::env;

#[derive(Parser)]
#[command(
    name = "suvx",
    about = "A containerized replacement for uvx",
    version = env!("CARGO_PKG_VERSION")
)]
struct Args {
    #[arg(long, help = "Use verbose output")]
    verbose: bool,

    #[arg(long = "image", help = "Docker image to use (default: python:3.12-alpine)")]
    image: Option<String>,

    #[arg(long = "alpine", help = "Use Alpine image (~80MB)")]
    alpine: bool,

    #[arg(long = "slim", help = "Use slim image (~150MB)")]
    slim: bool,

    #[arg(long = "standard", help = "Use standard image (~350MB)")]
    standard: bool,

    #[arg(help = "The package and arguments to execute")]
    package_args: Vec<String>,
}

fn determine_image(args: &Args) -> String {
    if let Some(ref custom_image) = args.image {
        custom_image.clone()
    } else if args.alpine {
        ImageVariants::PYTHON_ALPINE.to_string()
    } else if args.slim {
        ImageVariants::PYTHON_SLIM.to_string()
    } else if args.standard {
        ImageVariants::PYTHON_STANDARD.to_string()
    } else {
        ImageVariants::get_recommended_python().to_string()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    if args.package_args.is_empty() {
        eprintln!("Error: No command specified");
        std::process::exit(1);
    }

    let docker_image = determine_image(&args);

    if args.verbose {
        eprintln!("Using Docker image: {}", docker_image);
    }

    let runner = SuvRunner::new(docker_image, args.verbose);

    let result = if runner.check_docker_available()? {
        if args.verbose {
            eprintln!("Docker is available, using containerized execution");
        }
        runner
            .run_containerized_uv(&args.package_args)
            .await
    } else {
        if args.verbose {
            eprintln!("Docker is not available, falling back to regular uvx");
        }
        // For fallback, change the command from uv to uvx
        let mut uvx_args = vec!["x".to_string()];
        uvx_args.extend(args.package_args.iter().cloned());
        runner.run_fallback_uv(&uvx_args)
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