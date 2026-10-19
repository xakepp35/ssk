//! Read-only identity query on an already enumerated Vulkan adapter.
//! No extra instance/device/queue, enumeration, driver loading or GPU work.
use eframe::wgpu;

#[derive(Default)]
pub(crate) struct GpuIdentity {
    pub uuid: Option<String>,
    pub pci: Option<String>,
}

pub(crate) fn query(adapter: &wgpu::Adapter) -> GpuIdentity {
    use ash::{ext, vk};
    let mut identity = GpuIdentity::default();
    // SAFETY: borrow wgpu's Vulkan adapter under its HAL guard. No owned handles
    // escape, and no handle is destroyed or submitted to. Only immutable physical
    // properties are queried on the main graphics thread during adapter setup.
    let Some(hal) = (unsafe { adapter.as_hal::<wgpu::hal::api::Vulkan>() }) else {
        return identity;
    };
    let shared = hal.shared_instance();
    let caps = hal.physical_device_capabilities();
    // The core properties2 function and IDProperties require Vulkan 1.1. Don't
    // call a missing function or put unsupported extension structs in pNext.
    if shared.instance_api_version() < vk::API_VERSION_1_1
        || caps.properties().api_version < vk::API_VERSION_1_1
    {
        return identity;
    }
    let has_pci = caps.supports_extension(ext::pci_bus_info::NAME);
    let mut id = vk::PhysicalDeviceIDProperties::default();
    let mut pci = vk::PhysicalDevicePCIBusInfoPropertiesEXT::default();
    let mut props = vk::PhysicalDeviceProperties2::default().push_next(&mut id);
    if has_pci {
        props = props.push_next(&mut pci);
    }
    // SAFETY: valid guarded physical-device handle, Vulkan>=1.1, initialized
    // output/pNext structs live for this call; optional PCI struct is guarded
    // by the advertised VK_EXT_pci_bus_info physical-device extension.
    unsafe {
        shared
            .raw_instance()
            .get_physical_device_properties2(hal.raw_physical_device(), &mut props);
    }
    if id.device_uuid.iter().any(|byte| *byte != 0) {
        identity.uuid = Some(
            id.device_uuid
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect(),
        );
    }
    if has_pci {
        identity.pci = Some(format!(
            "{:04x}:{:02x}:{:02x}.{:x}",
            pci.pci_domain, pci.pci_bus, pci.pci_device, pci.pci_function
        ));
    }
    identity
}
