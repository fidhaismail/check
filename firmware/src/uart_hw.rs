/// refer ti_msp_dl_config.c 

use core::ptr::{read_volatile, write_volatile};
const UART0_BASE: usize = 0x4000_0000;

#[allow(dead_code)]
const GPIOA_BASE: usize = 0x4101_0000;

const SYSCTL_BASE: usize = 0x4010_0000;

const IOMUX_BASE: usize = 0x4500_0000;

#[allow(dead_code)]
const UART_DATA: usize = 0x008;     
#[allow(dead_code)]
const UART_RSR_ECR: usize = 0x004;  
const UART_FR: usize = 0x018;       
const UART_IBRD: usize = 0x024;     
const UART_FBRD: usize = 0x028;     
const UART_LCRH: usize = 0x02C;     
const UART_CR: usize = 0x030;       
#[allow(dead_code)]
const UART_IMSC: usize = 0x038;     
#[allow(dead_code)]
const UART_RIS: usize = 0x03C;      
#[allow(dead_code)]
const UART_MIS: usize = 0x040;      
#[allow(dead_code)]
const UART_ICR: usize = 0x044;      

// Flag Register Bits
#[allow(dead_code)]
const UART_FR_TXFE: u32 = 0x80;     
#[allow(dead_code)]
const UART_FR_RXFF: u32 = 0x40;     
const UART_FR_TXFF: u32 = 0x20;     
const UART_FR_RXFE: u32 = 0x10;     
#[allow(dead_code)]
const UART_FR_BUSY: u32 = 0x08;     // UART busy
#[allow(dead_code)]
const UART_FR_CTS: u32 = 0x01;      

const UART_LCRH_FEN: u32 = 0x10;   
const UART_LCRH_WLEN_8BITS: u32 = 0x60; 
const UART_LCRH_STP1: u32 = 0x00;   

const UART_CR_RXE: u32 = 0x200;     
const UART_CR_TXE: u32 = 0x100;     
const UART_CR_UARTEN: u32 = 0x001;  
#[allow(dead_code)]
const GPIO_OUT_EN: usize = 0x010;   
#[allow(dead_code)]
const GPIO_DO_OUT: usize = 0x014;   
#[allow(dead_code)]
const GPIO_OE: usize = 0x040;       
#[allow(dead_code)]
const GPIO_CFG: usize = 0x0C0;      

const SYSCTL_SOCLOCK: usize = 0x100; 
const SYSCTL_MCLKCFG: usize = 0x104; 
#[allow(dead_code)]
const SYSCTL_PCLKCFG0: usize = 0x110;
#[allow(dead_code)]
const SYSCTL_PCLKCFG1: usize = 0x114;
#[allow(dead_code)]
const SYSCTL_CLKEN0: usize = 0x118;  
const SYSCTL_CLKEN1: usize = 0x11C;  
#[allow(dead_code)]
const SYSCTL_RESET0: usize = 0x120;  
#[allow(dead_code)]
const SYSCTL_RESET1: usize = 0x124;  
#[allow(dead_code)]
const SYSCTL_BORTHRES: usize = 0x180;
const SYSCTL_PWREN0: usize = 0x1C0;  
#[allow(dead_code)]
const SYSCTL_PWREN1: usize = 0x1C4;  

const SYSCTL_CLKEN1_UART0: u32 = 0x00000001;
#[allow(dead_code)]
const SYSCTL_CLKEN1_UART1: u32 = 0x00000002;

const SYSCTL_PWREN0_GPIOA: u32 = 0x00000001;
const SYSCTL_PWREN0_GPIOB: u32 = 0x00000002;


const IOMUX_PINCM25: usize = 0x064;  // PA10
const IOMUX_PINCM26: usize = 0x068;  // PA11

const IOMUX_PF_UART0_TX: u32 = 0x00000029; // PA10 function select for UART0 TX
const IOMUX_PF_UART0_RX: u32 = 0x00000029; // PA11 function select for UART0 RX


