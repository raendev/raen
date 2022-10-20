use std::{
    ffi::OsStr,
    // TODO: make PR to cargo fmt to fix the following line to just `fs,`
    fs::{self},
    path::{Path, PathBuf},
    process::Stdio,
};

use anyhow::{bail, Context, Ok, Result};
use bat::PrettyPrinter;
use cargo_metadata::{Package, Target};
use cargo_witgen::Witgen;
use clap::{Args, Parser};
use clap_cargo_extra::ClapCargo;
use filetime::FileTime;
use witme::app::NearCommand;

/// Build tool for NEAR smart contracts
#[derive(Parser, Debug)]
// #[clap(author = "Willem Wyndham <willem@ahalabs.dev>")]
pub struct Raen {
    #[clap(subcommand)]
    pub top_level_command: TopLevelCommand,
}

#[derive(Parser, Debug)]
pub enum TopLevelCommand {
    /// Build near contract and embed ACI into binary
    Build(Build),
}

#[derive(Args, Debug)]
pub struct Build {
    // Cargo related options
    #[clap(flatten)]
    cargo: ClapCargo,

    /// Do not include the sdk types
    #[clap(long)]
    no_sdk: bool,

    /// Include the types for contract standards
    #[clap(long)]
    standards: bool,

    /// Compile release contract build (default is debug)
    #[clap(long)]
    release: bool,

    /// Only print build file path
    #[clap(long, short = 'q')]
    quiet: bool,
}

impl Raen {
    pub fn run(self) -> Result<()> {
        match self.top_level_command {
            TopLevelCommand::Build(command) => command.run(),
        }
    }
}

impl Build {
    pub fn run(&self) -> Result<()> {
        self.compile()?;
        self.cargo
            .current_packages()?
            .into_iter()
            .try_for_each(|p| {
                self.exec_package(p)
                    .with_context(|| format!("Failed to build package: {}", p.name))
                    .map_err(|e| {
                        eprintln!("{e}");
                        e
                    })
            })?;
        Ok(())
    }

    pub fn exec_package(&self, p: &Package) -> Result<()> {
        if !self.quiet {
            println!("Building {}", p.name);
        }
        if p.targets.is_empty() {
            bail!("no targets in package {}", p.name)
        }
        let target = &p.targets[0];
        if self.should_rebuild(target).unwrap_or(true) {
            self.generate_wit(target)?;
            self.generate_json(target)?;
            self.inject_binary(target)?;
        } else if self.quiet {
            println!(
                "{}",
                self.output_bin(&Self::bin_name(target))?.to_string_lossy()
            );
        }
        Ok(())
    }

    pub fn wit_out_dir(&self, t: &Target) -> Result<PathBuf> {
        Ok(self
            .cargo
            .target_dir()?
            .join("wit")
            .join(t.name.replace('-', "_")))
    }

    pub fn generate_wit(&self, t: &Target) -> Result<()> {
        let output_dir = self.wit_out_dir(t)?;
        let output = output_dir.join("index.wit");
        let cmd = NearCommand::Wit {
            typescript: Some(output_dir),
            sdk: !self.no_sdk,
            standards: self.standards,
            witgen: Witgen {
                input: Some(t.src_path.clone().into()),
                input_dir: PathBuf::new(),
                output,
                prefix_file: vec![],
                prefix_string: vec![],
                stdout: false,
            },
        };
        // Todo improve error reporting
        cmd.run().map_err(|err| {
            eprintln!("\nAdd 'witgen' as a dependency to add to type definition. e.g.\n");
            let input = r#"use witgen::witgen;

/// Type exposed by contract API
#[witgen]
struct Foo {}

"#;

            PrettyPrinter::new()
                .input_from_bytes(input.as_bytes())
                .language("rust")
                .line_numbers(true)
                .print()
                .unwrap_or(true);
            err
        })
    }

    pub fn generate_json(&self, t: &Target) -> Result<()> {
        let out_dir = self.wit_out_dir(t)?;
        let input = out_dir.join("index.ts");
        let cmd = NearCommand::Json {
            input,
            out_dir,
            args: vec![],
        };
        cmd.run()
    }

