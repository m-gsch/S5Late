use rusb::{Direction, GlobalContext, Recipient, RequestType};
use std::{fs::File, io::Read, iter, thread::sleep, time::Duration};
use zerocopy::{FromBytes, FromZeros, Immutable, IntoBytes};
// Define a constant for the Apple Vendor ID
const APPLE_VENDOR_ID: u16 = 0x05ac;

#[derive(Debug)]
pub enum AppleDeviceError {
    // An error in rusb
    #[allow(dead_code)]
    RusbError(rusb::Error),
    // The iPod Nano 7G was not found in connected USB devices
    DeviceNotFound,
    // The iPod is not in the right mode for the requested operation
    WrongMode,
    // We had an error dealing with a file
    File,
    // We have no idea what happened
    Unknown,
}
impl From<rusb::Error> for AppleDeviceError {
    fn from(e: rusb::Error) -> Self {
        AppleDeviceError::RusbError(e)
    }
}

#[derive(Debug, PartialEq)]
pub enum Mode {
    DFU,
    WTF,
    DISK,
    Unknown(u16),
}

impl From<u16> for Mode {
    fn from(value: u16) -> Self {
        match value {
            0x1234 => Mode::DFU,
            0x1249 => Mode::WTF,
            0x124A => Mode::WTF,
            0x1267 => Mode::DISK,
            _ => Mode::Unknown(value),
        }
    }
}

#[derive(Debug)]
pub(crate) struct AppleDevice {
    pub mode: Mode,
    handle: rusb::DeviceHandle<GlobalContext>,
}

#[derive(Debug, IntoBytes, FromBytes, Immutable)]
#[repr(C)]
struct ImgHeader {
    magic: [u8; 4],
    version: [u8; 3],
    format: u8,
    entrypoint: u32,
    body_length: u32,
    data_length: u32,
    cert_offset: u32,
    cert_length: u32,
    padding: [u8; 0x3E4],
}

#[derive(Debug, IntoBytes, FromBytes, Immutable)]
#[repr(C)]
struct BootRomState {
    dfu_buf: u32,
    dfu_buf_size: u32,
    dfu_transfered_bytes: u32,
    dfu_pending_size: u32,
    usb_upload_complete: u8,
    field_11: u8,
    field_12: u8,
    field_13: u8,
    dfu_on_detach: u32,
    dfu_on_download_chunk: u32,
    dfu_on_upload_chunk: u32,
    dfu_boot_verify_and_copy: u32,
    vftable: u32,
    dfu_loop_run: u8,
    dfu_image_ready: u8,
    field_2a: u8,
    field_2b: u8,
    field_2c: u32,
    img_header_jump_offset: u32,
    img_header_version: [u8; 3],
    field_37: u8,
    field_38: u32,
    dfu_recursive_state: u32,
    dfu_interface_subclass: u8,
    dfu_interface_protocol: u8,
    dfu_status: [u8; 6],
    crc: u32,
    crcable: [u8; 4],
    usb_state: u8,
    current_configuration: u8,
    self_powered: u16,
    usb_handlers: [u32; 7],
    usb_endpoint_descriptor: u32,
    usb_unk_interface_desc_stuff: u16,
    usb_interface_no: u8,
    field_77: u8,
    usb_device_descriptor: u32,
    usb_configuration_descriptor: u32,
    usb_string_desc_zero: u32,
    usb_string_desc_count: u8,
    field_85: u8,
    field_86: u8,
    field_87: u8,
    aes_flag: u32,
    sha_intr_flag: u32,
    sha1_offset: u32,
    sha1_hashed: u32,
    sha1_cont: u32,
    dma_channels_used: u8,
    channel_0_configured: u8,
    field_9e: u8,
    field_9f: u8,
    usb_state_enum: u32,
    field_a4: u8,
    ep0state: u8,
    field_a6: u8,
    field_a7: u8,
    ep0_rxbuf_addr: u32,
    ep0_rxbuf_offset: u32,
    ep0_rxbuf_size: u32,
    ep0_txbuf_addr: u32,
    ep0_txbuf_offs: u32,
    ep0_txbuf_size: u32,
    ep0_dma: u32,
    speed: u8,
    field_c5: u8,
    field_c6: u8,
    field_c7: u8,
    timer_scale_from_dfu: u32,
    large_blocks: [u32; 12],
    chunks_in_large_blocks: [u32; 12],
    small_blocks: [u32; 14],
    chunks_in_small_blocks: [u32; 14],
    modes: [u8; 896],
    unk1: u32,
    unk2: u32,
    timer_scale: u32,
    field_528: u32,
    certs: [u8; 260],
    dfu_boot: u32,
    chipinfo_unk: u32,
    cn_certification_authority: u32,
    cn_s5l8740_secure_boot: u32,
}

