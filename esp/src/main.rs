use config::StdConfig;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::delay::TickType;
use esp_idf_hal::gpio::PinDriver;
use esp_idf_hal::peripherals::Peripherals;


use std::mem;

use anyhow::Result;

use log::*;


use esp_idf_svc::hal::delay;
use esp_idf_svc::hal::gpio;
use esp_idf_svc::hal::i2c;
use esp_idf_hal::i2s::*;
use esp_idf_svc::hal::prelude::*;

use embedded_graphics::mono_font::{ascii::FONT_10X20, MonoTextStyle};
use embedded_graphics::pixelcolor::*;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;
use embedded_graphics::text::*;

use ssd1306;
use ssd1306::mode::DisplayConfig;

type MyDisplay = ssd1306::Ssd1306<ssd1306::prelude::I2CInterface<i2c::I2cDriver<'static>>, ssd1306::prelude::DisplaySize128x64, ssd1306::mode::BufferedGraphicsMode<ssd1306::prelude::DisplaySize128x64>>;


pub struct MicrophonePins {
    bck: gpio::Gpio46,
    ws: gpio::Gpio42,
    din: gpio::Gpio45,
}


fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let pins = peripherals.pins;

    let mut led = PinDriver::output(pins.gpio35)?;

    let mut display = init_display(pins.gpio21, peripherals.i2c0, pins.gpio17, pins.gpio18)?;

    set_status(&mut display, "Hello")?;

    let pins = MicrophonePins {
        bck: pins.gpio46,  // Brown wire (SCK)
        din: pins.gpio45,  // Orange wire (SD)
        ws: pins.gpio42,   // Yellow wire (WS)
    };

    set_status(&mut display, "Configuring I2S ...")?;
    
    let mut i2s_driver = configure_i2s(peripherals.i2s0, pins)?;

    set_status(&mut display, "Configured I2S,enabling ...")?;

    i2s_driver.rx_enable()?;

    set_status(&mut display, "I2S enabled")?;
    let mut i = 0;

    loop {

      // Read audio samples
      match read_audio_samples(&mut i2s_driver) {
        Ok(samples) => {
            if !samples.is_empty() {
                // let min = samples.iter().min().unwrap();
                // let max = samples.iter().max().unwrap();
                // let delta = max - min;
                // info!("{samples:?}");
                // set_status(&mut display, &format!("{delta}"))?;
                //info!("{samples:?}");
                let Ok((left, _right)) = process_audio_samples(samples) else { continue; };
                let _left_normalized = normalize_samples(&left);
                process_and_display_fft(&left, &mut display);
            }
        },
        Err(e) => {
            error!("Error reading audio: {:?}", e);
            set_status(&mut display, "Audio Error")?;
        }
    }


        //set_status(&mut display, &format!("Hello {i}"))?;

        i += 1;
    }
}



            
// let samples_read = n / 4; // Each sample is 32 bits (4 bytes)
// // Convert bytes to u32 samples
// let mut sample_buffer = Vec::with_capacity(samples_read);
// for chunk in buffer.chunks_exact(4) {
//     let sample = i32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
//     sample_buffer.push(convert_24bit_to_i32(sample));
// }

// // Log some debug info about the samples
// if samples_read > 0 {
//     info!("Read {} samples. First sample: {}", samples_read, sample_buffer[0]);
// }

fn init_display(
    rst: gpio::Gpio21,
    i2c: i2c::I2C0,
    sda: gpio::Gpio17,
    scl: gpio::Gpio18,
) -> Result<MyDisplay> {
    info!("About to initialize the Heltec SSD1306 I2C LED driver");

    let di = ssd1306::I2CDisplayInterface::new(i2c::I2cDriver::new(
        i2c,
        sda,
        scl,
        &i2c::I2cConfig::new().baudrate(400.kHz().into()),
    )?);

    let mut reset = gpio::PinDriver::output(rst)?;

    reset.set_high()?;
    delay::Ets::delay_ms(1 as u32);

    reset.set_low()?;
    delay::Ets::delay_ms(10 as u32);

    reset.set_high()?;

    // PinDriver has a Drop implementation that resets the pin, which would turn off the display
    mem::forget(reset);


    let mut display: MyDisplay = ssd1306::Ssd1306::new(
        di,
        ssd1306::size::DisplaySize128x64,
        ssd1306::rotation::DisplayRotation::Rotate0,
    )
    .into_buffered_graphics_mode();

    display
        .init()
        .map_err(|e| anyhow::anyhow!("Display error: {:?}", e))?;
    
    write_text(&mut display, "Hello Rust!",BinaryColor::Off, BinaryColor::On, BinaryColor::Off, BinaryColor::On)
        .map_err(|e| anyhow::anyhow!("Display error: {:?}", e))?;


    display
        .flush()
        .map_err(|e| anyhow::anyhow!("Display error: {:?}", e))?;

    Ok(display)
}


