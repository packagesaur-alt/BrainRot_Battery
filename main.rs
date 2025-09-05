use std::collections::VecDeque;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use clap::{Arg, Command};
use serde::{Deserialize, Serialize};

/// Convert Celsius to Fahrenheit
fn celsius_to_fahrenheit(celsius: f64) -> f64 {
    (celsius * 9.0 / 5.0) + 32.0
}

/// Generate Pac-Man cat animation based on elapsed time
fn generate_pacman_cat_animation(elapsed_secs: u64) -> String {
    // Calculate dots eaten based on elapsed seconds (1 dot per second)
    let dots_eaten = (elapsed_secs as usize).min(TOTAL_DOTS);
    let remaining_dots = TOTAL_DOTS - dots_eaten;
    
    // Animated cat with moving mouth - more frames for smoother animation
    let cat = match elapsed_secs % 4 {
        0 => "C",  // Closed mouth
        1 => "c",  // Slightly open
        2 => "o",  // Open mouth eating
        3 => "O",  // Wide open eating
        _ => "C",
    };
    
    let remaining_dots_str = "●".repeat(remaining_dots);
    
    if remaining_dots == 0 {
        format!("All dots eaten!")
    } else {
        format!("{}{}", cat, remaining_dots_str)
    }
}

/// Generate countdown dots that disappear one by one
fn generate_countdown_dots(elapsed_secs: u64) -> String {
    let remaining_seconds = PROGRAM_DURATION_SECS.saturating_sub(elapsed_secs);
    let remaining_dots = remaining_seconds as usize;
    let disappeared_dots = (PROGRAM_DURATION_SECS - remaining_seconds) as usize;
    
    let disappeared_spaces = " ".repeat(disappeared_dots);
    let remaining_dots_str = "●".repeat(remaining_dots);
    
    format!("{}[{}] {}s remaining", disappeared_spaces, remaining_dots_str, remaining_seconds)
}