#[allow(dead_code)]
impl AppleDevice {
    pub fn in_mode(mode: Mode) -> Result<Self, AppleDeviceError> {
        // Find the iPod Nano 7G USB device
        let device = rusb::devices()?
            .iter()
            .find(|device| {
                device.device_descriptor().map_or(false, |desc| {
                    desc.vendor_id() == APPLE_VENDOR_ID && Mode::from(desc.product_id()) == mode
                })
            })
            .ok_or(AppleDeviceError::DeviceNotFound)?;
        let handle = device.open()?;
        Ok(Self { mode, handle })
    }

    pub fn in_any_mode() -> Result<Self, AppleDeviceError> {
        // Find the iPod Nano 7G USB device
        let device = rusb::devices()?
            .iter()
            .find(|device| {
                device
                    .device_descriptor()
                    .map_or(false, |desc| desc.vendor_id() == APPLE_VENDOR_ID)
            })
            .ok_or(AppleDeviceError::DeviceNotFound)?;
        let mode: Mode = Mode::from(device.device_descriptor()?.product_id());
        let handle = device.open()?;
        Ok(Self { mode, handle })
    }

    fn dfu_dnload(&self, data: &[u8]) -> Result<(), AppleDeviceError> {
        if self.mode != Mode::DFU {
            return Err(AppleDeviceError::WrongMode);
        }

        let dnload_req_type =
            rusb::request_type(Direction::Out, RequestType::Class, Recipient::Interface);
        let nbytes =
            self.handle
                .write_control(dnload_req_type, 1, 1, 0, data, Duration::from_secs(5))?;
        log::debug!("DFU_DNLOAD: Sent {:#x} bytes {:x?}", nbytes, data);

        Ok(())
    }

    fn dfu_upload(&self, buffer: &mut [u8]) -> Result<(), AppleDeviceError> {
        if self.mode != Mode::DFU {
            return Err(AppleDeviceError::WrongMode);
        }

        let upload_req_type =
            rusb::request_type(Direction::In, RequestType::Class, Recipient::Interface);
        let nbytes =
            self.handle
                .read_control(upload_req_type, 2, 0, 0, buffer, Duration::from_secs(5))?;
        log::debug!("DFU_UPLOAD: Received {:#x} bytes {:x?}", nbytes, buffer);

        Ok(())
    }

    fn dfu_getstatus(&self) -> Result<Vec<u8>, AppleDeviceError> {
        if self.mode != Mode::DFU {
            return Err(AppleDeviceError::WrongMode);
        }

        let mut status_buffer = vec![0u8; 6];
        let getstatus_req_type =
            rusb::request_type(Direction::In, RequestType::Class, Recipient::Interface);
        let nbytes = self.handle.read_control(
            getstatus_req_type,
            3,
            0,
            0,
            &mut status_buffer,
            Duration::from_secs(5),
        )?;
        log::debug!(
            "DFU_GETSTATUS: Received {:#x} bytes {:x?}",
            nbytes,
            status_buffer
        );

        Ok(status_buffer)
    }

    fn dfu_clrstatus(&self) -> Result<(), AppleDeviceError> {
        if self.mode != Mode::DFU {
            return Err(AppleDeviceError::WrongMode);
        }

        let clrstatus_req_type =
            rusb::request_type(Direction::Out, RequestType::Class, Recipient::Interface);
        let nbytes =
            self.handle
                .write_control(clrstatus_req_type, 4, 0, 0, &[], Duration::from_secs(5))?;
        log::debug!("DFU_CLRSTATUS: Sent {:#x} bytes", nbytes);

        Ok(())
    }

    fn add_img_header(buffer: &[u8]) -> Vec<u8> {
        let mut img_header = ImgHeader::new_zeroed();
        let alignment_padding = 0x10 - (buffer.len() & 0xF);
        let body_len = buffer.len() + alignment_padding;
        let signature_len = 0x80;
        let certificates_len = 0x300;
        img_header.magic = *b"8740"; // Nano 7thGen magic
        img_header.version = *b"2.0"; // IMG version
        img_header.format = 4; // Unencrypted and signed
        img_header.body_length = body_len as u32;
        img_header.data_length = img_header.body_length + signature_len + certificates_len;
        img_header.cert_offset = img_header.body_length + signature_len;
        img_header.cert_length = certificates_len;

        let mut img_buffer = Vec::from(img_header.as_bytes());

        img_buffer.extend_from_slice(buffer);
        img_buffer.extend(iter::repeat_n(0, alignment_padding));
        img_buffer.extend(iter::repeat_n(b'S', signature_len as usize));
        img_buffer.extend(iter::repeat_n(b'C', certificates_len as usize));

        img_buffer
    }

