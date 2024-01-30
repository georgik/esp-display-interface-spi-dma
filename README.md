# ESP Display Interface with SPI and DMA

Rust Bare Metal implementation of SPI interface with DMA for ESP32.

## Usage

Add dependencies to the project:

```
cargo add esp-display-interface-spi-dma
cargo add static_cell
cargo add esp-bsp
```

## Code

Example for ESP32-S3-BOX:

```
use static_cell::make_static;
use esp_display_interface_spi_dma::display_interface_spi_dma;
...

let dma = Gdma::new(peripherals.DMA);
let dma_channel = dma.channel0;
let descriptors = make_static!([0u32; 8 * 3]);
let rx_descriptors = make_static!([0u32; 8 * 3]);
let (lcd_sclk, lcd_mosi, lcd_cs, lcd_miso, lcd_dc, mut lcd_backlight, lcd_reset) = lcd_gpios!(BoardType::ESP32S3Box, io);

let spi = Spi::new(
    peripherals.SPI2,
    40u32.MHz(),
    SpiMode::Mode0,
    &clocks
).with_pins(
    Some(lcd_sclk),
    Some(lcd_mosi),
    Some(lcd_miso),
    Some(lcd_cs),
).with_dma(
    dma_channel.configure(
        false,
        &mut *descriptors,
        &mut *rx_descriptors,
        DmaPriority::Priority0,
    )
);


let di = display_interface_spi_dma::new_no_cs(2 * 256 * 192, spi, lcd_dc);

let display_config = DisplayConfig::for_board(BoardType::ESP32S3Box);
let mut display = match mipidsi::Builder::ili9342c_rgb565(di)
    .with_display_size(display_config.h_res, display_config.v_res)
    .with_orientation(mipidsi::Orientation::PortraitInverted(false))
    .with_color_order(mipidsi::ColorOrder::Bgr)
    .init(&mut delay, Some(lcd_reset))
{
    Ok(display) => display,
    Err(_e) => {
        // Handle the error and possibly exit the application
        panic!("Display initialization failed");
    }
};
```