fn write_text<D>(
    display: &mut D,
    text: &str,
    bg: D::Color,
    fg: D::Color,
    fill: D::Color,
    stroke: D::Color,
) -> Result<(), D::Error>
where
    D: DrawTarget + Dimensions,
{
    display.clear(bg)?;

    Rectangle::new(display.bounding_box().top_left, display.bounding_box().size)
        .into_styled(
            PrimitiveStyleBuilder::new()
                .fill_color(fill)
                .stroke_color(stroke)
                .stroke_width(1)
                .build(),
        )
        .draw(display)?;

    Text::new(
        &text,
        Point::new(10, (display.bounding_box().size.height - 10) as i32 / 2),
        MonoTextStyle::new(&FONT_10X20, fg),
    )
    .draw(display)?;

    info!("LED rendering done");

    Ok(())
}

fn set_status(
    display: &mut MyDisplay,
    text: &str
) -> Result<()>
{
    write_text(display, &text, BinaryColor::Off, BinaryColor::On, BinaryColor::Off, BinaryColor::On)
        .map_err(|e| anyhow::anyhow!("Display error: {:?}", e))?;

    display
        .flush()
        .map_err(|e| anyhow::anyhow!("Display error: {:?}", e))?;

    return Ok(());
}




pub fn configure_i2s(i2s: I2S0, pins: MicrophonePins) -> Result<I2sDriver<'static, I2sRx>> {
    let config = StdConfig::new(
        esp_idf_hal::i2s::config::Config::new(),
        config::StdClkConfig::from_sample_rate_hz(48_000)/*.mclk_multiple(config::MclkMultiple::M384)*/,
        config::StdSlotConfig::philips_slot_default(config::DataBitWidth::Bits32, config::SlotMode::Stereo),
        config::StdGpioConfig::default()
    );

    let mclk : Option<gpio::Gpio15> = None;
    // Configure I2S peripheral in Standard mode for receiving audio
    let i2s_driver = I2sDriver::new_std_rx(
        i2s,
        &config,
        pins.bck,
        pins.din,
        mclk,
        pins.ws,
    )?;

    Ok(i2s_driver)
}


pub fn read_audio_samples(i2s_driver: &mut I2sDriver<'_, I2sRx>) -> Result<Vec<u8>> {
    let mut buffer = vec![0u8; 48_000 * 4 * 2 / 10]; // Buffer for samples,48kHz sample rate, 2 channels, 4 byte per channel-sample, 0.1 second sample time
    
    match i2s_driver.read(&mut buffer, 1_000_000)? {
        n => {
            // Truncate buffer to actual bytes read
            buffer.truncate(n);
            Ok(buffer)
        }
    }
}

pub fn process_audio_samples(buffer: Vec<u8>) -> Result<(Vec<i32>, Vec<i32>)> {
    if buffer.len() % 8 != 0 {  // Change to 8 since each sample takes 8 bytes
        return Err(anyhow::anyhow!("Buffer length must be a multiple of 8"));
    }
    
    let mut left_samples = Vec::with_capacity(buffer.len() / 8 / 2);
    let mut right_samples = Vec::with_capacity(buffer.len() / 8 / 2);
    
    // Process in 8-byte chunks and fill both left and right
    for chunk in buffer.chunks_exact(8) {
        let left_value = i32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        left_samples.push(left_value);

        let right_value = i32::from_le_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]);
        right_samples.push(right_value);
    }
    
    Ok((left_samples, right_samples))
}

pub fn normalize_samples(samples: &[i32]) -> Vec<f32> {
    // Maximum possible value for 24-bit signed integer
    const MAX_VALUE: f32 = (1 << 23) as f32 - 1.0;
    
    samples.iter()
        .map(|&sample| (sample as f32) / MAX_VALUE)
        .collect()
}


