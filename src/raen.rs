use once_cell::sync::OnceCell;
use std::{
    env,
    // TODO: make PR to cargo fmt to fix the following line to just `fs,`
    fs::{self},
    path::{Path, PathBuf},
    process::Stdio,
};

use anyhow::{bail, Ok, Result};
use cargo_metadata::{Metadata, Package, Target};
use cargo_witgen::Witgen;
use clap::{Args, Parser};
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
    manifest: clap_cargo::Manifest,
    #[clap(flatten)]
    workspace: clap_cargo::Workspace,
    #[clap(flatten)]
    features: clap_cargo::Features,

    /// Do not include the sdk types
    #[clap(long)]
    no_sdk: bool,

    /// Include the types for contract standards
    #[clap(long)]
    standards: bool,

    /// Compile release contract build (default is debug)
    #[clap(long)]
    release: bool,
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
        self.packages()?
            .into_iter()
            .try_for_each(|p| self.exec_package(p))?;
        Ok(())
    }
    pub fn metadata(&self) -> Result<&Metadata> {
        static INSTANCE: OnceCell<Metadata> = OnceCell::new();
        INSTANCE.get_or_try_init(|| {
            let mut metadata = self.manifest.metadata();
            self.features.forward_metadata(&mut metadata);
            metadata.exec().map_err(Into::into)
        })
    }

    pub fn target_dir(&self) -> Result<PathBuf> {
        Ok(self.metadata()?.target_directory.clone().into())
    }

    pub fn packages(&self) -> Result<Vec<&Package>> {
        let meta = self.metadata()?;
        Ok(self.workspace.partition_packages(meta).0)
    }

    pub fn exec_package(&self, p: &Package) -> Result<()> {
        if p.targets.is_empty() {
            bail!("no targets in package")
        } else {
            let target = &p.targets[0];
            self.generate_wit(target)?;
            self.generate_json(target)?;
            self.inject_binary(target)
        }
    }

    pub fn wit_out_dir(&self, t: &Target) -> Result<PathBuf> {
        Ok(self
            .target_dir()?
            .join("wit")
            .join(t.name.clone().replace('-', "_")))
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
        cmd.run()
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
        let bin_name = format!("{}.wasm", t.name.replace('-', "_"));
        let bin_dir = self.target_dir()?.join("res");
        fs::create_dir_all(&bin_dir)?;
        let cmd = NearCommand::Inject {
            input: self
                .target_dir()?
                .join("wasm32-unknown-unknown")
                .join(self.release_or_debug())
                .join(&bin_name),
            output: bin_dir.join(bin_name),
            data: None,
            file: Some(file),
            name: "json".to_string(),
        };
        cmd.run()
    }

    pub fn compile(&self) -> Result<()> {
        let mut cmd = std::process::Command::new(cargo());
        cmd.arg("build");
        cmd.arg("--target");
        cmd.arg("wasm32-unknown-unknown");
        if self.release {
            cmd.arg("--release");
        }
        if let Some(manifest_path) = &self.manifest.manifest_path {
            cmd.arg("--manifest-path");
            cmd.arg(manifest_path);
        }
        if self.features.no_default_features {
            cmd.arg("--no-default-features");
        }
        if self.features.all_features {
            cmd.arg("--all-features");
        } else {
            for feature in &self.features.features {
                cmd.arg("--features");
                cmd.arg(feature);
            }
        }
        for pack in &self.workspace.exclude {
            cmd.arg("--exclude");
            cmd.arg(pack);
        }
        if self.workspace.workspace || self.workspace.all {
            cmd.arg("--workspace");
        } else if !self.workspace.package.is_empty() {
            self.workspace.package.iter().for_each(|p| {
                cmd.arg("-p");
                cmd.arg(p);
            })
        }
        println!("{:#?}", cmd);

        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());
        let status = cmd.status()?;
        if !status.success() {
            bail!(
                "{:?}      {:?}",
                cmd.get_program(),
                cmd.get_args()
                    .map(|s| s.to_str().unwrap())
                    .collect::<Vec<&str>>()
                    .join(" ")
            );
        }
        Ok(())
    }
}

fn cargo() -> String {
    env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned())
}

fn compress_file(p: &Path) -> Result<Vec<u8>> {
    let buf = fs::read(p).map_err(anyhow::Error::from)?.into_boxed_slice();
    let mut out = Vec::<u8>::new();
    let params = brotli::enc::BrotliEncoderParams {
        quality: 11,
        ..Default::default()
    };

    brotli::BrotliCompress(&mut buf.as_ref(), &mut out, &params)?;
    Ok(out)
}
