#![no_std]
#![no_main]

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

// Todo el sistema se alimenta de una fuente
// de 3.3V
const VOLTAGE_REF: f32 = 3.3; // volts

// stm32 blue pill tiene un adc de 12 bits
const MAX_ADC_VALUE: f32 = 0b1111_1111_1111 as f32; // 4095.0

// Umbrales para el sensor
const LIGHT_THRESHOLD: f32 = 1000.; // Luxes
const DISTANCE_THRESHOLD: f32 = 2.5; // Metters

// Variables globales compartidas entre loop principal
// e interrupciones
static MANUAL_MODE: AtomicBool = AtomicBool::new(false);
static LIGHT: CriticalSectionMutex<Option<Output<'static>>> = CriticalSectionMutex::new(None);

// Convertir el valor del ADC a un voltaje
fn get_voltage(adc_value: f32) -> f32 {
    (adc_value / MAX_ADC_VALUE) * VOLTAGE_REF
}

// Valores de un sensor GP2Y0A710K0F
const DIST_MIN_V: f32 = 1.4; // 550 cm (5.5m)
const DIST_MAX_V: f32 = 2.5; // 100 cm (1.0m)

// Distancias correspondientes
const DIST_MIN_M: f32 = 5.5; // 5.5 metros (voltaje mínimo)
const DIST_MAX_M: f32 = 1.0; // 1.0 metro (voltaje máximo)

fn voltage_to_distance(voltage: f32) -> f32 {
    // Aplicamos saturación a los límites del sensor
    let clamped_voltage = voltage.clamp(DIST_MIN_V, DIST_MAX_V);

    // Mapeo lineal inverso (voltaje alto = distancia corta)
    let factor = (clamped_voltage - DIST_MIN_V) / (DIST_MAX_V - DIST_MIN_V);
    DIST_MIN_M + (DIST_MAX_M - DIST_MIN_M) * (1.0 - factor)
}

// Valores reales de un sensor DFRobot (DFR0026)
const LUX_MIN_V: f32 = 0.3; // 0 lux
const LUX_MAX_V: f32 = 3.0; // 6000 lux

const MAX_LUX_VALUE: f32 = 6000.;

fn voltage_to_lux(voltage: f32) -> f32 {
    // Aplicamos saturación a los límites del sensor
    let clamped_voltage = voltage.clamp(LUX_MIN_V, LUX_MAX_V);

    // Mapeo lineal directo
    let factor = (clamped_voltage - LUX_MIN_V) / (LUX_MAX_V - LUX_MIN_V);
    factor * MAX_LUX_VALUE
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    let mut adc = Adc::new(p.ADC1);

    // Pines asignados a los sensores
    let mut distance_sensor = p.PB0;
    let mut light_sensor = p.PA7;

    // Configurar un pin para EXTI
    let toggle_manual_btn = ExtiInput::new(p.PB13, p.EXTI13, Pull::Down);
    let toggle_light_btn = ExtiInput::new(p.PB12, p.EXTI12, Pull::Down);

    // Leds de salida
    let manual_mode_light = Output::new(p.PB5, Level::Low, Speed::Low);
    let light = Output::new(p.PB7, Level::Low, Speed::Low);

    // Inicializar variable global entre interrupciones
    unsafe { LIGHT.lock_mut(|l| *l = Some(light)) }

    // Inicializar interrupcion para establecer modo manual
    spawner
        .spawn(toggle_manual(toggle_manual_btn, manual_mode_light))
        .expect("Cannot create toggle_manual task");

    // Inicializar interrupcion para encender o apagar manualmente la luz
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
        let ambient_luminance = voltage_to_lux(luminicence_voltaje);

        defmt::info!(
            "Objeto a {} metros. Voltaje: {}",
            entity_distance,
            distance_voltage
        );
        defmt::info!(
            "Luminosidad de {} luxes. Voltaje {}",
            ambient_luminance,
            luminicence_voltaje
        );

        // Determinar si se enciende la luz
        let level = if ambient_luminance < LIGHT_THRESHOLD && entity_distance < DISTANCE_THRESHOLD {
            Level::High
        } else {
            Level::Low
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
