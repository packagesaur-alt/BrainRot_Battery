# Batfi - Advanced Battery Monitor

🔋 **Batfi** is an advanced battery monitoring tool for Linux with accurate time estimation and beautiful terminal interface.

## Features

### ⚡ Accurate Time Estimation
- **Rolling averages** with exponential smoothing to eliminate power spikes
- **Multiple calculation methods** with automatic fallbacks
- **Energy-based calculations** using Wh instead of just percentage
- **Smart thresholds** to avoid unreliable estimates from low power readings

### 📊 Comprehensive Monitoring
- Battery percentage with color-coded visual bar
- Power consumption (current and smoothed)
- Energy levels (current/full in Wh)
- Voltage, current, and temperature
- Battery health and cycle count
- Real-time power consumption graphs
- Trend indicators (increasing/decreasing/stable)

### 🎯 Multiple Output Modes
- **Pretty terminal UI** with colors and Unicode characters
- **JSON output** for integration with other tools
- **Single-shot mode** for scripts
- **Real-time monitoring** with configurable intervals

### 🔧 Robust Implementation
- **Multiple fallback methods** for reading battery data
- **Cross-laptop compatibility** - works with different sysfs layouts
- **Automatic battery detection** (BAT0, BAT1, etc.)
- **Error handling** for missing or unreliable data

## Installation

### From AUR (Arch Linux)
```bash
yay -S batfi
# or
paru -S batfi
```

### From Source
```bash
git clone https://github.com/username/batfi
cd batfi
cargo build --release
sudo cp target/release/batfi /usr/local/bin/
```

## Usage

### Basic monitoring (real-time)
```bash
batfi
```

### JSON output for integration
```bash
batfi --json
```

### Single reading
```bash
batfi --once
```

### Specify battery
```bash
batfi --battery BAT1
```

### All options
```bash
batfi --help
```

## How Accuracy Works

Batfi uses multiple techniques to provide accurate time estimates:

### 1. Power-Based Calculations
Instead of simple percentage-based estimates, Batfi uses actual power consumption:
- **Discharging**: `time = current_energy_wh / power_consumption_w`
- **Charging**: `time = (full_energy_wh - current_energy_wh) / charging_power_w`

### 2. Exponential Moving Average
Power readings are smoothed using exponential moving average:
```
smoothed_power = α × current_power + (1-α) × previous_smoothed_power
```
This eliminates spikes from temporary load changes.

### 3. Multiple Fallback Methods
- Primary: `energy_now` and `power_now` from sysfs
- Fallback 1: `charge_now × voltage_now` 
- Fallback 2: `current_now × voltage_now`

### 4. Smart Filtering
- Ignores readings below minimum power threshold (0.1W)
- Requires minimum sample count before showing estimates
- Uses rolling history to detect trends

## Technical Details

### Accuracy Features
- **10-second update interval** for responsive yet stable readings
- **120 sample rolling history** (20 minutes of data)
- **Configurable smoothing factor** (α = 0.3 by default)
- **Minimum 3 samples** required before showing estimates

### Compatibility
Works across different laptop models by:
- Supporting both `energy_*` and `charge_*` sysfs interfaces
- Automatic unit conversion (µWh → Wh, µV → V, µA → mA)
- Fallback calculations when direct power reading unavailable
- Graceful handling of missing sensors

### Data Sources
Reads from `/sys/class/power_supply/BAT*/`:
- `energy_now`, `energy_full`, `energy_full_design`
- `charge_now`, `charge_full`, `charge_full_design` 
- `power_now`, `voltage_now`, `current_now`
- `capacity`, `status`, `health`, `cycle_count`
- `manufacturer`, `model_name`, `technology`

## JSON Output Format

```json
{
  "status": "Discharging",
  "capacity_percent": 75,
  "health_percent": 95.2,
  "cycles": 245,
  "temperature_c": 32.1,
  "voltage_v": 11.55,
  "current_ma": -1250,
  "power_w": 14.4375,
  "smoothed_power_w": 13.8,
  "time_remaining_minutes": 180,
  "energy_now_wh": 41.5,
  "energy_full_wh": 55.3,
  "power_trend": "stable",
  "manufacturer": "ACME",
  "model": "SuperBattery",
  "technology": "Li-ion"
}
```

## Configuration

You can modify accuracy parameters by editing the constants in `main.rs`:

```rust
const POWER_SMOOTHING_ALPHA: f64 = 0.3;      // Smoothing factor (0.1-0.5)
const MIN_POWER_THRESHOLD: f64 = 0.1;        // Min power for estimates (W)
const UPDATE_INTERVAL_SECS: u64 = 10;        // Update frequency
const MIN_SAMPLES_FOR_ESTIMATE: usize = 3;   // Min samples for estimate
```

## Troubleshooting

### No batteries found
- Ensure you're on a laptop with battery
- Check if `/sys/class/power_supply/BAT*` exists
- Try running with `sudo` if permission issues

### Inaccurate estimates
- Wait for more samples (shown in accuracy indicator)
- Check if your laptop provides `power_now` readings
- Some laptops need time for power readings to stabilize

### Missing data
- Not all laptops expose all sensors
- Batfi gracefully handles missing data
- Use `--json` to see what data is available

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Credits

Created for accurate battery monitoring on Linux laptops. Inspired by the need for reliable time estimates that don't jump around with system load changes.
