use camino::Utf8PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BundlePlatform {
    Macos,
    WindowsX64,
    LinuxX64,
}

pub fn binary_relative_path(platform: BundlePlatform, plugin_name: &str) -> Utf8PathBuf {
    match platform {
        BundlePlatform::Macos => Utf8PathBuf::from(format!("Contents/MacOS/{plugin_name}")),
        BundlePlatform::WindowsX64 => {
            Utf8PathBuf::from(format!("Contents/x86_64-win/{plugin_name}.vst3"))
        }
        BundlePlatform::LinuxX64 => {
            Utf8PathBuf::from(format!("Contents/x86_64-linux/{plugin_name}.so"))
        }
    }
}
