use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AndroidError {
    #[error("Android NDK not found in standard paths or ANDROID_NDK_ROOT")]
    NdkNotFound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AndroidAbi {
    Arm64V8a,
    ArmeabiV7a,
    X86_64,
    X86,
}

impl AndroidAbi {
    pub fn triple(&self, api_level: u32) -> String {
        match self {
            AndroidAbi::Arm64V8a => format!("aarch64-linux-android{}", api_level),
            AndroidAbi::ArmeabiV7a => format!("armv7a-linux-androideabi{}", api_level),
            AndroidAbi::X86_64 => format!("x86_64-linux-android{}", api_level),
            AndroidAbi::X86 => format!("i686-linux-android{}", api_level),
        }
    }
}

pub struct AndroidNdk {
    pub root: PathBuf,
}

impl AndroidNdk {
    pub fn discover() -> Result<Self, AndroidError> {
        let candidates = [
            PathBuf::from(r"C:\Users\User\Documents\globals\android-ndk-r30"),
            std::env::var("ANDROID_NDK_ROOT").map(PathBuf::from).unwrap_or_default(),
            std::env::var("ANDROID_NDK_HOME").map(PathBuf::from).unwrap_or_default(),
        ];

        for c in &candidates {
            if c.exists() && c.join("toolchains").exists() {
                return Ok(Self { root: c.clone() });
            }
        }

        Err(AndroidError::NdkNotFound)
    }

    pub fn llvm_bin_dir(&self) -> PathBuf {
        self.root.join(r"toolchains\llvm\prebuilt\windows-x86_64\bin")
    }

    pub fn clang_compiler(&self, abi: AndroidAbi, api_level: u32) -> PathBuf {
        let triple = abi.triple(api_level);
        self.llvm_bin_dir().join(format!("{}-clang.cmd", triple))
    }
}