    fn release_or_debug(&self) -> &str {
        if self.release {
            "release"
        } else {
            "debug"
        }
    }

    pub fn inject_binary(&self, t: &Target) -> Result<()> {
        let output_dir = self.wit_out_dir(t)?;
        let compressed_data = compress_file(&output_dir.join("index.schema.json"))?;
        let file = output_dir.join("index.schema.json.br");
        fs::write(&file, compressed_data).map_err(anyhow::Error::from)?;
        let bin_name = Self::bin_name(t);
        let bin_dir = self.bin_dir()?;
        fs::create_dir_all(&bin_dir)?;
        let cmd = NearCommand::Inject {
            input: self.built_bin(&bin_name)?,
            output: self.output_bin(&bin_name)?,
            data: None,
            file: Some(file),
            name: "json".to_string(),
        };
        cmd.run()?;
        let output = format!("{:?}", bin_dir.join(bin_name));
        let output = output.trim_matches('"');
        if self.quiet {
            println!("{}", output);
        } else {
            println!("Built to:\n{}", output);
        }
        Ok(())
    }

    pub fn compile(&self) -> Result<()> {
        let mut cmd = ClapCargo::cargo_cmd();
        cmd.env("RUSTFLAGS", "-C link-args=-s");
        if self.release {
            cmd.arg("+nightly");
        }
        cmd.arg("build");
        cmd.arg("--target");
        cmd.arg("wasm32-unknown-unknown");
        if self.release {
            cmd.arg("--release");
            cmd.arg("-Z=build-std=std,panic_abort");
            cmd.arg("-Z=build-std-features=panic_immediate_abort");
        }
        self.cargo.add_cargo_args(&mut cmd);
        if !self.quiet {
            cmd.stdout(Stdio::inherit());
            cmd.stderr(Stdio::inherit());
        } else {
            cmd.stderr(Stdio::null());
            cmd.stdout(Stdio::null());
        }
        let status = cmd.status()?;
        if !status.success() {
            bail!(
                "Failed Command:\n{:?} {:?}",
                cmd.get_program(),
                cmd.get_args()
                    .map(OsStr::to_string_lossy)
                    .collect::<Vec<_>>()
                    .join(" ")
            );
        }
        Ok(())
    }

    pub fn bin_name(target: &Target) -> String {
        format!("{}.wasm", target.name.replace('-', "_"))
    }

    pub fn output_bin(&self, bin_name: &str) -> Result<PathBuf> {
        let bin_dir = self.bin_dir()?;
        Ok(bin_dir.join(bin_name))
    }

    pub fn bin_dir(&self) -> Result<PathBuf> {
        Ok(self.cargo.target_dir()?.join("res"))
    }

    pub fn built_bin(&self, bin_name: &str) -> Result<PathBuf> {
        Ok(self
            .cargo
            .target_dir()?
            .join("wasm32-unknown-unknown")
            .join(self.release_or_debug())
            .join(&bin_name))
    }

    pub fn should_rebuild(&self, t: &Target) -> Result<bool> {
        let bin_name = &Self::bin_name(t);
        let cargo_bin = &self.built_bin(bin_name)?;
        let output_bin = &self.output_bin(bin_name)?;

        Ok(get_time(output_bin)? < get_time(cargo_bin)?)
    }
}

fn get_time(path: &Path) -> Result<FileTime> {
    Ok(FileTime::from_last_modification_time(
        &fs::metadata(path).context(format!("failed to access {}", path.to_string_lossy()))?,
    ))
}

fn compress_file(p: &Path) -> Result<Vec<u8>> {
    let buf = fs::read(p)
        .map_err(anyhow::Error::from)
        .with_context(|| format!("{}", p.to_string_lossy()))?
        .into_boxed_slice();
    let mut out = Vec::<u8>::new();
    let params = brotli::enc::BrotliEncoderParams {
        quality: 11,
        ..Default::default()
    };

    brotli::BrotliCompress(&mut buf.as_ref(), &mut out, &params)?;
    Ok(out)
}
