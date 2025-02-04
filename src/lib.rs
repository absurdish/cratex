use beautify::Colors;
use indicatif::{ProgressBar, ProgressStyle};
use std::{
    collections::HashSet,
    error::Error,
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Command, Stdio},
};
use tempfile::TempDir;

pub struct Cratex {
    temp_dir: TempDir,
    crate_name: Box<str>,
    version: Option<Box<str>>,
    cargo_home: PathBuf,
    bin_path: PathBuf,
}

impl Cratex {
    #[inline]
    pub fn new(crate_name: &str, version: Option<String>) -> Result<Self, Box<dyn Error>> {
        let temp_dir = tempfile::tempdir()?;
        let cargo_home = temp_dir.path().join(".cargo");
        let bin_path = temp_dir.path().join("bin");

        Ok(Self {
            temp_dir,
            crate_name: crate_name.into(),
            version: version.map(String::into_boxed_str),
            cargo_home,
            bin_path,
        })
    }

    pub fn install_and_run(&self, args: Vec<String>) -> Result<(), Box<dyn Error>> {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.yellow} [{bar:40.red/orange}] {pos}% {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );

        pb.set_message("preparing environment...");
        pb.set_position(5);
        std::fs::create_dir_all(&self.cargo_home)?;
        pb.set_position(10);

        pb.set_message("preparing installation...");
        let mut install_args = Vec::with_capacity(10);
        install_args.push("install");
        install_args.push(&*self.crate_name);

        if let Some(ref version) = self.version {
            install_args.extend_from_slice(&["--version", version]);
        }

        install_args.extend_from_slice(&[
            "--root",
            self.temp_dir.path().to_str().unwrap(),
            "--jobs",
        ]);
        let cpus = num_cpus::get().to_string();
        install_args.push(cpus.as_str());
        pb.set_position(15);

        pb.set_message("starting installing...");
        pb.set_position(20);
        let mut child = Command::new("cargo")
            .env("CARGO_HOME", &self.cargo_home)
            .env("RUSTC_BOOTSTRAP", "1")
            .env("CARGO_PROFILE_RELEASE_LTO", "thin")
            .env("CARGO_PROFILE_RELEASE_CODEGEN_UNITS", "16")
            .env("RUSTFLAGS", "-C target-cpu=native -C opt-level=2")
            .env("CARGO_NET_GIT_FETCH_WITH_CLI", "true")
            .args(&install_args)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()?;
        pb.set_position(25);

        let stderr = child.stderr.take().unwrap();
        let mut reader = BufReader::with_capacity(8192, stderr);
        let mut line = String::with_capacity(256);
        let mut compiled = false;

        let mut downloading = false;
        let mut downloaded = false;
        let mut compiling = false;
        let mut building = false;
        let mut finishing = false;
        let mut seen_downloads = HashSet::new();
        let mut download_count = 0;

        while reader.read_line(&mut line)? > 0 {
            if line.contains("Downloading") {
                if !downloading {
                    downloading = true;
                    pb.set_position(30);
                }

                if let Some(pkg_info) = line.split("Downloading").nth(1) {
                    let pkg_info = pkg_info.trim();

                    let pkg_details = if pkg_info.contains('v') {
                        pkg_info
                            .trim_end_matches("...")
                            .trim_end_matches(")")
                            .trim()
                    } else {
                        pkg_info.trim_end_matches("...").trim()
                    };

                    if seen_downloads.insert(pkg_details.to_string()) {
                        download_count += 1;

                        let download_type = if pkg_info.contains("index") {
                            "registry index"
                        } else if pkg_info.contains("registry") {
                            "registry cache"
                        } else {
                            "package"
                        };

                        pb.set_message(format!(
                            "downloading [{}/{}] {} ({})",
                            download_count,
                            seen_downloads.len(),
                            pkg_details,
                            download_type
                        ));
                    }
                }
            } else if line.contains("Downloaded") && !downloaded {
                downloaded = true;
                pb.set_message(format!(
                    "downloaded {} packages successfully",
                    download_count
                ));
                pb.set_position(45);
            } else if line.contains("Compiling") && !compiling {
                compiling = true;
                let pkg_info = if let Some(pkg) = line.split("Compiling").nth(1) {
                    pkg.trim()
                        .trim_end_matches("...")
                        .trim_end_matches(")")
                        .trim()
                } else {
                    "packages"
                };
                pb.set_message(format!("compiling {} ...", pkg_info.text_blue()));
                pb.set_position(60);
            } else if line.contains("Compiling") {
                if let Some(pkg) = line.split("Compiling").nth(1) {
                    let pkg_info = pkg
                        .trim()
                        .trim_end_matches("...")
                        .trim_end_matches(")")
                        .trim();
                    pb.set_message(format!("compiling {} ...", pkg_info));
                }
            } else if line.contains("Building") && !building {
                building = true;
                pb.set_message("building binary...");
                pb.set_position(75);
            } else if line.contains("Finished") && !finishing {
                finishing = true;
                pb.set_message("finishing installation...");
                pb.set_position(85);
            }
            line.clear();
        }

        let status = child.wait()?;
        if !status.success() {
            return Err("failed to install crate".into());
        }

        pb.set_message("running binary...");
        pb.set_position(90);

        let status = Command::new(self.bin_path.join(&*self.crate_name))
            .args(args)
            .status()?;
        pb.set_position(95);

        if !status.success() {
            return Err("failed to run binary".into());
        }

        pb.set_position(100);
        pb.finish_with_message("complete!");
        Ok(())
    }
}

#[inline]
pub fn run(
    crate_name: &str,
    version: Option<String>,
    args: Vec<String>,
) -> Result<(), Box<dyn Error>> {
    Cratex::new(crate_name, version)?.install_and_run(args)
}
