#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "windows")]
use windows as dirs;

#[cfg(any(target_os = "macos", target_os = "ios"))]
mod macos;

#[cfg(any(target_os = "macos", target_os = "ios"))]
use macos as dirs;

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "ios")))]
mod linux;

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "ios")))]
use linux as dirs;

use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Clone, Debug)]
pub struct BaseDirs {
    home: PathBuf,
    cache: PathBuf,
    config: PathBuf,
    config_local: PathBuf,
    data: PathBuf,
    data_local: PathBuf,
    executable: Option<PathBuf>,
    preference: Option<PathBuf>,
    runtime: Option<PathBuf>,
    state: Option<PathBuf>,
}

impl BaseDirs {
    pub fn new() -> Result<Self, DirsError> {
        dirs::base_dirs().ok_or(DirsError::NoHomeDir)
    }

    pub fn home_dir(&self) -> &Path {
        self.home.as_path()
    }

    pub fn cache_dir(&self) -> &Path {
        self.cache.as_path()
    }

    pub fn config_dir(&self) -> &Path {
        self.config.as_path()
    }

    pub fn config_local_dir(&self) -> &Path {
        self.config_local.as_path()
    }

    pub fn data_dir(&self) -> &Path {
        self.data.as_path()
    }

    pub fn data_local_dir(&self) -> &Path {
        self.data_local.as_path()
    }

    pub fn executable(&self) -> Option<&Path> {
        self.executable.as_deref()
    }

    pub fn preference(&self) -> Option<&Path> {
        self.preference.as_deref()
    }

    pub fn runtime(&self) -> Option<&Path> {
        self.runtime.as_deref()
    }

    pub fn state(&self) -> Option<&Path> {
        self.state.as_deref()
    }
}

#[derive(Clone, Debug)]
pub struct UserDirs {
    home: PathBuf,
    audio: Option<PathBuf>,
    desktop: Option<PathBuf>,
    document: Option<PathBuf>,
    download: Option<PathBuf>,
    font: Option<PathBuf>,
    picture: Option<PathBuf>,
    public: Option<PathBuf>,
    template: Option<PathBuf>,
    video: Option<PathBuf>,
}

impl UserDirs {
    pub fn new() -> Result<Self, DirsError> {
        dirs::user_dirs().ok_or(DirsError::NoHomeDir)
    }

    pub fn home_dir(&self) -> &Path {
        self.home.as_path()
    }

    pub fn audio_dir(&self) -> Option<&Path> {
        self.audio.as_deref()
    }

    pub fn desktop_dir(&self) -> Option<&Path> {
        self.desktop.as_deref()
    }

    pub fn document_dir(&self) -> Option<&Path> {
        self.document.as_deref()
    }

    pub fn download_dir(&self) -> Option<&Path> {
        self.download.as_deref()
    }

    pub fn font_dir(&self) -> Option<&Path> {
        self.font.as_deref()
    }

    pub fn picture_dir(&self) -> Option<&Path> {
        self.picture.as_deref()
    }

    pub fn public_dir(&self) -> Option<&Path> {
        self.public.as_deref()
    }

    pub fn template_dir(&self) -> Option<&Path> {
        self.template.as_deref()
    }

    pub fn video_dir(&self) -> Option<&Path> {
        self.video.as_deref()
    }
}

#[derive(Debug, Eq, PartialEq, Error)]
pub enum DirsError {
    #[error("The Home directory is not defined")]
    NoHomeDir,
}
