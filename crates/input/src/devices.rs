use std::mem::size_of;

use windows::Win32::Devices::DeviceAndDriverInstallation::{
    CM_Get_DevNode_PropertyW, CM_Get_Device_Interface_PropertyW, CM_LOCATE_DEVNODE_NORMAL,
    CM_Locate_DevNodeW, CR_SUCCESS,
};
use windows::Win32::Devices::HumanInterfaceDevice::{
    HIDD_ATTRIBUTES, HidD_GetAttributes, HidD_GetManufacturerString, HidD_GetProductString,
};
use windows::Win32::Devices::Properties::{
    DEVPKEY_Device_ContainerId, DEVPKEY_Device_InstanceId, DEVPROPTYPE,
};
use windows::Win32::Foundation::{CloseHandle, ERROR_INSUFFICIENT_BUFFER, HANDLE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::UI::Input::{
    GetRawInputDeviceInfoW, GetRawInputDeviceList, RAWINPUTDEVICELIST, RIDI_DEVICENAME,
    RIM_TYPEKEYBOARD,
};
use windows::core::{GUID, PCWSTR};

use crate::Keyboard;

// Every keyboard Raw Input knows about. Listing reads no keys, so it needs no test running.
pub fn keyboards() -> windows::core::Result<Vec<Keyboard>> {
    let item = size_of::<RAWINPUTDEVICELIST>() as u32;
    // A keyboard can arrive between the two calls, so the second one may need a bigger buffer.
    for _ in 0..4 {
        let mut count = 0;
        unsafe { GetRawInputDeviceList(None, &mut count, item) };
        let mut list = vec![RAWINPUTDEVICELIST::default(); count as usize];
        let written = unsafe { GetRawInputDeviceList(Some(list.as_mut_ptr()), &mut count, item) };
        if written == u32::MAX {
            let error = windows::core::Error::from_thread();
            if error.code() == ERROR_INSUFFICIENT_BUFFER.to_hresult() {
                continue;
            }
            return Err(error);
        }
        list.truncate(written as usize);
        return Ok(list
            .iter()
            .filter(|d| d.dwType == RIM_TYPEKEYBOARD)
            .filter_map(|d| describe(d.hDevice))
            .collect());
    }
    Err(ERROR_INSUFFICIENT_BUFFER.to_hresult().into())
}

fn describe(handle: HANDLE) -> Option<Keyboard> {
    let path = device_path(handle)?;
    let wide: Vec<u16> = path.encode_utf16().chain([0]).collect();
    let mut keyboard = Keyboard {
        handle: handle.0 as isize,
        path,
        vendor_id: None,
        product_id: None,
        manufacturer: None,
        product: None,
        container: container(&wide),
    };
    // No access rights: the handle can ask the device for its strings but can't read its reports.
    // A keyboard that isn't HID, such as a PS/2 laptop keyboard, doesn't open and keeps its path.
    let Ok(hid) = (unsafe {
        CreateFileW(
            PCWSTR(wide.as_ptr()),
            0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            None,
        )
    }) else {
        return Some(keyboard);
    };
    let mut attributes = HIDD_ATTRIBUTES {
        Size: size_of::<HIDD_ATTRIBUTES>() as u32,
        ..Default::default()
    };
    if unsafe { HidD_GetAttributes(hid, &mut attributes) } {
        keyboard.vendor_id = Some(attributes.VendorID);
        keyboard.product_id = Some(attributes.ProductID);
    }
    keyboard.manufacturer =
        hid_string(|buf, len| unsafe { HidD_GetManufacturerString(hid, buf, len) });
    keyboard.product = hid_string(|buf, len| unsafe { HidD_GetProductString(hid, buf, len) });
    let _ = unsafe { CloseHandle(hid) };
    Some(keyboard)
}

fn device_path(handle: HANDLE) -> Option<String> {
    let mut chars = 0;
    unsafe { GetRawInputDeviceInfoW(Some(handle), RIDI_DEVICENAME, None, &mut chars) };
    let mut buffer = vec![0u16; chars as usize];
    let copied = unsafe {
        GetRawInputDeviceInfoW(
            Some(handle),
            RIDI_DEVICENAME,
            Some(buffer.as_mut_ptr().cast()),
            &mut chars,
        )
    };
    if copied == u32::MAX || copied == 0 {
        return None;
    }
    Some(from_wide(&buffer))
}

fn container(interface: &[u16]) -> Option<u128> {
    let mut kind = DEVPROPTYPE::default();
    let mut size = 0;
    unsafe {
        CM_Get_Device_Interface_PropertyW(
            PCWSTR(interface.as_ptr()),
            &DEVPKEY_Device_InstanceId,
            &mut kind,
            None,
            &mut size,
            0,
        )
    };
    let mut instance = vec![0u16; (size as usize).div_ceil(2)];
    let found = unsafe {
        CM_Get_Device_Interface_PropertyW(
            PCWSTR(interface.as_ptr()),
            &DEVPKEY_Device_InstanceId,
            &mut kind,
            Some(instance.as_mut_ptr().cast()),
            &mut size,
            0,
        )
    };
    if found != CR_SUCCESS {
        return None;
    }
    let mut node = 0;
    let located = unsafe {
        CM_Locate_DevNodeW(
            &mut node,
            PCWSTR(instance.as_ptr()),
            CM_LOCATE_DEVNODE_NORMAL,
        )
    };
    if located != CR_SUCCESS {
        return None;
    }
    let mut guid = GUID::zeroed();
    let mut size = size_of::<GUID>() as u32;
    let read = unsafe {
        CM_Get_DevNode_PropertyW(
            node,
            &DEVPKEY_Device_ContainerId,
            &mut kind,
            Some((&mut guid as *mut GUID).cast()),
            &mut size,
            0,
        )
    };
    (read == CR_SUCCESS).then(|| guid.to_u128())
}

fn hid_string(read: impl Fn(*mut core::ffi::c_void, u32) -> bool) -> Option<String> {
    // USB caps a string at 126 characters plus the terminator.
    let mut buffer = [0u16; 128];
    if !read(buffer.as_mut_ptr().cast(), size_of::<[u16; 128]>() as u32) {
        return None;
    }
    let text = from_wide(&buffer);
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

fn from_wide(buffer: &[u16]) -> String {
    let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_keyboards_by_their_raw_input_handles() {
        for keyboard in keyboards().unwrap() {
            assert_ne!(keyboard.handle, 0);
            assert!(!keyboard.path.is_empty());
        }
    }

    #[test]
    fn stops_wide_strings_at_the_terminator() {
        let wide: Vec<u16> = "Keyboard\0junk".encode_utf16().collect();
        assert_eq!(from_wide(&wide), "Keyboard");
        assert_eq!(from_wide(&[]), "");
    }
}
