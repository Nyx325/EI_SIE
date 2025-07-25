#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::{
    adc::Adc,
};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    let mut adc = Adc::new(p.ADC1);
    let mut potenciometer = p.PB1;

    loop {
        let value = adc.read(&mut potenciometer).await;
        info!("Valor potenciometro: {}", value);
        Timer::after_millis(500).await;
    }
}