#[inline]
fn read_reg(base: usize, offset: usize) -> u32 {
    unsafe { read_volatile((base + offset) as *const u32) }
}

#[inline]
fn write_reg(base: usize, offset: usize, value: u32) {
    unsafe { write_volatile((base + offset) as *mut u32, value) }
}

#[inline]
fn set_bits(base: usize, offset: usize, mask: u32) {
    let value = read_reg(base, offset);
    write_reg(base, offset, value | mask);
}

#[inline]
#[allow(dead_code)]
fn clear_bits(base: usize, offset: usize, mask: u32) {
    let value = read_reg(base, offset);
    write_reg(base, offset, value & !mask);
}

#[inline]
fn delay_cycles(mut cycles: u32) {
    while cycles > 0 {
        unsafe { core::arch::asm!("nop") };
        cycles -= 1;
    }
}

fn init_power() {
    set_bits(SYSCTL_BASE, SYSCTL_PWREN0, SYSCTL_PWREN0_GPIOA);
    set_bits(SYSCTL_BASE, SYSCTL_PWREN0, SYSCTL_PWREN0_GPIOB);
    
    delay_cycles(16);
}

fn init_sysctl() {
    let mut value = read_reg(SYSCTL_BASE, SYSCTL_SOCLOCK);
    value &= !0x0F000000; // Clear FREQ bits
    value |= 0x00000000;  // Set to BASE (32 MHz) - this is the default
    write_reg(SYSCTL_BASE, SYSCTL_SOCLOCK, value);
    
    let mut value = read_reg(SYSCTL_BASE, SYSCTL_MCLKCFG);
    value &= !0x07000000; // Clear divider bits
    value |= 0x00000000;  // Set to DISABLE (no divider)
    write_reg(SYSCTL_BASE, SYSCTL_MCLKCFG, value);
}

fn init_gpio() {
    write_reg(IOMUX_BASE, IOMUX_PINCM25, IOMUX_PF_UART0_TX);
    
    write_reg(IOMUX_BASE, IOMUX_PINCM26, IOMUX_PF_UART0_RX);
}

fn init_uart0() {
    set_bits(SYSCTL_BASE, SYSCTL_CLKEN1, SYSCTL_CLKEN1_UART0);
    
    write_reg(UART0_BASE, UART_IBRD, 17);
    write_reg(UART0_BASE, UART_FBRD, 23);
    
    let lcrh = UART_LCRH_FEN | UART_LCRH_WLEN_8BITS | UART_LCRH_STP1;
    write_reg(UART0_BASE, UART_LCRH, lcrh);
    
    let cr = UART_CR_TXE | UART_CR_RXE | UART_CR_UARTEN;
    write_reg(UART0_BASE, UART_CR, cr);
}

pub fn init_uart() {
    init_power();
    init_sysctl();
    init_gpio();
    init_uart0();
}

const UART_READ_TIMEOUT: u32 = 10_000_000;

pub fn uart_read_byte() -> u8 {
    let mut timeout_count = 0u32;
    
    loop {
        let fr = read_reg(UART0_BASE, UART_FR);
        if (fr & UART_FR_RXFE) == 0 {
            break;
        }
        
        timeout_count += 1;
        if timeout_count >= UART_READ_TIMEOUT {
            return 0x00;
        }
        
        core::hint::spin_loop();
    }
    
    let data = read_reg(UART0_BASE, UART_DATA);
    (data & 0xFF) as u8
}

pub fn uart_write_byte(byte: u8) {
    loop {
        let fr = read_reg(UART0_BASE, UART_FR);
        if (fr & UART_FR_TXFF) == 0 {
            break;
        }
        core::hint::spin_loop();
    }
    
    write_reg(UART0_BASE, UART_DATA, byte as u32);
}

#[allow(dead_code)]
pub fn uart_data_available() -> bool {
    let fr = read_reg(UART0_BASE, UART_FR);
    (fr & UART_FR_RXFE) == 0
}
