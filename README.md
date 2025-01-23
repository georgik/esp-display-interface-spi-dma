# ESP Display Interface with SPI and DMA

Rust Bare Metal implementation of SPI interface with DMA for ESP32.

## Usage

Add dependencies to the project:

```
cargo add esp-display-interface-spi-dma
```

## Code

Example for ESP32-S3-BOX:

```rust
use esp_display_interface_spi_dma::display_interface_spi_dma;
//  ...
fn display_code() {
    let spi = Spi::new_with_config(
        peripherals.SPI2,
        esp_hal::spi::master::Config {
            frequency: 40u32.MHz(),
            ..esp_hal::spi::master::Config::default()
        },
    )
    .with_sck(lcd_sclk)
    .with_mosi(lcd_mosi)
    .with_miso(lcd_miso)
    .with_cs(lcd_cs)

    .with_dma(dma_channel.configure(false, DmaPriority::Priority0));
    
    let di = display_interface_spi_dma::new_no_cs(LCD_MEMORY_SIZE, spi, lcd_dc);

    let mut display = mipidsi::Builder::new(mipidsi::models::ILI9341Rgb565, di)
        .display_size(240, 320)
        .orientation(mipidsi::options::Orientation::new())
        .color_order(mipidsi::options::ColorOrder::Bgr)
        .reset_pin(lcd_reset)
        .init(&mut delay)
        .unwrap();

    let _ = lcd_backlight.set_high();

    println!("Initializing...");
    Text::new(
        "Initializing...",
        Point::new(80, 110),
        MonoTextStyle::new(&FONT_8X13, RgbColor::WHITE),
    )
    .draw(&mut display)
    .unwrap();
}
```

## Related

- [ESP32 Board Support Package for Rust - esp-bsp](https://crates.io/crates/esp-bsp)
- 