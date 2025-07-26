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

pub struct AnalogReader<A, P>
where
    A: Peripheral<P = A> + Instance + 'static,
    P: AdcChannel<A>,
{
    adc: Adc<'static, A>,
    pin: P,
}

impl<A, P> AnalogReader<A, P>
where
    A: Peripheral<P = A> + Instance + 'static,
    P: AdcChannel<A>,
{
    pub fn new(pin: P, adc: Adc<'static, A>) -> Self {
        Self { adc, pin }
    }

    pub async fn read(&mut self) -> u16 {
        self.adc.read(&mut self.pin).await
    }
}

pub struct LightSensor<A, P>
where
    A: Peripheral<P = A> + Instance + 'static,
    P: AdcChannel<A>,
{
    analog_reader: AnalogReader<A, P>,
    threshold: u16,
}

#[allow(dead_code)]
impl<A, P> LightSensor<A, P>
where
    A: Peripheral<P = A> + Instance + 'static,
    P: AdcChannel<A>,
{
    pub fn new(pin: P, adc: Adc<'static, A>, threshold: u16) -> Self {
        Self {
            analog_reader: AnalogReader::new(pin, adc),
            threshold,
        }
    }

    pub async fn read_level(&mut self) -> LightLevel {
        if self.is_bright().await {
            LightLevel::Bright
        } else {
            LightLevel::Dark
        }
    }

    pub async fn is_bright(&mut self) -> bool {
        self.analog_reader.read().await >= self.threshold
    }

    pub async fn is_dark(&mut self) -> bool {
        self.analog_reader.read().await < self.threshold
    }
}