    pub fn hax(&self) -> Result<(), AppleDeviceError> {
        if self.mode != Mode::DFU {
            return Err(AppleDeviceError::WrongMode);
        }

        let bootrom_state_bytes = include_bytes!("../assets/bootrom_state.bin");

        let mut bootrom_state = BootRomState::read_from_bytes(bootrom_state_bytes)
            .map_err(|_| AppleDeviceError::Unknown)?;
        bootrom_state.dfu_recursive_state = 0x2202e380;
        bootrom_state.usb_upload_complete = 0;
        bootrom_state.dfu_image_ready = 0;
        bootrom_state.dfu_loop_run = 1;
        // Patch on_upload_callback
        bootrom_state.dfu_on_upload_chunk = bootrom_state.dfu_buf + 0x400;

        self.dfu_clrstatus()?;

        // Send the stub code
        let mut data = vec![0u8; 0x780];
        let stub_code = include_bytes!("../assets/stub.bin");
        data[0x400..0x400 + stub_code.len()].copy_from_slice(stub_code);
        self.dfu_dnload(&data)?;
        self.dfu_getstatus()?;

        let data = vec![0u8; 0x100];
        self.dfu_dnload(&data)?;
        self.dfu_getstatus()?;

        // Send the new bootrom state
        self.dfu_dnload(bootrom_state.as_bytes())?;
        self.dfu_getstatus()?;

        // Calculate bytes until end of SRAM
        let bytes_remaining = 0x24C0 - 0x640 - 0x100 - 0x780;
        let loops_needed = bytes_remaining / 0x40;

        let data = vec![0u8; 0x40];
        for _ in 0..loops_needed {
            self.dfu_dnload(&data)?;
            self.dfu_getstatus()?;
        }
        let data = vec![0u8; 0x30];
        self.dfu_dnload(&data)?;
        self.dfu_getstatus()?;

        // End of SRAM in Nano 7:
        // (Standard) 43 10 58 ba 18 00 00 00 3c ba 02 22 00 d9 02 22
        // (DFU)      67 10 58 ba 08 00 00 00 3c ba 02 22 00 d9 02 22
        let overwrite: [u8; 0x10] = [
            0x67, 0x10, 0x58, 0xba, 0x00, 0x00, 0x00, 0x00, 0x80, 0xe3, 0x02, 0x22, 0x00, 0xd9,
            0x02, 0x22,
        ];
        self.dfu_dnload(&overwrite)?;
        self.dfu_getstatus()?;

        // Execute overwritten on_upload_callback()
        let mut data = vec![0u8; 0x40];
        self.dfu_clrstatus()?;
        self.dfu_upload(&mut data)?;
        self.dfu_getstatus()?;

        let product_name = self.handle.read_string_descriptor_ascii(2)?;

        log::debug!("Product name: {}", product_name);

        if product_name != "PWN DFU" {
            return Err(AppleDeviceError::Unknown);
        }

        Ok(())
    }

    fn load_image(&self, buffer: &[u8]) -> Result<(), AppleDeviceError> {
        if self.mode != Mode::DFU {
            return Err(AppleDeviceError::WrongMode);
        }

        let img_buffer = if buffer.len() < 0x400 || &buffer[0..4] != b"8740" {
            log::info!("Adding IMG header to the binary.");
            Self::add_img_header(buffer)
        } else {
            Vec::from(buffer)
        };

        // Send the image in chunks
        for chunk in img_buffer.chunks(0x800) {
            self.dfu_dnload(chunk)?;

            let mut dfu_status = self.dfu_getstatus()?;
            let mut poll_timeout =
                u64::from_le_bytes([dfu_status[1], dfu_status[2], dfu_status[3], 0, 0, 0, 0, 0]);
            let mut state = dfu_status[4];

            // wait in case of dfuDNBUSY
            while state != 5 {
                sleep(Duration::from_millis(poll_timeout));
                dfu_status = self.dfu_getstatus()?;
                poll_timeout = u64::from_le_bytes([
                    dfu_status[1],
                    dfu_status[2],
                    dfu_status[3],
                    0,
                    0,
                    0,
                    0,
                    0,
                ]);
                state = dfu_status[4];
            }
        }

        // Indicate everything has been sent
        self.dfu_dnload(&[])?;

        let status = self.dfu_getstatus()?;

        if status != [0, 0xB8, 0xB, 0, 7, 0] {
            // Unknown error when loading the image
            return Err(AppleDeviceError::Unknown);
        }

        Ok(())
    }

    pub fn load_image_from_file(&self, image_path: &str) -> Result<(), AppleDeviceError> {
        let mut image_buffer = Vec::new();
        let mut f = File::open(image_path).map_err(|_| AppleDeviceError::File)?;
        f.read_to_end(&mut image_buffer).unwrap();

        self.load_image(&image_buffer)?;

        Ok(())
    }
}
