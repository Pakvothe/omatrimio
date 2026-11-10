use std::{
    fs,
    path::{Path, PathBuf},
};

/// DRM drivers for emulated display adapters: the GPU a virtual machine shows
/// its guest (QEMU/UTM, VMware, VirtualBox, Hyper-V, Parallels) or the plain
/// framebuffer firmware leaves behind. None of them accelerates WebKit's
/// rasterization well; on virtio-gpu it makes posters flicker on the home board.
const VIRTUAL_DRIVERS: &[&str] = &[
    "virtio_gpu",
    "vmwgfx",
    "qxl",
    "bochs",
    "bochs-drm",
    "cirrus",
    "cirrus-qemu",
    "vboxvideo",
    "hyperv_drm",
    "simpledrm",
];

/// Whether every display adapter is virtual. A VM with a passed-through GPU
/// has a real driver too, and keeps GPU rendering.
pub fn only_virtual_gpus() -> bool {
    let drivers = drm_drivers(Path::new("/sys/class/drm"));
    !drivers.is_empty()
        && drivers
            .iter()
            .all(|driver| VIRTUAL_DRIVERS.contains(&driver.as_str()))
}

/// The driver of each `cardN` device. For a virtio GPU the PCI device is bound
/// to the generic `virtio-pci`, and the GPU driver sits on its `virtioN` child.
fn drm_drivers(class: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(class) else {
        return vec![];
    };

    entries
        .flatten()
        .filter(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            name.starts_with("card") && !name.contains('-')
        })
        .filter_map(|entry| {
            let device = entry.path().join("device");
            let driver = driver_name(&device)?;
            if driver != "virtio-pci" {
                return Some(driver);
            }
            fs::read_dir(&device)
                .ok()?
                .flatten()
                .map(|child| child.path())
                .filter(|path| path_name(path).starts_with("virtio"))
                .find_map(|path| driver_name(&path))
                .or(Some(driver))
        })
        .collect()
}

fn driver_name(device: &Path) -> Option<String> {
    let target: PathBuf = fs::read_link(device.join("driver")).ok()?;
    Some(path_name(&target))
}

fn path_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    fn card(root: &Path, name: &str, driver: &str, virtio_child: Option<&str>) {
        let device = root.join(name).join("device");
        fs::create_dir_all(&device).unwrap();
        symlink(format!("../../bus/pci/drivers/{driver}"), device.join("driver")).unwrap();
        if let Some(child_driver) = virtio_child {
            let child = device.join("virtio2");
            fs::create_dir_all(&child).unwrap();
            symlink(format!("../../bus/virtio/drivers/{child_driver}"), child.join("driver"))
                .unwrap();
        }
    }

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("omatrimio-gpu-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn finds_virtio_gpu_behind_virtio_pci() {
        let root = temp("virtio");
        card(&root, "card0", "virtio-pci", Some("virtio_gpu"));
        fs::create_dir_all(root.join("card0-Virtual-1")).unwrap();
        assert_eq!(drm_drivers(&root), vec!["virtio_gpu"]);
    }

    #[test]
    fn reads_real_drivers() {
        let root = temp("real");
        card(&root, "card0", "amdgpu", None);
        card(&root, "card1", "simpledrm", None);
        let mut drivers = drm_drivers(&root);
        drivers.sort();
        assert_eq!(drivers, vec!["amdgpu", "simpledrm"]);
        assert!(!drivers.iter().all(|d| VIRTUAL_DRIVERS.contains(&d.as_str())));
    }
}
