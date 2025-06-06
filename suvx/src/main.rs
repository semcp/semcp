use anyhow::Result;
use clap::Parser;
use semcp_common::{ContainerExecutor, ImageVariants, PolicyConfig, Runner, Transport};
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

    #[arg(
        long = "image",
        help = "Docker image to use (default: python:3.12-alpine)"
    )]
    image: Option<String>,

    #[arg(long = "alpine", help = "Use Alpine image (~200MB)")]
    alpine: bool,

    #[arg(long = "slim", help = "Use slim image (~300MB)")]
    slim: bool,

    #[arg(long = "standard", help = "Use standard image (~1GB)")]
    standard: bool,

    #[arg(short = 'p', long = "python", help = "Python interpreter to use")]
    python: Option<String>,

    #[arg(long = "from", help = "Install the command from a different package")]
    from_package: Option<String>,

    #[arg(
        long = "with",
        help = "Install additional packages alongside the main package"
    )]
    with_packages: Vec<String>,

    #[arg(
        long = "with-editable",
        help = "Install additional packages in editable mode"
    )]
    with_editable: Vec<String>,

    #[arg(long = "index", help = "Base URL of Python package index")]
    index: Option<String>,

    #[arg(long = "index-url", help = "Base URL of Python package index")]
    index_url: Option<String>,

    #[arg(long = "extra-index-url", help = "Extra URLs of package indexes")]
    extra_index_url: Vec<String>,

    #[arg(long = "find-links", help = "Additional sources for packages")]
    find_links: Vec<String>,

    #[arg(long = "no-index", help = "Ignore package index, only use find-links")]
    no_index: bool,

    #[arg(long = "prerelease", help = "Allow pre-release versions")]
    prerelease: bool,

    #[arg(long = "upgrade", help = "Allow package upgrades")]
    upgrade: bool,

    #[arg(long = "force-reinstall", help = "Force reinstall packages")]
    force_reinstall: bool,

    #[arg(long = "no-deps", help = "Don't install dependencies")]
    no_deps: bool,

    #[arg(long = "policy", help = "Path to policy file")]
    policy: Option<String>,

    #[arg(trailing_var_arg = true, help = "arguments to execute")]
    package_args: Vec<String>,
}

struct SuvxRunner {
    executor: ContainerExecutor,
}

impl SuvxRunner {
    pub fn with_policy(docker_image: String, verbose: bool, policy_config: PolicyConfig) -> Self {
        Self {
            executor: ContainerExecutor::with_policy(docker_image, verbose, policy_config),
        }
    }

    pub fn check_docker_available(&self) -> Result<bool> {
        self.executor.check_docker_available()
    }

    pub async fn run_containerized_uvx_with_flags(
        &self,
        uvx_flags: &[String],
        uvx_args: &[String],
    ) -> Result<std::process::ExitStatus> {
        self.executor
            .run_containerized(self, uvx_flags, uvx_args)
            .await
    }
}

impl Runner for SuvxRunner {
    fn command(&self) -> &str {
        "uvx"
    }

    fn default_image(&self) -> &str {
        ImageVariants::get_python_recommended()
    }

    fn default_flags(&self) -> Vec<String> {
        vec![]
    }

    fn detect_transport(&self, _package: &str) -> Transport {
        Transport::Stdio
    }

    fn requires_tty(&self, transport: &Transport) -> bool {
        matches!(transport, Transport::Http | Transport::SSE)
    }
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
        ImageVariants::get_python_recommended().to_string()
    }
}

fn build_uvx_flags(args: &Args) -> Vec<String> {
    let mut flags = Vec::new();

    if let Some(ref python) = args.python {
        flags.push("--python".to_string());
        flags.push(python.clone());
    }

    if let Some(ref from_pkg) = args.from_package {
        flags.push("--from".to_string());
        flags.push(from_pkg.clone());
    }

    for with_pkg in &args.with_packages {
        flags.push("--with".to_string());
        flags.push(with_pkg.clone());
    }

    for with_edit in &args.with_editable {
        flags.push("--with-editable".to_string());
        flags.push(with_edit.clone());
    }

    if let Some(ref index) = args.index {
        flags.push("--index".to_string());
        flags.push(index.clone());
    }

    if let Some(ref index_url) = args.index_url {
        flags.push("--index-url".to_string());
        flags.push(index_url.clone());
    }

    for extra_url in &args.extra_index_url {
        flags.push("--extra-index-url".to_string());
        flags.push(extra_url.clone());
    }

    for find_link in &args.find_links {
        flags.push("--find-links".to_string());
        flags.push(find_link.clone());
    }

    if args.no_index {
        flags.push("--no-index".to_string());
    }

    if args.prerelease {
        flags.push("--prerelease".to_string());
        flags.push("allow".to_string());
    }

    if args.upgrade {
        flags.push("--upgrade".to_string());
    }

    if args.force_reinstall {
        flags.push("--force-reinstall".to_string());
    }

    if args.no_deps {
        flags.push("--no-deps".to_string());
    }

    flags
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

    let runner = SuvxRunner::with_policy(docker_image, args.verbose, policy_config);

    let uvx_flags = build_uvx_flags(&args);

    if !runner.check_docker_available()? {
        eprintln!("Docker is not available or not running");
        eprintln!("suvx requires Docker to be installed and running");
        std::process::exit(1);
    }

    let result = runner
        .run_containerized_uvx_with_flags(&uvx_flags, &args.package_args)
        .await;

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
