#![no_std]
#![no_main]

mod sensors;

use crate::sensors::{AnalogReader, LightSensor};
use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::adc::Adc;
use embassy_time::Timer;

use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    let adc = Adc::new(p.ADC1);
    let mut light_sensor = LightSensor::new(p.PB0, adc, 800);

    let adc = Adc::new(p.ADC2);
    let mut analog_reader = AnalogReader::new(p.PB1, adc);

    loop {
        let value = analog_reader.read().await;
        info!("Valor potenciometro: {}", value);
        Timer::after_millis(500).await;
    }
}
