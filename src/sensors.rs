use embassy_stm32::Peripheral;
use embassy_stm32::adc::{Adc, AdcChannel, Instance};

/// Represents the detected light level by the LDR sensor
#[derive(defmt::Format, Debug, PartialEq, Eq)]
pub enum LightLevel {
    /// Light level is above the configured threshold
    Bright,
    /// Light level is below the configured threshold
    Dark,
}

/// Light Dependent Resistor (LDR) sensor driver
///
/// This driver provides an interface to read light levels using an LDR
/// connected to an ADC pin. The sensor converts light intensity to
/// electrical resistance which is measured by the ADC.
///
/// # Type Parameters
/// - `A`: ADC peripheral instance (e.g., ADC1, ADC2)
/// - `P`: ADC-compatible pin implementing `AdcChannel<A>`
///
/// # Example
/// ```rust
/// let p = embassy_stm32::init(Default::default());
/// let adc = Adc::new(p.ADC1);
/// let mut sensor = LightSensor::new(p.PB1, adc, 800);
///
/// if sensor.is_bright().await {
///     defmt::info!("Bright environment detected");
/// }
/// ```
pub struct LightSensor<A, P>
where
    A: Peripheral<P = A> + Instance + 'static,
    P: AdcChannel<A>,
{
    adc: Adc<'static, A>,
    pin: P,
    brightness_threshold: u16,
}

impl<A, P> LightSensor<A, P>
where
    A: Peripheral<P = A> + Instance + 'static,
    P: AdcChannel<A>,
{
    /// Creates a new light sensor instance
    ///
    /// # Arguments
    /// * `pin` - ADC-compatible pin connected to the LDR
    /// * `adc` - ADC peripheral instance
    /// * `brightness_threshold` - ADC value threshold that separates dark from bright environments
    ///                            (typical range: 0-4095 for 12-bit ADC)
    pub fn new(pin: P, adc: Adc<'static, A>, brightness_threshold: u16) -> Self {
        Self {
            adc,
            pin,
            brightness_threshold,
        }
    }

    /// Reads and returns the current light level classification
    pub async fn read_level(&mut self) -> LightLevel {
        if self.is_bright().await {
            LightLevel::Bright
        } else {
            LightLevel::Dark
        }
    }

    /// Checks if the environment is bright (above threshold)
    pub async fn is_bright(&mut self) -> bool {
        self.read_adc().await >= self.brightness_threshold
    }

    /// Checks if the environment is dark (below threshold)
    pub async fn is_dark(&mut self) -> bool {
        self.read_adc().await < self.brightness_threshold
    }

    /// Reads the raw ADC value from the sensor
    async fn read_adc(&mut self) -> u16 {
        self.adc.read(&mut self.pin).await
    }
}
