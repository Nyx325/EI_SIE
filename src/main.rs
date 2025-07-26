#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_futures::select;
use embassy_stm32::{exti::ExtiInput, gpio::Pull, init};
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = init(Default::default());

    // 1) Conecta la salida del RQ‑S003, p. ej. a PB1:
    let mut ir_rx = ExtiInput::new(p.PB1, p.EXTI1, Pull::Up);

    loop {
        // Detecta flanco de interrupción (cuando alguien bloquea el haz)
        ir_rx.wait_for_falling_edge().await;
        info!("🚶 Objeto en el haz");
        // Espera a que el haz vuelva a verse
        ir_rx.wait_for_rising_edge().await;
        info!("✔️ Haz restablecido");
    }
}
