#![no_std]
#![no_main]

extern crate alloc;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, Ordering};
use alloc::alloc::GlobalAlloc;
use alloc::vec::Vec;

struct BumpAllocator;

const HEAP_SIZE: usize = 16384; // 16 KB for embedded

struct SyncHeap {
    heap: UnsafeCell<[u8; HEAP_SIZE]>,
}

// :( :( :(
unsafe impl Sync for SyncHeap {}

static HEAP: SyncHeap = SyncHeap { 
    heap: UnsafeCell::new([0; HEAP_SIZE]) 
};
static HEAP_OFFSET: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let offset = HEAP_OFFSET.load(Ordering::SeqCst);
        let aligned = (offset + layout.align() - 1) & !(layout.align() - 1);
        let new_offset = aligned + layout.size();
        
        if new_offset > HEAP_SIZE {
            // alloc failure
            return core::ptr::null_mut();
        }
        
        HEAP_OFFSET.store(new_offset, Ordering::SeqCst);
        (HEAP.heap.get() as *mut u8).add(aligned)
    }
    
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {
        // Bump allocator doesn't support deallocation
    }
}

#[global_allocator]
static ALLOCATOR: BumpAllocator = BumpAllocator;

use embassy_time::{Duration, Timer};
use panic_halt as _;

mod crypto;
mod uart_hw;

#[embassy_executor::main]
async fn main(spawner: embassy_executor::Spawner) {
    init_hardware().await;

    spawner.spawn(uart_listener()).unwrap();

    loop {
        // remove
        Timer::after(Duration::from_secs(5)).await;
        
// todo
    }
}

trait UartPort {
    fn read_byte(&mut self) -> u8;
    
    fn write_byte(&mut self, byte: u8);
    
    fn data_available(&mut self) -> bool {
        false  
    }
}

/// fix
struct MockUart {
    rx_buffer: [u8; 1024],
    rx_head: usize,
    rx_tail: usize,
    rx_count: usize,
    
    tx_buffer: [u8; 1024],
    #[allow(dead_code)]
    tx_head: usize,
    tx_tail: usize,
    tx_count: usize,
}

impl MockUart {
    fn new() -> Self {
        MockUart {
            rx_buffer: [0u8; 1024],
            rx_head: 0,
            rx_tail: 0,
            rx_count: 0,
            
            tx_buffer: [0u8; 1024],
            tx_head: 0,
            tx_tail: 0,
            tx_count: 0,
        }
    }
    
    fn inject_bytes(&mut self, data: &[u8]) {
        for &byte in data {
            if self.rx_count < self.rx_buffer.len() {
                self.rx_buffer[self.rx_tail] = byte;
                self.rx_tail = (self.rx_tail + 1) % self.rx_buffer.len();
                self.rx_count += 1;
            }
        }
    }
}

impl UartPort for MockUart {
    fn read_byte(&mut self) -> u8 {
        loop {
            if self.rx_count > 0 {
                let byte = self.rx_buffer[self.rx_head];
                self.rx_head = (self.rx_head + 1) % self.rx_buffer.len();
                self.rx_count -= 1;
                return byte;
            }
            core::hint::spin_loop();
        }
    }
    
    fn write_byte(&mut self, byte: u8) {
        if self.tx_count < self.tx_buffer.len() {
            self.tx_buffer[self.tx_tail] = byte;
            self.tx_tail = (self.tx_tail + 1) % self.tx_buffer.len();
            self.tx_count += 1;
        }
    }
    
    fn data_available(&mut self) -> bool {
        self.rx_count > 0
    }
}

#[cfg(target_arch = "arm")]
struct HardwareUart {
//fix
}

#[cfg(target_arch = "arm")]
impl HardwareUart {
    fn new() -> Self {
        HardwareUart {}
    }
}

#[cfg(target_arch = "arm")]
impl UartPort for HardwareUart {
    fn read_byte(&mut self) -> u8 {
        uart_hw::uart_read_byte()
    }
    
    fn write_byte(&mut self, byte: u8) {
        uart_hw::uart_write_byte(byte);
    }
    
    fn data_available(&mut self) -> bool {
        uart_hw::uart_data_available()
    }
}