/// Configuration constants for smoothing and accuracy
const POWER_SMOOTHING_ALPHA: f64 = 0.25; // Exponential moving average factor (optimized)
const MIN_POWER_THRESHOLD: f64 = 0.05; // Minimum power in watts for calculations (more sensitive)
const MAX_HISTORY_SIZE: usize = 300; // 5 minutes at 1s intervals
const UPDATE_INTERVAL_SECS: u64 = 2; // Update every 2 seconds
const PROGRAM_DURATION_SECS: u64 = 20; // Stop program after 20 seconds
const MIN_SAMPLES_FOR_ESTIMATE: usize = 3; // Minimum samples before showing estimate
const ROLLING_WINDOW_SIZE: usize = 10; // Rolling average window for ultra-smooth estimates
const MIN_VALID_TEMP: f64 = 10.0; // Minimum valid temperature in Celsius
const MAX_VALID_TEMP: f64 = 110.0; // Maximum valid temperature in Celsius
const TOTAL_DOTS: usize = 20; // Total dots for Pac-Man cat animation

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryReading {
    pub timestamp: u64,
    pub capacity_percent: u8,
    pub energy_now_wh: Option<f64>,
    pub energy_full_wh: Option<f64>,
    pub power_now_w: Option<f64>,
    pub voltage_v: Option<f64>,
    pub current_ma: Option<i32>,
    pub status: String,
    pub temperature_c: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryInfo {
    pub status: String,
    pub capacity_percent: u8,
    pub health_percent: f64,
    pub cycles: Option<u32>,
    pub temperature_c: Option<f64>,
    pub voltage_v: Option<f64>,
    pub current_ma: Option<i32>,
    pub power_w: Option<f64>,
    pub smoothed_power_w: Option<f64>,
    pub manufacturer: String,
    pub model: String,
    pub technology: String,
    pub time_remaining_minutes: Option<u32>,
    pub energy_now_wh: Option<f64>,
    pub energy_full_wh: Option<f64>,
    pub power_trend: String, // "stable", "increasing", "decreasing"
    pub cpu_temperature_c: Option<f64>,
}

#[derive(Debug)]
pub struct PowerSample {
    pub timestamp: u64,
    pub power_w: f64,
    pub energy_wh: f64,
}

#[derive(Debug, Clone)]
pub struct TemperatureSensor {
    pub sensor_type: String, // "coretemp", "k10temp", "battery", etc.
    pub path: String,
    pub label: Option<String>,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct TemperatureReading {
    pub raw_value: f64,
    pub smoothed_value: f64,
    pub sensor_info: TemperatureSensor,
    pub timestamp: u64,
}

#[derive(Debug)]
pub struct TemperatureMonitor {
    pub cpu_sensors: Vec<TemperatureSensor>,
    pub battery_sensors: Vec<TemperatureSensor>,
    pub last_cpu_temp: Option<TemperatureReading>,
    pub last_battery_temp: Option<TemperatureReading>,
}

impl TemperatureMonitor {
    pub fn new() -> Self {
        let mut monitor = Self {
            cpu_sensors: Vec::new(),
            battery_sensors: Vec::new(),
            last_cpu_temp: None,
            last_battery_temp: None,
        };
        monitor.discover_sensors();
        monitor
    }

    /// Comprehensive sensor discovery with detailed logging
    fn discover_sensors(&mut self) {
        println!("🔍 Discovering temperature sensors...");
        
        // Discover CPU sensors from hwmon
        self.discover_cpu_sensors();
        
        // Discover battery sensors
        self.discover_battery_sensors();
        
        // Log discovery results
        if self.cpu_sensors.is_empty() && self.battery_sensors.is_empty() {
            println!("⚠️  No temperature sensors found!");
        } else {
            println!("✅ Temperature sensor discovery complete:");
            for sensor in &self.cpu_sensors {
                println!("   CPU: {} ({})", sensor.name, sensor.path);
            }
            for sensor in &self.battery_sensors {
                println!("   BAT: {} ({})", sensor.name, sensor.path);
            }
        }
    }

    /// Scan /sys/class/hwmon/hwmon*/name for CPU temperature sensors
    fn discover_cpu_sensors(&mut self) {
        let hwmon_path = Path::new("/sys/class/hwmon");
        if !hwmon_path.exists() {
            println!("❌ /sys/class/hwmon not found - ensure you're running on Linux");
            return;
        }

        println!("🔍 Scanning /sys/class/hwmon/ for temperature sensors...");
        
        if let Ok(entries) = fs::read_dir(hwmon_path) {
            let mut hwmon_dirs: Vec<_> = entries.filter_map(|e| e.ok()).collect();
            hwmon_dirs.sort_by_key(|e| e.file_name());
            
            for entry in hwmon_dirs {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with("hwmon") {
                        println!("🔍 Found hwmon directory: {}", entry.path().display());
                        self.scan_hwmon_device(&entry.path());
                    }
                }
            }
        } else {
            println!("❌ Failed to read /sys/class/hwmon directory");
        }

        if self.cpu_sensors.is_empty() {
            println!("⚠️  No CPU temperature sensors found in /sys/class/hwmon/");
        } else {
            // Sort CPU sensors by preference: coretemp > k10temp > others
            self.cpu_sensors.sort_by(|a, b| {
                let priority_a = match a.sensor_type.as_str() {
                    "coretemp" => 1,   // Intel - highest priority
                    "k10temp" => 2,    // AMD Ryzen
                    "zenpower" => 3,   // AMD alternative
                    "amdgpu" => 4,     // AMD GPU (if needed)
                    _ => 9,            // Others - lowest priority
                };
                let priority_b = match b.sensor_type.as_str() {
                    "coretemp" => 1,   // Intel - highest priority
                    "k10temp" => 2,    // AMD Ryzen
                    "zenpower" => 3,   // AMD alternative
                    "amdgpu" => 4,     // AMD GPU (if needed)
                    _ => 9,            // Others - lowest priority
                };
                priority_a.cmp(&priority_b)
            });
            
            println!("📊 CPU sensors sorted by priority:");
            for (i, sensor) in self.cpu_sensors.iter().enumerate() {
                println!("   {}. {} [{}]", i+1, sensor.name, sensor.path);
            }
        }
    }


    fn scan_hwmon_device(&mut self, hwmon_path: &Path) {
        // Read the device name
        let name_path = hwmon_path.join("name");
        let device_name = match fs::read_to_string(&name_path) {
            Ok(name) => name.trim().to_string(),
            Err(e) => {
                println!("❌ Cannot read name from {}: {}", name_path.display(), e);
                return;
            }
        };

        println!("🔍 Scanning hwmon device: '{}' at {}", device_name, hwmon_path.display());

        // Skip virtual/invalid sensors with explicit logging
        if device_name == "acpitz" || device_name.contains("virtual") {
            println!("🚫 Skipping virtual/ACPI sensor: '{}' (not a real temperature sensor)", device_name);
            return;
        }

        // Look for temperature inputs with detailed scanning
        if let Ok(entries) = fs::read_dir(hwmon_path) {
            let mut temp_files: Vec<_> = entries.filter_map(|e| e.ok()).collect();
            temp_files.sort_by_key(|e| e.file_name());
            
            let mut found_temp_inputs = Vec::new();
            
            for entry in temp_files {
                let filename = entry.file_name();
                let filename_str = filename.to_string_lossy();
                
                if filename_str.starts_with("temp") && filename_str.ends_with("_input") {
                    found_temp_inputs.push(filename_str.to_string());
                }
            }
            
            if found_temp_inputs.is_empty() {
                println!("   ❌ No temp*_input files found in {}", hwmon_path.display());
                return;
            }
            
            println!("   📊 Found temp inputs: {:?}", found_temp_inputs);
            
            for temp_input in found_temp_inputs {
                // Extract temp number (e.g., temp1_input -> 1)
                if let Some(temp_num) = temp_input.strip_prefix("temp").and_then(|s| s.strip_suffix("_input")) {
                    let temp_input_path = hwmon_path.join(&temp_input);
                    let label_path = hwmon_path.join(format!("temp{}_label", temp_num));
                    
                    // Read label if available
                    let label = match fs::read_to_string(&label_path) {
                        Ok(l) => {
                            let label_str = l.trim().to_string();
                            println!("   🏷️  temp{}_label = '{}'", temp_num, label_str);
                            Some(label_str)
                        }
                        Err(_) => {
                            println!("   ❌ No temp{}_label file (using temp{})", temp_num, temp_num);
                            None
                        }
                    };
                    
                    // Check if this is a CPU temperature we want
                    if self.is_cpu_temp_sensor(&device_name, &label) {
                        let sensor = TemperatureSensor {
                            sensor_type: device_name.clone(),
                            path: temp_input_path.to_string_lossy().to_string(),
                            label: label.clone(),
                            name: format!("{} {}", device_name, label.unwrap_or_else(|| format!("temp{}", temp_num))),
                        };
                        
                        // Test if we can actually read from this sensor
                        println!("   🧪 Testing sensor: {} -> {}", sensor.name, sensor.path);
                        match self.read_temperature_from_path(&sensor.path) {
                            Some(raw_temp) => {
                                let temp_celsius = raw_temp / 1000.0; // Convert millidegrees to Celsius
                                if self.is_valid_temperature(temp_celsius) {
                                    println!("   ✅ VALID CPU sensor: {} = {:.1}°C (raw: {})", sensor.name, temp_celsius, raw_temp);
                                    self.cpu_sensors.push(sensor);
                                } else {
                                    println!("   🚫 INVALID temperature from {}: {:.1}°C (outside {}-{}°C range)", 
                                        sensor.name, temp_celsius, MIN_VALID_TEMP, MAX_VALID_TEMP);
                                }
                            }
                            None => {
                                println!("   ❌ Cannot read from sensor: {} (file: {})", sensor.name, sensor.path);
                            }
                        }
                    } else {
                        println!("   🚫 Skipping temp{}: '{}' sensor '{}' with label '{:?}' (not a main CPU sensor)", 
                            temp_num, device_name, temp_input, label);
                    }
                }
            }
        } else {
            println!("   ❌ Cannot read directory contents of {}", hwmon_path.display());
        }
    }

    fn is_cpu_temp_sensor(&self, device_name: &str, label: &Option<String>) -> bool {
        println!("   🔍 Checking if '{}' with label '{:?}' is a CPU sensor", device_name, label);
        
        // Check device name first
        match device_name {
            "coretemp" => {
                // Intel CPU temperature sensor
                if let Some(ref label_str) = label {
                    let label_lower = label_str.to_lowercase();
                    // Look for package temperature specifically
                    let is_package = label_lower.contains("package") || 
                                   label_lower == "package id 0" ||
                                   label_lower.contains("package id");
                    println!("   📊 coretemp label '{}' -> package sensor: {}", label_str, is_package);
                    is_package
                } else {
                    // If no label, assume temp1 is the main package sensor for coretemp
                    println!("   📊 coretemp with no label -> assuming main package sensor");
                    true
                }
            }
            "k10temp" => {
                // AMD Ryzen temperature sensor
                if let Some(ref label_str) = label {
                    let label_lower = label_str.to_lowercase();
                    // Look for main die temperature (Tctl or Tdie)
                    let is_main = label_lower.contains("tctl") || 
                                label_lower.contains("tdie") ||
                                label_lower == "tctl" ||
                                label_lower == "tdie";
                    println!("   📊 k10temp label '{}' -> main sensor: {}", label_str, is_main);
                    is_main
                } else {
                    // If no label, assume temp1 is the main sensor for k10temp
                    println!("   📊 k10temp with no label -> assuming main sensor");
                    true
                }
            }
            "zenpower" => {
                // AMD alternative temperature sensor
                if let Some(ref label_str) = label {
                    let label_lower = label_str.to_lowercase();
                    let is_main = label_lower.contains("tctl") || 
                                label_lower.contains("tdie") ||
                                label_lower.contains("die");
                    println!("   📊 zenpower label '{}' -> main sensor: {}", label_str, is_main);
                    is_main
                } else {
                    println!("   📊 zenpower with no label -> assuming main sensor");
                    true
                }
            }
            "amdgpu" => {
                // AMD GPU temperature - only if specifically requested
                if let Some(ref label_str) = label {
                    let is_gpu = label_str.to_lowercase().contains("edge");
                    println!("   📊 amdgpu label '{}' -> GPU edge sensor: {}", label_str, is_gpu);
                    is_gpu
                } else {
                    println!("   🚫 amdgpu with no label -> skipping");
                    false
                }
            }
            _ => {
                println!("   🚫 Unknown device type '{}' -> skipping", device_name);
                false
            }
        }
    }

    /// Discover battery temperature sensors
    fn discover_battery_sensors(&mut self) {
        println!("🔍 Scanning for battery temperature sensors...");
        
        // Method 1: Direct battery power supply sensors
        let power_supply_path = Path::new("/sys/class/power_supply");
        println!("🔍 Checking /sys/class/power_supply/ for battery temp sensors...");
        
        if let Ok(entries) = fs::read_dir(power_supply_path) {
            let mut power_entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
            power_entries.sort_by_key(|e| e.file_name());
            
            for entry in power_entries {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                
                if name_str.starts_with("BAT") || name_str.starts_with("battery") {
                    println!("🔍 Found battery device: {}", name_str);
                    let temp_path = entry.path().join("temp");
                    
                    if temp_path.exists() {
                        println!("   📊 Found temp file: {}", temp_path.display());
                        let sensor = TemperatureSensor {
                            sensor_type: "battery".to_string(),
                            path: temp_path.to_string_lossy().to_string(),
                            label: Some(name_str.to_string()),
                            name: format!("Battery {}", name_str),
                        };
                        
                        // Test the sensor
                        println!("   🧪 Testing battery sensor: {} -> {}", sensor.name, sensor.path);
                        match self.read_temperature_from_path(&sensor.path) {
                            Some(raw_temp) => {
                                let normalized_temp = self.normalize_battery_temperature(raw_temp);
                                println!("   📊 Raw temp: {}, normalized: {:.1}°C", raw_temp, normalized_temp);
                                
                                if self.is_valid_temperature(normalized_temp) {
                                    println!("   ✅ VALID battery sensor: {} = {:.1}°C", sensor.name, normalized_temp);
                                    self.battery_sensors.push(sensor);
                                } else {
                                    println!("   🚫 INVALID battery temperature: {:.1}°C (outside {}-{}°C range)", 
                                        normalized_temp, MIN_VALID_TEMP, MAX_VALID_TEMP);
                                }
                            }
                            None => {
                                println!("   ❌ Cannot read from battery sensor: {}", sensor.path);
                            }
                        }
                    } else {
                        println!("   ❌ No temp file found for battery {}", name_str);
                    }
                } else {
                    println!("🚫 Skipping non-battery device: {}", name_str);
                }
            }
        } else {
            println!("❌ Cannot read /sys/class/power_supply directory");
        }

        // Method 2: Thermal zones with type=battery
        let thermal_path = Path::new("/sys/class/thermal");
        println!("🔍 Checking /sys/class/thermal/ for battery thermal zones...");
        
        if let Ok(entries) = fs::read_dir(thermal_path) {
            let mut thermal_entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
            thermal_entries.sort_by_key(|e| e.file_name());
            
            for entry in thermal_entries {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                
                if name_str.starts_with("thermal_zone") {
                    let type_path = entry.path().join("type");
                    match fs::read_to_string(&type_path) {
                        Ok(zone_type_raw) => {
                            let zone_type = zone_type_raw.trim();
                            println!("🔍 thermal_zone {} type: '{}'", name_str, zone_type);
                            
                            if zone_type == "battery" {
                                let temp_path = entry.path().join("temp");
                                if temp_path.exists() {
                                    println!("   📊 Found battery thermal zone temp file: {}", temp_path.display());
                                    let sensor = TemperatureSensor {
                                        sensor_type: "thermal_zone".to_string(),
                                        path: temp_path.to_string_lossy().to_string(),
                                        label: Some(zone_type.to_string()),
                                        name: format!("Battery Thermal {}", name_str),
                                    };
                                    
                                    println!("   🧪 Testing thermal zone sensor: {} -> {}", sensor.name, sensor.path);
                                    match self.read_temperature_from_path(&sensor.path) {
                                        Some(raw_temp) => {
                                            let normalized_temp = self.normalize_battery_temperature(raw_temp);
                                            println!("   📊 Raw temp: {}, normalized: {:.1}°C", raw_temp, normalized_temp);
                                            
                                            if self.is_valid_temperature(normalized_temp) {
                                                println!("   ✅ VALID battery thermal zone: {} = {:.1}°C", sensor.name, normalized_temp);
                                                self.battery_sensors.push(sensor);
                                            } else {
                                                println!("   🚫 INVALID thermal zone temperature: {:.1}°C", normalized_temp);
                                            }
                                        }
                                        None => {
                                            println!("   ❌ Cannot read from thermal zone: {}", sensor.path);
                                        }
                                    }
                                } else {
                                    println!("   ❌ No temp file in thermal zone {}", name_str);
                                }
                            } else {
                                println!("   🚫 Skipping thermal zone {} (type: '{}')", name_str, zone_type);
                            }
                        }
                        Err(e) => {
                            println!("   ❌ Cannot read type from {}: {}", type_path.display(), e);
                        }
                    }
                }
            }
        } else {
            println!("❌ Cannot read /sys/class/thermal directory");
        }
        
        if self.battery_sensors.is_empty() {
            println!("⚠️  No battery temperature sensors found");
        } else {
            println!("📊 Found {} battery sensor(s):", self.battery_sensors.len());
            for (i, sensor) in self.battery_sensors.iter().enumerate() {
                println!("   {}. {} [{}]", i+1, sensor.name, sensor.path);
            }
        }
    }

    fn read_temperature_from_path(&self, path: &str) -> Option<f64> {
        fs::read_to_string(path)
            .ok()?
            .trim()
            .parse::<f64>()
            .ok()
    }

    fn normalize_battery_temperature(&self, raw_value: f64) -> f64 {
        if raw_value > 1000.0 {
            // Millidegrees Celsius - divide by 1000
            let normalized = raw_value / 1000.0;
            println!("   🔄 Normalized battery temp: {} (millidegrees) -> {:.1}°C", raw_value, normalized);
            normalized
        } else if raw_value > 200.0 {
            // Decidegrees Celsius - divide by 10
            let normalized = raw_value / 10.0;
            println!("   🔄 Normalized battery temp: {} (decidegrees) -> {:.1}°C", raw_value, normalized);
            normalized
        } else {
            // Already in Celsius
            println!("   ✅ Battery temp already in Celsius: {:.1}°C", raw_value);
            raw_value
        }
    }

    fn is_valid_temperature(&self, temp: f64) -> bool {
        temp >= MIN_VALID_TEMP && temp <= MAX_VALID_TEMP
    }

    /// Get current CPU temperature (raw value only)
    pub fn get_cpu_temp(&mut self) -> Option<TemperatureReading> {
        for sensor in &self.cpu_sensors {
            if let Some(raw_temp) = self.read_temperature_from_path(&sensor.path) {
                let temp_celsius = raw_temp / 1000.0; // Convert millidegrees to Celsius
                
                if self.is_valid_temperature(temp_celsius) {
                    let reading = TemperatureReading {
                        raw_value: temp_celsius,
                        smoothed_value: temp_celsius, // No averaging - same as raw
                        sensor_info: sensor.clone(),
                        timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                    };
                    
                    self.last_cpu_temp = Some(reading.clone());
                    return Some(reading);
                }
            }
        }
        None
    }

    /// Get current battery temperature (raw value only)
    pub fn get_battery_temp(&mut self) -> Option<TemperatureReading> {
        for sensor in &self.battery_sensors {
            if let Some(raw_temp) = self.read_temperature_from_path(&sensor.path) {
                let temp_celsius = self.normalize_battery_temperature(raw_temp);
                
                if self.is_valid_temperature(temp_celsius) {
                    let reading = TemperatureReading {
                        raw_value: temp_celsius,
                        smoothed_value: temp_celsius, // No averaging - same as raw
                        sensor_info: sensor.clone(),
                        timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                    };
                    
                    self.last_battery_temp = Some(reading.clone());
                    return Some(reading);
                }
            }
        }
        None
    }
}

pub struct BatteryMonitor {
    base_path: String,
    readings_history: VecDeque<BatteryReading>,
    power_history: VecDeque<PowerSample>,
    smoothed_power: Option<f64>,
    rolling_power_window: VecDeque<f64>,
    temperature_monitor: TemperatureMonitor,
    max_history: usize,
    last_update: u64,
}

impl BatteryMonitor {
    pub fn new(battery_name: &str) -> Self {
        Self {
            base_path: format!("/sys/class/power_supply/{}", battery_name),
            readings_history: VecDeque::new(),
            power_history: VecDeque::new(),
            smoothed_power: None,
            rolling_power_window: VecDeque::new(),
            temperature_monitor: TemperatureMonitor::new(),
            max_history: MAX_HISTORY_SIZE,
            last_update: 0,
        }
    }

    fn read_file(&self, filename: &str) -> Option<String> {
        let path = format!("{}/{}", self.base_path, filename);
        fs::read_to_string(path).ok().map(|s| s.trim().to_string())
    }

    fn read_as_number<T: std::str::FromStr>(&self, filename: &str) -> Option<T> {
        self.read_file(filename)?.parse().ok()
    }

    /// Read energy values with fallback between energy_* and charge_* files
    fn read_energy_values(&self) -> (Option<f64>, Option<f64>) {
        // Try energy_* first (preferred for modern systems)
        let energy_now = self.read_as_number::<f64>("energy_now")
            .map(|e| e / 1_000_000.0) // Convert µWh to Wh
            .or_else(|| {
                // Fallback: charge_now * voltage_now
                let charge = self.read_as_number::<f64>("charge_now")?;
                let voltage = self.read_as_number::<f64>("voltage_now")?;
                Some((charge * voltage) / 1_000_000_000_000.0) // µAh * µV to Wh
            });

        let energy_full = self.read_as_number::<f64>("energy_full")
            .map(|e| e / 1_000_000.0) // Convert µWh to Wh
            .or_else(|| {
                // Fallback: charge_full * voltage_now
                let charge = self.read_as_number::<f64>("charge_full")?;
                let voltage = self.read_as_number::<f64>("voltage_now")?;
                Some((charge * voltage) / 1_000_000_000_000.0) // µAh * µV to Wh
            });

        (energy_now, energy_full)
    }

    /// Read power with multiple fallback methods using instantaneous values
    fn read_power(&self, voltage_v: Option<f64>, current_ma: Option<i32>) -> Option<f64> {
        // Method 1: Direct power reading (most accurate)
        if let Some(power_uw) = self.read_as_number::<f64>("power_now") {
            return Some(power_uw / 1_000_000.0); // Convert µW to W
        }

        // Method 2: Instantaneous Power = Voltage × Current (most reliable for time estimation)
        if let (Some(voltage), Some(current)) = (voltage_v, current_ma) {
            let power_w = voltage * (current.abs() as f64 / 1000.0); // V * |A| = W
            return Some(power_w);
        }

        None
    }

    /// Get CPU temperature using the new temperature monitor
    pub fn get_cpu_temperature(&mut self) -> Option<TemperatureReading> {
        self.temperature_monitor.get_cpu_temp()
    }

    /// Get battery temperature using the new temperature monitor
    pub fn get_battery_temperature(&mut self) -> Option<TemperatureReading> {
        self.temperature_monitor.get_battery_temp()
    }

    /// Update smoothed power using exponential moving average and rolling window
    fn update_smoothed_power(&mut self, current_power: f64) {
        // Update exponential moving average
        self.smoothed_power = Some(match self.smoothed_power {
            Some(prev) => POWER_SMOOTHING_ALPHA * current_power + (1.0 - POWER_SMOOTHING_ALPHA) * prev,
            None => current_power,
        });

        // Update rolling window for ultra-smooth estimates
        self.rolling_power_window.push_back(current_power);
        if self.rolling_power_window.len() > ROLLING_WINDOW_SIZE {
            self.rolling_power_window.pop_front();
        }
    }


    /// Get rolling average power for ultra-stable estimates
    fn get_rolling_average_power(&self) -> Option<f64> {
        if self.rolling_power_window.len() < 3 {
            return self.smoothed_power;
        }
        
        let sum: f64 = self.rolling_power_window.iter().sum();
        Some(sum / self.rolling_power_window.len() as f64)
    }

    /// Calculate highly accurate time remaining using multiple smoothing techniques
    fn calculate_time_remaining(&self, info: &BatteryReading) -> Option<u32> {
        let instantaneous_power = info.power_now_w?;
        let smoothed_power = self.smoothed_power?;
        let rolling_power = self.get_rolling_average_power()?;
        
        // Skip calculation if power is too low (likely noise or system idle)
        if instantaneous_power.abs() < MIN_POWER_THRESHOLD {
            return None;
        }

        // Need enough samples for reliable estimate
        if self.power_history.len() < MIN_SAMPLES_FOR_ESTIMATE {
            return None;
        }

        // Advanced weighted power calculation for maximum accuracy
        let weighted_power = if self.power_history.len() < 5 {
            // Very early: mostly instantaneous for quick adaptation
            0.8 * instantaneous_power + 0.2 * smoothed_power
        } else if self.power_history.len() < ROLLING_WINDOW_SIZE {
            // Early: balance instantaneous and smoothed
            0.5 * instantaneous_power + 0.5 * smoothed_power
        } else {
            // Mature: use all three methods for ultra-stable estimates
            0.2 * instantaneous_power + 0.3 * smoothed_power + 0.5 * rolling_power
        };

        match info.status.as_str() {
            "Discharging" => {
                if let Some(energy_now) = info.energy_now_wh {
                    if weighted_power > 0.0 {
                        // Time to drain = Current Energy / Power Consumption
                        let hours = energy_now / weighted_power;
                        Some((hours * 60.0).max(1.0) as u32) // At least 1 minute
                    } else {
                        None
                    }
                } else {
                    // Fallback: use capacity percentage if energy not available
                    if let (Some(voltage), Some(current)) = (info.voltage_v, info.current_ma) {
                        if current < 0 && voltage > 0.0 {
                            // Estimate based on capacity and current draw
                            let capacity_fraction = info.capacity_percent as f64 / 100.0;
                            let estimated_energy = voltage * 3.0 * capacity_fraction; // Rough 3Ah estimate
                            let power = voltage * ((-current) as f64 / 1000.0);
                            if power > MIN_POWER_THRESHOLD {
                                let hours = estimated_energy / power;
                                return Some((hours * 60.0).max(1.0) as u32);
                            }
                        }
                    }
                    None
                }
            }
            "Charging" => {
                if let (Some(energy_now), Some(energy_full)) = (info.energy_now_wh, info.energy_full_wh) {
                    if weighted_power > 0.0 {
                        let energy_to_charge = energy_full - energy_now;
                        
                        // Advanced charging calculation considering charging curve
                        let charge_progress = energy_now / energy_full;
                        let charging_efficiency = if charge_progress > 0.8 {
                            // Charging slows down significantly above 80%
                            0.6 + (0.9 - charge_progress) * 2.0 // Efficiency drops as we approach 100%
                        } else if charge_progress > 0.95 {
                            // Trickle charge phase
                            0.3
                        } else {
                            // Normal charging phase
                            0.9
                        };
                        
                        let effective_power = weighted_power * charging_efficiency;
                        let hours = energy_to_charge / effective_power;
                        Some((hours * 60.0).max(1.0) as u32) // At least 1 minute
                    } else {
                        None
                    }
                } else {
                    // Enhanced fallback for systems without energy readings
                    if let (Some(voltage), Some(current)) = (info.voltage_v, info.current_ma) {
                        if current > 0 && voltage > 0.0 {
                            let remaining_capacity = (100 - info.capacity_percent) as f64 / 100.0;
                            
                            // Better capacity estimation based on voltage
                            let estimated_full_capacity = match voltage {
                                v if v > 12.0 => 4.0, // Larger battery
                                v if v > 7.0 => 3.0,  // Standard laptop battery
                                _ => 2.0,             // Smaller battery
                            };
                            
                            let estimated_energy_needed = voltage * estimated_full_capacity * remaining_capacity;
                            let power = voltage * (current as f64 / 1000.0);
                            
                            // Apply charging curve to fallback calculation too
                            let charge_progress = info.capacity_percent as f64 / 100.0;
                            let efficiency = if charge_progress > 0.8 { 0.7 } else { 0.9 };
                            let effective_power = power * efficiency;
                            
                            if effective_power > MIN_POWER_THRESHOLD {
                                let hours = estimated_energy_needed / effective_power;
                                return Some((hours * 60.0).max(1.0) as u32);
                            }
                        }
                    }
                    None
                }
            }
            "Not charging" | "Full" => {
                // Battery is full or not charging
                None
            }
            _ => None,
        }
    }

    /// Determine power trend from recent history
    fn get_power_trend(&self) -> String {
        if self.power_history.len() < 5 {
            return "stable".to_string();
        }

        let recent: Vec<&PowerSample> = self.power_history.iter().rev().take(5).collect();
        let power_changes: Vec<f64> = recent.windows(2)
            .map(|w| w[0].power_w - w[1].power_w)
            .collect();

        let avg_change: f64 = power_changes.iter().sum::<f64>() / power_changes.len() as f64;

        if avg_change > 0.5 {
            "increasing".to_string()
        } else if avg_change < -0.5 {
            "decreasing".to_string()
        } else {
            "stable".to_string()
        }
    }

    pub fn get_battery_info(&mut self) -> Option<BatteryInfo> {
        if !Path::new(&self.base_path).exists() {
            return None;
        }

        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        
        // Read basic values
        let status = self.read_file("status").unwrap_or_else(|| "Unknown".to_string());
        let capacity = self.read_as_number("capacity").unwrap_or(0u8);
        let voltage_v = self.read_as_number::<f64>("voltage_now").map(|v| v / 1_000_000.0);
        let current_ma = self.read_as_number::<i32>("current_now").map(|c| c / 1000);
        let _temperature_c = self.read_as_number::<f64>("temp").map(|t| t / 10.0);
        let cycles = self.read_as_number("cycle_count");

        // Read energy values with fallbacks
        let (energy_now_wh, energy_full_wh) = self.read_energy_values();

        // Read power with fallbacks
        let power_w = self.read_power(voltage_v, current_ma);

        // Get real-time temperatures using the new API
        let cpu_temp_reading = self.get_cpu_temperature();
        let battery_temp_reading = self.get_battery_temperature();
        let cpu_temperature_c = cpu_temp_reading.as_ref().map(|r| r.raw_value);
        let _temperature_c = battery_temp_reading.as_ref().map(|r| r.raw_value);

        // Update smoothed values
        if let Some(power) = power_w {
            self.update_smoothed_power(power);
            
            // Add to power history
            if let Some(energy) = energy_now_wh {
                self.power_history.push_back(PowerSample {
                    timestamp,
                    power_w: power,
                    energy_wh: energy,
                });
                
                if self.power_history.len() > self.max_history {
                    self.power_history.pop_front();
                }
            }
        }

        self.last_update = timestamp;

        // Create reading for history
        let reading = BatteryReading {
            timestamp,
            capacity_percent: capacity,
            energy_now_wh,
            energy_full_wh,
            power_now_w: power_w,
            voltage_v,
            current_ma,
            status: status.clone(),
            temperature_c: battery_temp_reading.as_ref().map(|r| r.raw_value),
        };

        // Calculate time remaining
        let time_remaining_minutes = self.calculate_time_remaining(&reading);

        // Add to readings history
        self.readings_history.push_back(reading);
        if self.readings_history.len() > self.max_history {
            self.readings_history.pop_front();
        }

        // Calculate health
        let health_percent = match (energy_full_wh, self.read_as_number::<f64>("energy_full_design").map(|e| e / 1_000_000.0)) {
            (Some(full), Some(design)) if design > 0.0 => (full / design) * 100.0,
            _ => {
                // Fallback to charge-based calculation
                match (
                    self.read_as_number::<f64>("charge_full"),
                    self.read_as_number::<f64>("charge_full_design")
                ) {
                    (Some(full), Some(design)) if design > 0.0 => (full / design) * 100.0,
            _ => 0.0,
                }
            }
        };

        let power_trend = self.get_power_trend();

        Some(BatteryInfo {
            status,
            capacity_percent: capacity,
            health_percent,
            cycles,
            temperature_c: battery_temp_reading.as_ref().map(|r| r.raw_value),
            voltage_v,
            current_ma,
            power_w,
            smoothed_power_w: self.smoothed_power,
            manufacturer: self.read_file("manufacturer").unwrap_or_else(|| "Unknown".to_string()),
            model: self.read_file("model_name").unwrap_or_else(|| "Unknown".to_string()),
            technology: self.read_file("technology").unwrap_or_else(|| "Unknown".to_string()),
            time_remaining_minutes,
            energy_now_wh,
            energy_full_wh,
            power_trend,
            cpu_temperature_c,
        })
    }

    pub fn get_battery_bar(&self, capacity: u8, width: usize) -> String {
        let filled = (capacity as f32 / 100.0 * width as f32) as usize;
        let empty = width - filled;
        
        let color = match capacity {
            0..=15 => "\x1b[31m",   // Red
            16..=30 => "\x1b[33m",  // Yellow
            31..=80 => "\x1b[32m",  // Green
            _ => "\x1b[36m",        // Cyan
        };
        
        format!("{}{}{}{}",
            color,
            "█".repeat(filled),
            "░".repeat(empty),
            "\x1b[0m"
        )
    }

    pub fn get_trend_indicator(&self) -> String {
        if self.readings_history.len() < 2 {
            return "━".to_string();
        }
        
        let recent: Vec<&BatteryReading> = self.readings_history.iter().rev().take(5).collect();
        if recent.len() < 2 {
            return "━".to_string();
        }

        let trend: i32 = recent.windows(2)
            .map(|w| w[0].capacity_percent as i32 - w[1].capacity_percent as i32)
            .sum();
        
        match trend {
            t if t > 0 => "\x1b[32m↗\x1b[0m".to_string(),  // Green up
            t if t < 0 => "\x1b[31m↘\x1b[0m".to_string(),  // Red down
            _ => "\x1b[37m━\x1b[0m".to_string(),           // Gray stable
        }
    }

    pub fn get_power_graph(&self, width: usize) -> String {
        if self.power_history.len() < 2 {
            return " ".repeat(width);
        }

        let values: Vec<f64> = self.power_history.iter().map(|p| p.power_w).collect();
        let min_val = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_val = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let range = if (max_val - min_val).abs() < 0.1 { 0.1 } else { max_val - min_val };

        let bars = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
        
        values.iter()
            .rev()
            .take(width)
            .map(|&val| {
                let normalized = ((val - min_val) / range * (bars.len() - 1) as f64) as usize;
                bars[normalized.min(bars.len() - 1)]
            })
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    pub fn format_time(&self, minutes: u32) -> String {
        let hours = minutes / 60;
        let mins = minutes % 60;
        if hours > 0 {
            format!("{}h {:02}m", hours, mins)
        } else {
            format!("{}m", mins)
        }
    }

    pub fn display_battery_info(&mut self, info: &BatteryInfo, elapsed_secs: u64) {
        // Clear screen and move to top
        print!("\x1b[2J\x1b[H");
        
        // Header
        println!("\x1b[1;36m╔══════════════════════════════════════════════════════════════╗\x1b[0m");
        println!("\x1b[1;36m║\x1b[0m \x1b[1;37m🔋 Batfi v2.0 - Advanced Battery Monitor\x1b[0m                \x1b[1;36m║\x1b[0m");
        println!("\x1b[1;36m╚══════════════════════════════════════════════════════════════╝\x1b[0m");
        println!();

        // Main battery display
        let bar_width = 40;
        let battery_bar = self.get_battery_bar(info.capacity_percent, bar_width);
        let trend = self.get_trend_indicator();
        
        println!(" \x1b[1m{}%\x1b[0m [{}] {}", info.capacity_percent, battery_bar, trend);
        println!(" Status: \x1b[1m{}\x1b[0m", match info.status.as_str() {
            "Charging" => format!("\x1b[32m{} ⚡\x1b[0m", info.status),
            "Discharging" => format!("\x1b[33m{} 🔋\x1b[0m", info.status),
            "Full" => format!("\x1b[36m{} ✓\x1b[0m", info.status),
            _ => format!("\x1b[37m{}\x1b[0m", info.status),
        });

        // Enhanced time display with real-time precision
        if let Some(time) = info.time_remaining_minutes {
            let time_str = self.format_time(time);
            let (icon, status_text) = match info.status.as_str() {
                "Charging" => {
                    let charge_phase = if info.capacity_percent > 95 {
                        " (trickle charge)"
                    } else if info.capacity_percent > 80 {
                        " (slowing down)"
                    } else {
                        " (fast charge)"
                    };
                    ("⚡", format!("to full{}", charge_phase))
                },
                "Discharging" => ("🔋", "remaining".to_string()),
                _ => ("🔋", "remaining".to_string()),
            };
            
            let accuracy = if self.rolling_power_window.len() >= ROLLING_WINDOW_SIZE {
                "\x1b[32m●●●\x1b[0m" // Three green dots for ultra-high accuracy
            } else if self.power_history.len() >= MIN_SAMPLES_FOR_ESTIMATE * 3 {
                "\x1b[32m●●\x1b[0m" // Two green dots for high accuracy
            } else if self.power_history.len() >= MIN_SAMPLES_FOR_ESTIMATE {
                "\x1b[33m●\x1b[0m" // One yellow dot for basic accuracy
            } else {
                "\x1b[31m○\x1b[0m" // Red circle for low confidence
            };
            
            println!(" Time:   \x1b[1m{} {} {}\x1b[0m {}", time_str, icon, status_text, accuracy);
        } else {
            let calculating_dots = match SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() % 4 {
                0 => "   ",
                1 => "●  ",
                2 => "●● ",
                _ => "●●●",
            };
            println!(" Time:   \x1b[2mCalculating{}\x1b[0m", calculating_dots);
        }

        println!();

        // Cat animation
        let cat_animation = generate_pacman_cat_animation(elapsed_secs);
        println!(" {}", cat_animation);

        println!();

        // Enhanced power information with real-time analytics
        println!(" \x1b[1mReal-Time Power Analytics:\x1b[0m");
        if let Some(power) = info.power_w {
            let power_color = if info.status == "Charging" { "\x1b[32m" } else { "\x1b[33m" };
            println!(" ├─ Current:   {}{:.2}W\x1b[0m", power_color, power);
        }
        if let Some(smoothed) = info.smoothed_power_w {
            let rolling_avg = self.get_rolling_average_power().unwrap_or(smoothed);
            println!(" ├─ Smoothed:  \x1b[1m{:.2}W\x1b[0m (trend: {})", 
                smoothed, 
                match info.power_trend.as_str() {
                    "increasing" => "\x1b[31m↑\x1b[0m",
                    "decreasing" => "\x1b[32m↓\x1b[0m",
                    _ => "\x1b[37m→\x1b[0m",
                }
            );
            if self.rolling_power_window.len() >= 3 {
                println!(" ├─ Rolling:   \x1b[1m{:.2}W\x1b[0m ({}s avg)", 
                    rolling_avg, 
                    self.rolling_power_window.len() * UPDATE_INTERVAL_SECS as usize
                );
            }
        }
        if let Some(voltage) = info.voltage_v {
            println!(" ├─ Voltage:   \x1b[1m{:.2}V\x1b[0m", voltage);
        }
        if let Some(current) = info.current_ma {
            let current_str = if current >= 0 {
                format!("\x1b[32m+{} mA\x1b[0m", current)
            } else {
                format!("\x1b[31m{} mA\x1b[0m", current)
            };
            println!(" └─ Current:   {}", current_str);
        }

        println!();

        // Energy information
        println!(" \x1b[1mEnergy Details:\x1b[0m");
        if let (Some(now), Some(full)) = (info.energy_now_wh, info.energy_full_wh) {
            println!(" ├─ Current:   \x1b[1m{:.1} Wh\x1b[0m", now);
            println!(" └─ Full:      \x1b[1m{:.1} Wh\x1b[0m", full);
        }

        println!();

        // Real-time temperature monitoring (2s updates, raw values only)
        let mut has_temp = false;
        println!(" \x1b[1mReal-Time Temperature (2s updates):\x1b[0m");
        
        // Battery temperature - raw values only
        if let Some(battery_reading) = self.temperature_monitor.last_battery_temp.as_ref() {
            let temp_c = battery_reading.raw_value;
            let temp_f = celsius_to_fahrenheit(temp_c);
            let temp_color = match temp_c as u32 {
                0..=35 => "\x1b[36m",   // Cyan (cool)
                36..=45 => "\x1b[32m",  // Green (normal) 
                46..=55 => "\x1b[33m",  // Yellow (warm)
                _ => "\x1b[31m",        // Red (hot)
            };
            println!(" ├─ Battery:   {}{:.1}°C ({:.1}°F)\x1b[0m [{}]", 
                temp_color, temp_c, temp_f, battery_reading.sensor_info.sensor_type);
            has_temp = true;
        } else {
            println!(" ├─ Battery:   \x1b[2m—\x1b[0m (no sensor found)");
        }
        
        // CPU temperature - raw values only with Fahrenheit
        if let Some(cpu_reading) = self.temperature_monitor.last_cpu_temp.as_ref() {
            let temp_c = cpu_reading.raw_value;
            let temp_f = celsius_to_fahrenheit(temp_c);
            let temp_color = match temp_c as u32 {
                0..=45 => "\x1b[36m",   // Cyan (cool)
                46..=60 => "\x1b[32m",  // Green (normal)
                61..=75 => "\x1b[33m",  // Yellow (warm)
                76..=85 => "\x1b[31m",  // Red (hot)
                _ => "\x1b[41m\x1b[37m", // Red background (critical)
            };
            println!(" └─ CPU:       {}{:.1}°C ({:.1}°F)\x1b[0m [{}]", 
                temp_color, temp_c, temp_f, cpu_reading.sensor_info.sensor_type);
            has_temp = true;
        } else {
            println!(" └─ CPU:       \x1b[2m—\x1b[0m (no sensor found)");
        }
        
        if !has_temp {
            println!(" └─ No valid temperature sensors found (range: {:.0}-{:.0}°C)", MIN_VALID_TEMP, MAX_VALID_TEMP);
        }

        println!();

        // Power consumption graph
        if self.power_history.len() > 1 {
            println!(" \x1b[1mPower History (last {} samples):\x1b[0m", self.power_history.len());
            let graph = self.get_power_graph(60);
            println!(" {}", graph);
            println!();
        }

        // Enhanced footer with real-time stats
        let samples = self.power_history.len();
        let rolling_samples = self.rolling_power_window.len();
        let accuracy_text = if rolling_samples >= ROLLING_WINDOW_SIZE {
            format!("\x1b[32mUltra-high accuracy\x1b[0m ({} samples, {}s rolling)", samples, rolling_samples * UPDATE_INTERVAL_SECS as usize)
        } else if samples >= MIN_SAMPLES_FOR_ESTIMATE * 3 {
            format!("\x1b[32mHigh accuracy\x1b[0m ({} samples)", samples)
        } else if samples >= MIN_SAMPLES_FOR_ESTIMATE {
            format!("\x1b[33mMedium accuracy\x1b[0m ({} samples)", samples)
        } else {
            format!("\x1b[31mBuilding accuracy\x1b[0m ({}/{} samples)", samples, MIN_SAMPLES_FOR_ESTIMATE)
        };
        
        let elapsed = if self.last_update > 0 {
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            format!("{}s ago", now - self.last_update)
        } else {
            "starting".to_string()
        };
        
        println!(" {} • \x1b[2mLast update: {} • Press Ctrl+C to exit • Real-time {}s updates\x1b[0m", 
            accuracy_text, elapsed, UPDATE_INTERVAL_SECS);
        
        io::stdout().flush().unwrap();
    }

    pub fn to_json(&self, info: &BatteryInfo) -> String {
        serde_json::to_string_pretty(info).unwrap_or_else(|_| "{}".to_string())
    }
}

pub fn find_batteries() -> Vec<String> {
    let power_supply_path = Path::new("/sys/class/power_supply");
    if !power_supply_path.exists() {
        return vec![];
    }

    fs::read_dir(power_supply_path)
        .unwrap_or_else(|_| fs::read_dir(".").unwrap())
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            if name.starts_with("BAT") || name.starts_with("battery") {
                Some(name)
            } else {
                None
            }
        })
        .collect()
}