use embedded_graphics::{
    mono_font::ascii::FONT_6X10,
    pixelcolor::BinaryColor,
    primitives::{Line, PrimitiveStyle},
    text::Text,
};
use rustfft::{num_complex::Complex, FftPlanner};

pub fn process_and_display_fft(
    samples: &[i32],
    display: &mut MyDisplay,
) -> Result<()> {
    // Convert samples to complex numbers and normalize
    let mut fft_input: Vec<Complex<f32>> = samples
        .iter()
        .map(|&s| Complex::new((s as f32) / (1 << 23) as f32, 0.0))
        .collect();

    // Perform FFT
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_input.len());
    fft.process(&mut fft_input);

    // Find dominant frequency
    let sample_rate = 48000.0;
    let bin_size = sample_rate / fft_input.len() as f32;
    
    // Skip DC component (index 0) and first few bins to avoid low frequency noise
    let dominant_bin = fft_input.iter()
        .skip(4)  // Skip first few bins
        .take(fft_input.len()/2)  // Only look at first half (Nyquist)
        .enumerate()
        .max_by_key(|(_i, c)| (c.norm() * 1000.0) as u32)
        .map(|(i, _)| i + 4)  // Add back the skipped bins
        .unwrap_or(0);

    let dominant_freq = dominant_bin as f32 * bin_size;

    // Clear display
    display.clear(BinaryColor::Off,).map_err(|e| anyhow::anyhow!("Display error: {:?}", e))?;

    // Draw frequency text
    let text_style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    Text::new(
        &format!("Freq: {:.0} Hz", dominant_freq),
        Point::new(0, 10),
        text_style,
    )
    .draw(display).map_err(|e| anyhow::anyhow!("Display error: {:?}", e))?;

    // // Draw frequency spectrum
    // let line_style = PrimitiveStyleBuilder::new().stroke_width(1).stroke_color(BinaryColor::On).build();
    
    // // We'll draw 64 frequency bins
    // let bins_to_draw = 64;
    // let bin_width = 128 / bins_to_draw;  // Display width / number of bins
    
    // for i in 0..bins_to_draw {
    //     // Get magnitude of this frequency bin
    //     let magnitude = if i < fft_input.len()/2 {
    //         fft_input[i].norm()
    //     } else {
    //         0.0
    //     };
        
    //     // Scale magnitude to display height (0-32 pixels, leaving room for text)
    //     let height = (magnitude * 32.0).min(32.0) as i32;
        
    //     // Draw vertical line for this bin
    //     Line::new(
    //         Point::new((i * bin_width) as i32, 64),
    //         Point::new((i * bin_width) as i32, 64 - height),
    //     )
    //     .into_styled(line_style)
    //     .draw(display).map_err(|e| anyhow::anyhow!("Display error: {:?}", e))?;
    // }

    // Draw frequency spectrum with more careful bounds checking and scaling
let line_style = PrimitiveStyleBuilder::new().stroke_width(1).stroke_color(BinaryColor::On).build();

// We'll draw 64 frequency bins
let bins_to_draw = 64;
let bin_width = 2;  // Fixed 2 pixels per bin to fit in 128 pixel width
let max_height = 40;  // Leave more room for text at top
let y_offset = 60;   // Start drawing from this y position

// Find maximum magnitude for scaling
let max_magnitude = fft_input.iter()
    .take(bins_to_draw)
    .map(|c| c.norm())
    .fold(0.0f32, f32::max);

for i in 0..bins_to_draw {
    // Get magnitude of this frequency bin, with bounds check
    let magnitude = if i < fft_input.len()/2 {
        fft_input[i].norm()
    } else {
        0.0
    };
    
    // Scale magnitude relative to maximum value
    let height = if max_magnitude > 0.0 {
        ((magnitude / max_magnitude) * max_height as f32) as i32
    } else {
        0
    };
    
    // Ensure x coordinate is within bounds
    let x = i32::min(127, i as i32 * bin_width);
    
    // Ensure y coordinates are within bounds
    let y_start = y_offset;
    let y_end = i32::max(20, y_offset - height); // Don't go above text area
    
    // Draw vertical line for this bin
    Line::new(
        Point::new(x, y_start),
        Point::new(x, y_end),
    )
    .into_styled(line_style)
    .draw(display).map_err(|e| anyhow::anyhow!("Display error: {:?}", e))?;
}

    display.flush().map_err(|e| anyhow::anyhow!("Display error: {:?}", e))?;
    Ok(())
}

