#![no_std]
#![no_main]

pub mod sensors;

use core::sync::atomic::{AtomicBool, Ordering};

use embassy_executor::Spawner;
use embassy_stm32::{
    adc::Adc,
    exti::ExtiInput,
    gpio::{Level, Output, Pull, Speed},
};
use embassy_sync::blocking_mutex::CriticalSectionMutex;
use embassy_time::Timer;

use {defmt_rtt as _, panic_probe as _};

const VOLTAGE_REF: f32 = 3.3; // volts

// stm32 blue pill tiene un adc de 12 bits
const MAX_ADC_VALUE: f32 = 0b1111_1111_1111 as f32; // 4095.0

const LIGHT_THRESHOLD: f32 = 50.; // Luxes
const DISTANCE_THRESHOLD: f32 = 2.5; // Metters

static MANUAL_MODE: AtomicBool = AtomicBool::new(false);
static LIGHT: CriticalSectionMutex<Option<Output<'static>>> = CriticalSectionMutex::new(None);

fn get_voltage(adc_value: f32) -> f32 {
    (adc_value / MAX_ADC_VALUE) * VOLTAGE_REF
}

// Valores minimos y maximos de un sensor GP2Y0A710K0F
const DIST_MIN_V: f32 = 0.4; // 5.5 metros (550 cm)
const DIST_MAX_V: f32 = 2.6; // 1.0 metro (100 cm)

fn voltage_to_distance(voltage: f32) -> f32 {
    // Aplicar límites físicos del sensor
    let clamped_voltage = voltage.clamp(DIST_MIN_V, DIST_MAX_V);

    // Fórmula calibrada (ajustar según tu sensor)
    let distance_cm = 9462.4 / (clamped_voltage - 0.16);

    // Convertir a metros y retornar
    distance_cm / 100.0
}

const LUX_MIN_V: f32 = 0.3; // 0 lux
const LUX_MAX_V: f32 = 3.0; // 6000 lux

fn voltage_to_lux(voltage: f32) -> f32 {
    match voltage {
        /* cuando el sensor alcanza sus
         * límites físicos y no puede medir
         * más allá de esos valores */
        v if v <= LUX_MIN_V => 0.0, // Saturación inferior

        /* Cuando la luz ambiental es
         * tan baja que el sensor no puede
         * detectar diferencias */
        v if v >= LUX_MAX_V => 6000.0, // Saturación superior

        // Caso comun
        v => ((v - 0.3) / 2.7) * 6000.0, // Rango lineal
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    let mut adc = Adc::new(p.ADC1);

    let mut distance_sensor = p.PB0;
    let mut light_sensor = p.PA7;

    let toggle_manual_btn = ExtiInput::new(p.PB13, p.EXTI13, Pull::Down);
    let toggle_light_btn = ExtiInput::new(p.PB12, p.EXTI12, Pull::Down);

    let manual_mode_light = Output::new(p.PB5, Level::Low, Speed::Low);
    let light = Output::new(p.PB6, Level::Low, Speed::Low);

    unsafe { LIGHT.lock_mut(|l| *l = Some(light)) }

    spawner
        .spawn(toggle_manual(toggle_manual_btn, manual_mode_light))
        .expect("Cannot create toggle_manual task");

    spawner
        .spawn(toggle_light(toggle_light_btn))
        .expect("Cannot create toggle_manual task");

    loop {
        Timer::after_millis(100).await;
        if MANUAL_MODE.load(Ordering::Relaxed) {
            continue;
        }

        let raw_distance = adc.read(&mut distance_sensor).await;
        let raw_luminicence = adc.read(&mut light_sensor).await;

        let distance_voltage = get_voltage(raw_distance as f32);
        let luminicence_voltaje = get_voltage(raw_luminicence as f32);

        let entity_distance = voltage_to_distance(distance_voltage);
        let ambient_luxes = voltage_to_lux(luminicence_voltaje);

        defmt::info!("Objeto a {} metros - Voltaje: {}", entity_distance, distance_voltage);
        defmt::info!("Luminosidad de {} luxes - Voltaje {}", ambient_luxes, luminicence_voltaje);

        let level = match (entity_distance, ambient_luxes) {
            (d, l) if l < LIGHT_THRESHOLD && d < DISTANCE_THRESHOLD => Level::High,
            _ => Level::Low,
        };

        unsafe {
            LIGHT.lock_mut(|l| {
                if let Some(l) = l {
                    l.set_level(level);
                }
            })
        }
    }
}

#[embassy_executor::task]
async fn toggle_manual(
    mut toggle_manual_btn: ExtiInput<'static>,
    mut manual_mode_light: Output<'static>,
) {
    loop {
        Timer::after_millis(100).await;
        toggle_manual_btn.wait_for_falling_edge().await;
        Timer::after_millis(50).await;

        let current = MANUAL_MODE.load(Ordering::Relaxed);
        MANUAL_MODE.store(!current, Ordering::Relaxed);
        manual_mode_light.toggle();
        defmt::info!("Modo manual {}", manual_mode_light.is_set_high());
    }
}

#[embassy_executor::task]
async fn toggle_light(mut toggle_light_btn: ExtiInput<'static>) {
    loop {
        Timer::after_millis(100).await;
        toggle_light_btn.wait_for_falling_edge().await;
        Timer::after_millis(50).await;

        let manual = MANUAL_MODE.load(Ordering::Relaxed);
        if !manual {
            continue;
        }

        Timer::after_millis(10).await;
        unsafe {
            LIGHT.lock_mut(|l| {
                if let Some(l) = l {
                    l.toggle();
                    defmt::info!("Foco encendido: {}", l.is_set_high());
                }
            })
        }
    }
}
