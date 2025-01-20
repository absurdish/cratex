use indicatif::{ProgressBar, ProgressStyle};
use std::{
    error::Error,
    io::{BufRead, BufReader},
    process::{Command, Stdio},
};
use tempfile::TempDir;

pub struct Cratex {
    temp_dir: TempDir,
    crate_name: String,
    version: Option<String>,
}

impl Cratex {
    pub fn new(crate_name: &str, version: Option<String>) -> Result<Self, Box<dyn Error>> {
        let temp_dir = tempfile::tempdir()?;
        Ok(Self {
            temp_dir,
            crate_name: crate_name.to_string(),
            version,
        })
    }

    pub fn install_and_run(&self, args: Vec<String>) -> Result<(), Box<dyn Error>> {
        let pb = ProgressBar::new(100);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.yellow} [{bar:40.red/orange}] {pos}% {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );

        pb.set_message("creating directories...");
        pb.set_position(10);
        let cargo_home = self.temp_dir.path().join(".cargo");
        std::fs::create_dir_all(&cargo_home)?;

        pb.set_message("preparing installation...");
        pb.set_position(20);
        let mut install_args = vec!["install", &self.crate_name];
        if let Some(version) = &self.version {
            install_args.extend(["--version", version]);
        }
        install_args.extend(["--root", self.temp_dir.path().to_str().unwrap()]);

        pb.set_message("installing crate...");
        pb.set_position(30);

        let mut child = Command::new("cargo")
            .env("CARGO_HOME", &cargo_home)
            .args(&install_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let mut stderr = BufReader::new(child.stderr.take().unwrap());
        let mut compiled = false;
        let mut line = String::new();

        while stderr.read_line(&mut line)? > 0 {
                  if line.contains("Downloaded") {
                      pb.set_position(50);
                      pb.set_message("dependencies downloaded");
                  } else if line.contains("Compiling") {
                      if !compiled {
                          compiled = true;
                          pb.set_position(60);
                          pb.set_message("compiling dependencies");
                      }
                  } else if line.contains("Building") {
                      pb.set_position(70);
                      pb.set_message("building final binary");
                  }
                  line.clear();
              }

        let status = child.wait()?;
        if !status.success() {
            pb.finish_with_message("installation failed!");
            return Err("failed to install crate".into());
        }

        pb.set_message("preparing to run...");
        pb.set_position(80);
        let bin_path = self.temp_dir.path().join("bin");
        let binary = bin_path.join(&self.crate_name);

        pb.set_message("running binary...");
        pb.set_position(90);
        let status = Command::new(binary).args(&args).status()?;

        if !status.success() {
            pb.finish_with_message("execution failed!");
            return Err("failed to run binary".into());
        }

        pb.finish_with_message("complete!");
        Ok(())
    }
}

pub fn run(
    crate_name: &str,
    version: Option<String>,
    args: Vec<String>,
) -> Result<(), Box<dyn Error>> {
    let cratex = Cratex::new(crate_name, version)?;
    cratex.install_and_run(args)
}