#[cfg(not(target_arch = "arm"))]
struct HardwareUart {
    _phantom: core::marker::PhantomData<()>,
}

#[cfg(not(target_arch = "arm"))]
impl HardwareUart {
    fn new() -> Self {
        HardwareUart {
            _phantom: core::marker::PhantomData,
        }
    }
}

#[cfg(not(target_arch = "arm"))]
impl UartPort for HardwareUart {
    fn read_byte(&mut self) -> u8 {
        0
    }
    
    fn write_byte(&mut self, _byte: u8) {}
}

async fn init_hardware() {
    #[cfg(target_arch = "arm")]
    {
        uart_hw::init_uart();
    }
}

// host interfacing
const MAGIC_BYTE: u8 = b'%'; // 0x25

const OPCODE_LIST: u8 = b'L'; // 0x4C
const OPCODE_READ: u8 = b'R'; // 0x52
const OPCODE_WRITE: u8 = b'W'; // 0x57
#[allow(dead_code)]
const OPCODE_RECEIVE: u8 = b'C'; // 0x43
#[allow(dead_code)]
const OPCODE_INTERROGATE: u8 = b'I'; // 0x49
#[allow(dead_code)]
const OPCODE_LISTEN: u8 = b'N'; // 0x4E
const OPCODE_ACK: u8 = b'A';  // 0x41
const OPCODE_ERROR: u8 = b'E'; // 0x45
#[allow(dead_code)]
const OPCODE_DEBUG: u8 = b'D'; // 0x44

const MSG_HEADER_SIZE: usize = 4; // MAGIC(1) + OPCODE(1) + LENGTH(2)
const MAX_PIN_LEN: usize = 6;
const FILE_NAME_SIZE: usize = 32;
const MAX_FILES: usize = 8;

#[repr(C)]
#[derive(Copy, Clone)]
struct FileMetadata {
    slot: u8,           // 1 byte
    group_id: u16,      // 2 bytes (little-endian)
    name: [u8; FILE_NAME_SIZE], // 32 bytes (null-terminated string)
}

// fix
struct MockFileSystem {
    files: [FileMetadata; MAX_FILES],
    file_count: usize,
}

impl MockFileSystem {
    fn new() -> Self {
        let default_file = FileMetadata {
            slot: 0,
            group_id: 0,
            name: [0; FILE_NAME_SIZE],
        };

        let mut fs = MockFileSystem {
            files: [default_file; MAX_FILES],
            file_count: 0,
        };

        let file1_name = b"secret.key";
        let mut file1 = FileMetadata {
            slot: 0,
            group_id: 1,
            name: [0; FILE_NAME_SIZE],
        };
        file1.name[..file1_name.len()].copy_from_slice(file1_name);
        fs.files[0] = file1;

        let file2_name = b"config.dat";
        let mut file2 = FileMetadata {
            slot: 1,
            group_id: 2,
            name: [0; FILE_NAME_SIZE],
        };
        file2.name[..file2_name.len()].copy_from_slice(file2_name);
        fs.files[1] = file2;

        let file3_name = b"firmware.bin";
        let mut file3 = FileMetadata {
            slot: 2,
            group_id: 1,
            name: [0; FILE_NAME_SIZE],
        };
        file3.name[..file3_name.len()].copy_from_slice(file3_name);
        fs.files[2] = file3;

        fs.file_count = 3;
        fs
    }

    fn get_files(&self) -> &[FileMetadata] {
        &self.files[..self.file_count]
    }

    fn count(&self) -> u32 {
        self.file_count as u32
    }
}

/// fix properly
fn validate_pin(pin: &[u8]) -> bool {
return true
}

struct MessageHandler {
    filesystem: MockFileSystem,
}

impl MessageHandler {
    fn new() -> Self {
        MessageHandler {
            filesystem: MockFileSystem::new(),
        }
    }

    /// MAGIC + OPCODE + LENGTH + BODY
    fn build_response(&self, opcode: u8, body: &[u8], output: &mut [u8]) -> usize {
        if output.len() < MSG_HEADER_SIZE + body.len() {
            return 0; // Buffer too small
        }

        let total_len = MSG_HEADER_SIZE + body.len();
        output[0] = MAGIC_BYTE;
        output[1] = opcode;
        let len_bytes = (body.len() as u16).to_le_bytes();
        output[2] = len_bytes[0];
        output[3] = len_bytes[1];
        output[4..4 + body.len()].copy_from_slice(body);
        
        total_len
    }

