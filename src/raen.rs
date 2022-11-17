use std::{
    collections::HashSet,
    ffi::OsStr,
    // TODO: make PR to cargo fmt to fix the following line to just `fs,`
    fs::{self},
    path::PathBuf,
    process::Stdio,
};

use anyhow::{bail, Context, Ok, Result};
use bat::PrettyPrinter;
use cargo_metadata::{camino::Utf8PathBuf, DependencyKind, Package, Target};
use cargo_witgen::Witgen;
use clap::{Args, Parser};
use clap_cargo_extra::{ClapCargo, TargetTools};
use witme::app::NearCommand;

use crate::ext::{compress_file, get_time, PackageExt};

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

#[derive(Args, Debug, Default)]
pub struct Build {
    // Cargo related options
    #[clap(flatten)]
    pub cargo: ClapCargo,

    /// Do not include the sdk types
    #[clap(long)]
    pub no_sdk: bool,

    /// Include the types for contract standards
    #[clap(long)]
    pub standards: bool,

    /// Only print build file path
    #[clap(long, short = 'q')]
    pub quiet: bool,
}

impl Raen {
    pub fn run(self) -> Result<()> {
        match self.top_level_command {
            TopLevelCommand::Build(mut command) => command.run(),
        }
    }
}

impl Build {
    pub fn run(&mut self) -> Result<()> {
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
            self.generate_wit_from_target(target, p)?;
            self.generate_json(target)?;
            self.inject_binary(target)?;
        } else if self.quiet {
            println!("{}", self.output_bin(target)?.to_string_lossy());
        }
        Ok(())
    }

    pub fn wit_out_dir(&self, t: &Target) -> Result<PathBuf> {
        self.wit_out_dir_str(&t.name)
    }

    pub fn wit_out_dir_str(&self, name: &str) -> Result<PathBuf> {
        Ok(self
            .cargo
            .target_dir()?
            .join("wit")
            .join(name.replace('-', "_")))
    }

    pub fn generate_wit_from_target(&self, target: &Target, p: &Package) -> Result<()> {
        let output_dir = self.wit_out_dir(target)?;
        let input_src = target.src_path.clone().into();
        let mut prefix_file = vec![];
        for (input, output_dir) in self.get_witgen_deps(p)? {
            prefix_file.push(output_dir.join("index.wit"));
            if !output_dir.exists() {
                fs::create_dir_all(output_dir.as_path())?;
                self.generate_wit(
                    input.into(),
                    output_dir,
                    None,
                    Some(false),
                    Some(false),
                    false,
                )?;
            }
        }
        self.generate_wit(input_src, output_dir, Some(prefix_file), None, None, true)
    }

    pub fn generate_wit(
        &self,
        input_src: PathBuf,
        output_dir: PathBuf,
        prefix_file: Option<Vec<PathBuf>>,
        no_sdk: Option<bool>,
        standards: Option<bool>,
        typescript: bool,
    ) -> Result<()> {
        let output = output_dir.join("index.wit");
        let cmd = NearCommand::Wit {
            typescript: if typescript { Some(output_dir) } else { None },
            sdk: no_sdk.unwrap_or(!self.no_sdk),
            standards: standards.unwrap_or(self.standards),
            witgen: Witgen {
                input: Some(input_src),
                input_dir: PathBuf::new(),
                output,
                prefix_file: prefix_file.unwrap_or_default(),
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
    pub fn inject_binary(&self, t: &Target) -> Result<()> {
        let output_dir = self.wit_out_dir(t)?;
        let compressed_data = compress_file(&output_dir.join("index.schema.json"))?;
        let file = output_dir.join("index.schema.json.br");
        fs::write(&file, compressed_data).map_err(anyhow::Error::from)?;
        let bin_name = Self::bin_name(t);
        let bin_dir = self.bin_dir()?;
        fs::create_dir_all(&bin_dir)?;
        let input = self.cargo.built_bin(t)?;
        let output = self.output_bin(t)?;
        let cmd = NearCommand::Inject {
            input,
            output,
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

    pub fn compile(&mut self) -> Result<()> {
        self.cargo.cargo_build.target = Some("wasm32-unknown-unknown".to_string());
        self.cargo.cargo_build.link_args = true;
        let mut cmd = self.cargo.build_cmd();
        if self.quiet {
            cmd.stderr(Stdio::null());
            cmd.stdout(Stdio::null());
        } else {
            cmd.stdout(Stdio::inherit());
            cmd.stderr(Stdio::inherit());
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

    pub fn output_bin(&self, target: &Target) -> Result<PathBuf> {
        let bin_dir = self.bin_dir()?;
        Ok(bin_dir.join(target.wasm_bin_name()))
    }

    pub fn bin_dir(&self) -> Result<PathBuf> {
        Ok(self.cargo.target_dir()?.join("res"))
    }

    pub fn should_rebuild(&self, t: &Target) -> Result<bool> {
        let cargo_bin = &self.cargo.built_bin(t)?;
        let output_bin = &self.output_bin(t)?;

        Ok(get_time(output_bin)? < get_time(cargo_bin)?)
    }

    pub fn get_witgen_deps(&self, package: &Package) -> Result<Vec<(Utf8PathBuf, PathBuf)>> {
        self.cargo
            .get_deps(package, DependencyKind::Normal)?
            .into_iter()
            .filter(|p| Package::witgen_dep(p))
            .map(|p| {
                let version = &p.version;
                let name = &p.name;
                let dir_str = format!("{name}{version}");
                let out_dir = self.wit_out_dir_str(&dir_str)?;
                let res = (
                    p.manifest_path
                        .parent()
                        .ok_or_else(|| anyhow::anyhow!("Failed to get parent of {}", p.name))?
                        .join("src")
                        .join("lib.rs"),
                    out_dir,
                );
                Ok(res)
            })
            .collect::<Result<HashSet<_>>>()
            .and_then(|set| Ok(set.into_iter().collect::<Vec<_>>()))
    }
}