fn main() {
    let matches = Command::new("batfi")
        .version("2.0.0")
        .author("Your Name <your.email@example.com>")
        .about("Advanced battery monitoring tool with accurate time estimation")
        .arg(
            Arg::new("json")
                .long("json")
                .short('j')
                .help("Output in JSON format")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("once")
                .long("once")
                .short('o')
                .help("Run once and exit")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("battery")
                .long("battery")
                .short('b')
                .value_name("NAME")
                .help("Specify battery name (e.g., BAT0, BAT1)")
                .action(clap::ArgAction::Set),
        )
        .get_matches();

    // Find available batteries
    let batteries = find_batteries();
    if batteries.is_empty() {
        eprintln!("❌ No batteries found in /sys/class/power_supply/");
        eprintln!("   Make sure you're running this on a laptop with battery support.");
        std::process::exit(1);
    }

    // Select battery
    let battery_name = if let Some(name) = matches.get_one::<String>("battery") {
        if batteries.contains(name) {
            name
        } else {
            eprintln!("❌ Battery '{}' not found. Available batteries: {}", name, batteries.join(", "));
            std::process::exit(1);
        }
    } else {
        &batteries[0] // Use first battery found
    };

    let mut monitor = BatteryMonitor::new(battery_name);
    let json_output = matches.get_flag("json");
    let run_once = matches.get_flag("once");

    if !json_output && !run_once {
        println!("🔋 Starting Batfi v2.0...");
    println!("   Found battery: {}", battery_name);
        println!("   Will run for {} seconds with {}s updates", PROGRAM_DURATION_SECS, UPDATE_INTERVAL_SECS);
        println!("   🐱 Watch the cat eat {} dots!", TOTAL_DOTS);
        println!("   Pac-Cat Progress: {}", "●".repeat(TOTAL_DOTS));
        thread::sleep(Duration::from_millis(1000));
    }

    // Record start time for auto-stop
    let start_time = SystemTime::now();
    let mut update_count = 0;

    // Main monitoring loop with auto-stop
    loop {
        match monitor.get_battery_info() {
            Some(info) => {
                if json_output {
                    println!("{}", monitor.to_json(&info));
                } else {
                    update_count += 1;
                    let elapsed = start_time.elapsed().unwrap().as_secs();
                    
                    // Show Pac-Man cat animation and countdown
                    let animation = generate_pacman_cat_animation(elapsed);
                    let countdown = generate_countdown_dots(elapsed);
                    println!("🔋 Update #{} ({}s elapsed)", update_count, elapsed);
                    println!("🐱 Pac-Cat: {}", animation);
                    println!("⏰ Countdown: {}", countdown);
                    println!();
                    
                monitor.display_battery_info(&info, elapsed);
                }
            }
            None => {
                if json_output {
                    eprintln!("{{\"error\": \"Could not read battery information\"}}");
                } else {
                println!("❌ Could not read battery information");
                println!("   Make sure {} exists and is readable", monitor.base_path);
                }
                std::process::exit(1);
            }
        }

        if run_once {
            break;
        }

        // Check if we should stop (20 seconds elapsed)
        let elapsed = start_time.elapsed().unwrap().as_secs();
        if elapsed >= PROGRAM_DURATION_SECS {
            println!("⏰ Program completed after {} seconds", elapsed);
            println!("\nPress Enter to exit...");
            // Run the curl command to get ASCII art immediately
            use std::io;
            use std::process::Command;
            
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            
            let status = Command::new("curl")
                .arg("ascii.live/rick")
                .status()
                .expect("Failed to run curl");
            
            if !status.success() {
                eprintln!("Error: curl command failed.");
            }
            
        }

        // Wait before next update
        thread::sleep(Duration::from_secs(UPDATE_INTERVAL_SECS));
    }
}



