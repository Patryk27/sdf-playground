use log::{error, info};
use spirv_builder::{MetadataPrintout, SpirvBuilder};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime};

#[derive(Debug)]
pub struct Compiler {
    rx: mpsc::Receiver<PathBuf>,
}

impl Compiler {
    pub fn spawn() -> Self {
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let mut previous_modified_at: Option<
                SystemTime,
            > = None;

            loop {
                let crate_dir =
                    Path::new(env!("CARGO_MANIFEST_DIR"))
                        .parent()
                        .unwrap()
                        .join("shader");

                let modified_at = crate_dir
                    .join("src")
                    .join("lib.rs")
                    .metadata()
                    .unwrap()
                    .modified()
                    .unwrap();

                if previous_modified_at
                    .map_or(true, |p| p != modified_at)
                {
                    info!("Compiling shader");

                    let shader_path = SpirvBuilder::new(
                        crate_dir,
                        "spirv-unknown-vulkan1.1",
                    )
                    .print_metadata(MetadataPrintout::None)
                    .build()
                    .map(|result| {
                        result
                            .module
                            .unwrap_single()
                            .to_owned()
                    });

                    if let Ok(shader_path) = shader_path {
                        _ = tx.send(shader_path);
                    } else {
                        error!("Compilation failed");
                    }

                    previous_modified_at =
                        Some(modified_at);
                } else {
                    thread::sleep(Duration::from_millis(5));
                }
            }
        });

        Self { rx }
    }

    pub fn poll(&self) -> Option<PathBuf> {
        self.rx.try_recv().ok()
    }
}