    fn handle_list_command(&self, body: &[u8]) -> Result<Vec<u8>, &'static str> {
        use alloc::vec::Vec;
        
        if body.len() < MAX_PIN_LEN {
            return Err("Invalid command format: PIN too short");
        }

        let pin = &body[..MAX_PIN_LEN];

        if !validate_pin(pin) {
            return Err("Invalid PIN format");
        }

        // impl pin check

        let mut response = Vec::new();
        let num_files = self.filesystem.count();
        
        response.extend_from_slice(&num_files.to_le_bytes());

        for file_meta in self.filesystem.get_files() {
            response.push(file_meta.slot);
            response.extend_from_slice(&file_meta.group_id.to_le_bytes());
            response.extend_from_slice(&file_meta.name);
        }

        Ok(response)
    }

    fn build_error_response(message: &[u8], output: &mut [u8]) -> usize {
        if output.len() < MSG_HEADER_SIZE + message.len() {
            return 0;
        }

        let total_len = MSG_HEADER_SIZE + message.len();
        output[0] = MAGIC_BYTE;
        output[1] = OPCODE_ERROR;
        let len_bytes = (message.len() as u16).to_le_bytes();
        output[2] = len_bytes[0];
        output[3] = len_bytes[1];
        output[4..4 + message.len()].copy_from_slice(message);
        
        total_len
    }
}

#[embassy_executor::task]
async fn uart_listener() {
    #[cfg(target_arch = "arm")]
    let mut uart = HardwareUart::new();
    
    #[cfg(not(target_arch = "arm"))]
    let mut uart = MockUart::new();
    
    let handler = MessageHandler::new();
    
    #[cfg(not(target_arch = "arm"))]
    {
        let test_command = [
            b'%', b'L',
            0x06, 0x00,
            b'a', b'b', b'c', b'1', b'2', b'3',
        ];
        uart.inject_bytes(&test_command);
    }
    
    loop {
        let magic = uart.read_byte();
        if magic != MAGIC_BYTE {
            continue;  
        }
        
        let opcode = uart.read_byte();
        let length_lo = uart.read_byte();
        let length_hi = uart.read_byte();
        let body_len = u16::from_le_bytes([length_lo, length_hi]) as usize;
        
        if body_len > 512 {
            continue;
        }
        
        let mut body = [0u8; 512];
        for i in 0..body_len {
            body[i] = uart.read_byte();
        }
        
        let mut response_buf = [0u8; 1024];
        let response_len = match opcode {
            OPCODE_LIST => {
                match handler.handle_list_command(&body[..body_len]) {
                    Ok(list_data) => {
                        if list_data.len() <= response_buf.len() - MSG_HEADER_SIZE {
                            handler.build_response(OPCODE_LIST, &list_data, &mut response_buf)
                        } else {
                            let error_msg = b"Response too large";
                            MessageHandler::build_error_response(error_msg, &mut response_buf)
                        }
                    }
                    Err(err_msg) => {
                        let error_bytes = err_msg.as_bytes();
                        MessageHandler::build_error_response(error_bytes, &mut response_buf)
                    }
                }
            }
            OPCODE_READ => {
                let error_msg = b"READ not yet implemented";
                MessageHandler::build_error_response(error_msg, &mut response_buf)
            }
            OPCODE_WRITE => {
                let error_msg = b"WRITE not yet implemented";
                MessageHandler::build_error_response(error_msg, &mut response_buf)
            }
            _ => {
                let error_msg = b"Unknown command";
                MessageHandler::build_error_response(error_msg, &mut response_buf)
            }
        };
        
        for &byte in &response_buf[..response_len] {
            uart.write_byte(byte);
        }
    }
}


#[allow(dead_code)] /// fix
mod crypto_ops {
    use super::crypto;
    
    #[inline]
    pub fn hash_file(data: &[u8]) -> [u8; 32] {
        crypto::blake3_hash(data)
    }
    
    #[inline]
    pub fn verify_signature(_data: &[u8], _signature: &[u8], _pubkey: &[u8]) -> bool {
        false
    }
}

