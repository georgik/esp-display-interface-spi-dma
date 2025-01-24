//! DMA SPI interface for display drivers

use core::cell::RefCell;
use core::ptr::addr_of_mut;

use byte_slice_cast::AsByteSlice;
use display_interface::{DataFormat, DisplayError, WriteOnlyDataCommand};
use esp_hal::{
    dma::{DmaDescriptor, DmaTxBuf},
    gpio::Output,
    spi::master::SpiDmaTransfer,
};

const DMA_BUFFER_SIZE: usize = 4096;
type SpiDma<'d> = esp_hal::spi::master::SpiDma<'d, esp_hal::Blocking>;

/// SPI display interface.
///
/// This combines the SPI peripheral and a data/command as well as a chip-select pin
pub struct SPIInterface<'d> {
    avg_data_len_hint: usize,
    spi: RefCell<Option<SpiDma<'d>>>,
    transfer: RefCell<Option<SpiDmaTransfer<'d, esp_hal::Blocking, DmaTxBuf>>>,
    dc: Output<'d>,
    cs: Option<Output<'d>>,
}

impl<'d> SPIInterface<'d> {
    pub fn new(avg_data_len_hint: usize, spi: SpiDma<'d>, dc: Output<'d>, cs: Output<'d>) -> Self {
        Self {
            avg_data_len_hint,
            spi: RefCell::new(Some(spi)),
            transfer: RefCell::new(None),
            dc,
            cs: Some(cs),
        }
    }

    fn single_transfer(&mut self, send_buffer: &'static mut [u8]) {
        let mut buffer = DmaTxBuf::new(descriptors(), send_buffer).unwrap();
        let transfer = self.spi.take().unwrap().write(buffer.len(), buffer).unwrap();
        let (reclaimed_spi, _) = transfer.wait();
        self.spi.replace(Some(reclaimed_spi));
    }

    fn send_u8(&mut self, words: DataFormat<'_>) -> Result<(), DisplayError> {
        if let Some(transfer) = self.transfer.take() {
            let (reclaimed_spi, _) = transfer.wait();
            self.spi.replace(Some(reclaimed_spi));
        }

        match words {
            DataFormat::U8(slice) => {
                let send_buffer = dma_buffer1();
                send_buffer[..slice.len()].copy_from_slice(slice.as_byte_slice());
                self.single_transfer(send_buffer);
            }
            DataFormat::U16(slice) => {
                let send_buffer = dma_buffer1();
                send_buffer[..slice.len() * 2].copy_from_slice(slice.as_byte_slice());
                self.single_transfer(send_buffer);
            }
            // Handle other cases similarly...
            _ => return Err(DisplayError::DataFormatNotImplemented),
        }
        Ok(())
    }

    fn iter_transfer<WORD>(
        &mut self,
        iter: &mut dyn Iterator<Item = WORD>,
        convert: fn(WORD) -> <WORD as num_traits::ToBytes>::Bytes,
    ) where
        WORD: num_traits::int::PrimInt + num_traits::ToBytes,
    {
        let mut desired_chunk_sized =
            self.avg_data_len_hint - ((self.avg_data_len_hint / DMA_BUFFER_SIZE) * DMA_BUFFER_SIZE);
        let mut spi = Some(self.spi.take().unwrap());
        let mut current_buffer = 0;
        let mut transfer: Option<SpiDmaTransfer<'d, esp_hal::Blocking, DmaTxBuf>> = None;
        loop {
            let buffer = if current_buffer == 0 {
                &mut dma_buffer1()[..]
            } else {
                &mut dma_buffer2()[..]
            };
            let mut idx = 0;
            loop {
                let b = iter.next();
                match b {
                    Some(b) => {
                        let b = convert(b);
                        let b = b.as_byte_slice();
                        buffer[idx..idx + b.len()].copy_from_slice(b);
                        idx += b.len();
                    }
                    None => break,
                }
                if idx >= usize::min(desired_chunk_sized, DMA_BUFFER_SIZE) {
                    break;
                }
            }
            desired_chunk_sized = DMA_BUFFER_SIZE;

            if let Some(transfer) = transfer.take() {
                if idx > 0 {
                    let (reclaimed_spi, _) = transfer.wait();
                    spi = Some(reclaimed_spi);
                } else {
                    self.transfer.replace(Some(transfer));
                }
            }

            if idx > 0 {
                let mut dma_buffer = DmaTxBuf::new(descriptors(), buffer).unwrap();
                dma_buffer.set_length(idx);
                transfer = Some(spi.take().unwrap().write(dma_buffer.len(), dma_buffer).unwrap());
                current_buffer = (current_buffer + 1) % 2;
            } else {
                break;
            }
        }
    }
}

pub fn new_no_cs<'d>(
    avg_data_len_hint: usize,
    spi: SpiDma<'d>,
    dc: Output<'d>,
) -> SPIInterface<'d> {
    SPIInterface {
        avg_data_len_hint,
        spi: RefCell::new(Some(spi)),
        transfer: RefCell::new(None),
        dc,
        cs: None,
    }
}

impl<'d> WriteOnlyDataCommand for SPIInterface<'d> {
    fn send_commands(&mut self, cmds: DataFormat<'_>) -> Result<(), DisplayError> {
        if let Some(cs) = self.cs.as_mut() {
            cs.set_low();
        }
        self.dc.set_low();
        let res = self.send_u8(cmds);
        if let Some(cs) = self.cs.as_mut() {
            cs.set_high();
        }
        res
    }

    fn send_data(&mut self, buf: DataFormat<'_>) -> Result<(), DisplayError> {
        if let Some(cs) = self.cs.as_mut() {
            cs.set_low();
        }
        self.dc.set_high();
        let res = self.send_u8(buf);
        if let Some(cs) = self.cs.as_mut() {
            cs.set_high();
        }
        res
    }
}

fn descriptors() -> &'static mut [DmaDescriptor; 8 * 3] {
    static mut DESCRIPTORS: [DmaDescriptor; 8 * 3] = [DmaDescriptor::EMPTY; 8 * 3];
    unsafe { &mut *addr_of_mut!(DESCRIPTORS) }
}

fn dma_buffer1() -> &'static mut [u8; DMA_BUFFER_SIZE] {
    static mut BUFFER: [u8; DMA_BUFFER_SIZE] = [0u8; DMA_BUFFER_SIZE];
    unsafe { &mut *addr_of_mut!(BUFFER) }
}

fn dma_buffer2() -> &'static mut [u8; DMA_BUFFER_SIZE] {
    static mut BUFFER: [u8; DMA_BUFFER_SIZE] = [0u8; DMA_BUFFER_SIZE];
    unsafe { &mut *addr_of_mut!(BUFFER) }
}
