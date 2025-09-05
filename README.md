# BrainRot_Battery - Advanced Battery Monitor

🔋 **BrainRot_Battery** is an advanced battery monitoring tool for Linux with accurate time estimation and beautiful terminal interface.

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
yay -S brainrot-battery
# or
paru -S brainrot-battery
```