// pub fn process_and_display_fft(
//     samples: &[i32],
//     display: &mut MyDisplay,
// ) -> Result<()> {
//     // Convert samples to complex numbers and normalize
//     let mut fft_input: Vec<Complex<f32>> = samples
//         .iter()
//         .map(|&s| Complex::new((s as f32) / (1 << 23) as f32, 0.0))
//         .collect();

//     // Perform FFT
//     let mut planner = FftPlanner::new();
//     let fft = planner.plan_fft_forward(fft_input.len());
//     fft.process(&mut fft_input);

//     // Find dominant frequency (using the same method as before for the text display)
//     let sample_rate = 48000.0;
//     let bin_size = sample_rate / fft_input.len() as f32;
    
//     let dominant_bin = fft_input.iter()
//         .skip(4)
//         .take(fft_input.len()/2)
//         .enumerate()
//         .max_by_key(|(_i, c)| (c.norm() * 1000.0) as u32)
//         .map(|(i, _)| i + 4)
//         .unwrap_or(0);

//     let dominant_freq = dominant_bin as f32 * bin_size;

//     // Clear display and show frequency text
//     display.clear(BinaryColor::Off).map_err(|e| anyhow::anyhow!("Display error: {:?}", e))?;
//     let text_style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
//     Text::new(
//         &format!("Freq: {:.0} Hz", dominant_freq),
//         Point::new(0, 10),
//         text_style,
//     )
//     .draw(display).map_err(|e| anyhow::anyhow!("Display error: {:?}", e))?;

//     // Calculate logarithmic frequency bands
//     let num_display_bands = 64;
//     let min_freq = 20.0;
//     let max_freq = sample_rate / 2.0;  // Nyquist frequency
//     let freq_ratio = (max_freq / min_freq).powf(1.0 / num_display_bands as f32);

//     // Create array for our display bands
//     let mut display_bands = vec![0.0f32; num_display_bands];
//     let mut band_counts = vec![0u32; num_display_bands];

//     // Distribute FFT bins into logarithmic bands
//     for (i, value) in fft_input.iter().take(fft_input.len()/2).enumerate() {
//         let freq = i as f32 * bin_size;
//         if freq < min_freq { continue; }
//         if freq > max_freq { break; }

//         // Find which display band this frequency belongs to
//         let band_index = (freq / min_freq).ln() / freq_ratio.ln();
//         let band = band_index.floor() as usize;
//         if band < num_display_bands {
//             display_bands[band] += value.norm();
//             band_counts[band] += 1;
//         }
//     }

//     // Average the bands and find maximum for scaling
//     let mut max_magnitude = 0.0f32;
//     for i in 0..num_display_bands {
//         if band_counts[i] > 0 {
//             display_bands[i] /= band_counts[i] as f32;
//             max_magnitude = max_magnitude.max(display_bands[i]);
//         }
//     }

//     // Draw the spectrum
//     let line_style = PrimitiveStyleBuilder::new()
//         .stroke_width(1)
//         .stroke_color(BinaryColor::On)
//         .build();

//     let max_height = 40;  // Leave room for text
//     let y_offset = 60;    // Start drawing from this y position

//     for i in 0..num_display_bands {
//         // Calculate height with logarithmic scaling for magnitude as well
//         let magnitude = display_bands[i];
//         let height = if max_magnitude > 0.0 {
//             let log_magnitude = if magnitude > 0.0 {
//                 (magnitude / max_magnitude).ln() + 1.0
//             } else {
//                 0.0
//             };
//             (log_magnitude * max_height as f32) as i32
//         } else {
//             0
//         };

//         // Draw at every other pixel position
//         let x = i32::min(127, (i * 2) as i32);
//         let y_end = i32::max(20, y_offset - height);

//         Line::new(
//             Point::new(x, y_offset),
//             Point::new(x, y_end),
//         )
//         .into_styled(line_style)
//         .draw(display).map_err(|e| anyhow::anyhow!("Display error: {:?}", e))?;
//     }

//     display.flush().map_err(|e| anyhow::anyhow!("Display error: {:?}", e))?;
//     Ok(())
// }