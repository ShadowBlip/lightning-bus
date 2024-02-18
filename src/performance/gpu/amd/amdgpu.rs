use std::{
    fs::{self, OpenOptions},
    io::Write,
    sync::{
        Arc, Mutex
    }
};

use crate::performance::gpu::interface::GPUIface;
use crate::performance::gpu::{amd, tdp::TDPDevice};
use crate::performance::gpu::interface::{GPUError, GPUResult};

pub struct AMDGPU {
    pub name: String,
    pub path: String,
    pub class: String,
    pub class_id: String,
    pub vendor: String,
    pub vendor_id: String,
    pub device: String,
    pub device_id: String,
    pub device_type: String,
    pub subdevice: String,
    pub subdevice_id: String,
    pub subvendor_id: String,
    pub revision_id: String,
}


impl GPUIface for AMDGPU {

    /// Returns the TDP DBus interface for this GPU
    fn get_tdp_interface(&self) -> Option<Arc<Mutex<dyn TDPDevice>>> {
        match self.class.as_str() {
            "integrated" => Some(
                Arc::new(
                    Mutex::new(
                        amd::tdp::TDP::new(
                            self.path.clone(),
                            self.device_id.clone()
                        )
                    )
                )
            ),
            _ => None,
        }
    }
    
    fn name(&self) -> String {
        self.name.clone()
    }

    fn path(&self) -> String {
        self.path.clone()
    }

    fn class(&self) -> String {
        self.class.clone()
    }

    fn class_id(&self) -> String {
        self.class_id.clone()
    }

    fn vendor(&self) -> String {
        self.vendor.clone()
    }

    fn vendor_id(&self) -> String {
        self.vendor_id.clone()
    }

    fn device(&self) -> String {
        self.device.clone()
    }

    fn device_id(&self) -> String {
        self.device_id.clone()
    }

    fn subdevice(&self) -> String {
        self.subdevice.clone()
    }

    fn subdevice_id(&self) -> String {
        self.subdevice_id.clone()
    }

    fn subvendor_id(&self) -> String {
        self.subvendor_id.clone()
    }

    fn revision_id(&self) -> String {
        self.revision_id.clone()
    }

    fn clock_limit_mhz_min(&self) -> GPUResult<f64> {
        let limits = get_clock_limits(self.path())
            .map_err(|err| GPUError::IOError(err.to_string()))?;

        let (min, _) = limits;
        Ok(min)
    }

    fn clock_limit_mhz_max(&self) -> GPUResult<f64> {
        let limits = get_clock_limits(self.path())
            .map_err(|err| GPUError::IOError(err.to_string()))?;

        let (_, max) = limits;
        Ok(max)
    }

    fn clock_value_mhz_min(&self) -> GPUResult<f64> {
        let values = get_clock_values(self.path())
            .map_err(|err| GPUError::IOError(err.to_string()))?;

        let (min, _) = values;
        Ok(min)
    }

    fn set_clock_value_mhz_min(&mut self, value: f64) -> GPUResult<()> {
        // Build the clock command to send
        // https://www.kernel.org/doc/html/v5.9/gpu/amdgpu.html#pp-od-clk-voltage
        let command = format!("s 0 {}\n", value);

        // Open the sysfs file to write to
        let path = format!("{0}/{1}", self.path(), "device/pp_od_clk_voltage");
        let file = OpenOptions::new().write(true).open(path.clone());

        // Write the value
        log::debug!(
            "Writing value '{}' to: {}",
            command.clone().trim(),
            path.clone()
        );
        file
            .map_err(|err| GPUError::FailedOperation(err.to_string()))?
            .write_all(command.as_bytes())
            .map_err(|err| GPUError::IOError(err.to_string()))?;

        // Write the "commit" command
        log::debug!("Writing value '{}' to: {}", "c", path.clone());
        let file = OpenOptions::new().write(true).open(path);
        file
            .map_err(|err| GPUError::FailedOperation(err.to_string()))?
            .write_all("c\n".as_bytes())
            .map_err(|err| GPUError::IOError(err.to_string()))?;

        Ok(())
    }

    fn clock_value_mhz_max(&self) -> GPUResult<f64> {
        let values = get_clock_values(self.path())
            .map_err(|err| GPUError::IOError(err.to_string()))?;

        let (_, max) = values;
        Ok(max)
    }

    fn set_clock_value_mhz_max(&mut self, value: f64) -> GPUResult<()> {
        // Build the clock command to send
        // https://www.kernel.org/doc/html/v5.9/gpu/amdgpu.html#pp-od-clk-voltage
        let command = format!("s 1 {}\n", value);

        // Open the sysfs file to write to
        let path = format!("{0}/{1}", self.path(), "device/pp_od_clk_voltage");
        let file = OpenOptions::new().write(true).open(path.clone());

        // Write the value
        file
            .map_err(|err| GPUError::FailedOperation(err.to_string()))?
            .write_all(command.as_bytes())
            .map_err(|err| GPUError::IOError(err.to_string()))?;

        // Write the "commit" command
        let file = OpenOptions::new().write(true).open(path);
        file
            .map_err(|err| GPUError::FailedOperation(err.to_string()))?
            .write_all("c\n".as_bytes())
            .map_err(|err| GPUError::IOError(err.to_string()))?;

        Ok(())
    }

    fn manual_clock(&self) -> GPUResult<bool> {
        let path = format!(
            "{0}/{1}",
            self.path(),
            "device/power_dpm_force_performance_level"
        );

        let result = fs::read_to_string(path);
        let status = result
            .map_err(|err| GPUError::IOError(err.to_string()))?
            .trim()
            .to_lowercase();

        Ok(status == "manual")
    }

    fn set_manual_clock(&mut self, enabled: bool) -> GPUResult<()> {
        let status = if enabled { "manual" } else { "auto" };

        // Open the sysfs file to write to
        let path = format!(
            "{0}/{1}",
            self.path(),
            "device/power_dpm_force_performance_level"
        );
        let file = OpenOptions::new().write(true).open(path);

        // Write the value
        file
            .map_err(|err| GPUError::FailedOperation(err.to_string()))?
            .write_all(status.as_bytes())
            .map_err(|err| GPUError::IOError(err.to_string()))?;

        Ok(())
    }
}

/// Reads the pp_od_clk_voltage from sysfs and returns the OD_RANGE values.
/// This file will be empty if not in "manual" for pp_od_performance_level.
fn get_clock_limits(gpu_path: String) -> Result<(f64, f64), std::io::Error> {
    let path = format!("{0}/{1}", gpu_path, "device/pp_od_clk_voltage");
    let result = fs::read_to_string(path);
    let result = result?;
    let lines = result.split('\n');

    // Parse the output
    let mut min: Option<f64> = None;
    let mut max: Option<f64> = None;
    for line in lines {
        let mut parts = line.trim().split_whitespace();
        let part1 = parts.next();
        if !part1.is_some_and(|part| part == "SCLK:") {
            continue;
        }

        let part2 = parts.next();
        if part2.is_none() {
            continue;
        }
        let parsed2 = part2.unwrap().trim_end_matches("Mhz").parse::<f64>();
        if parsed2.is_err() {
            continue;
        }
        min = Some(parsed2.unwrap());

        let part3 = parts.next();
        if part3.is_none() {
            continue;
        }
        let parsed3 = part3.unwrap().trim_end_matches("Mhz").parse::<f64>();
        if parsed3.is_err() {
            continue;
        }
        max = Some(parsed3.unwrap());
    }

    if min.is_none() || max.is_none() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No limits found",
        ));
    }

    Ok((min.unwrap(), max.unwrap()))
}

/// Reads the pp_od_clk_voltage from sysfs and returns the OD_SCLK values. This file will
/// be empty if not in "manual" for pp_od_performance_level.
fn get_clock_values(gpu_path: String) -> Result<(f64, f64), std::io::Error> {
    let path = format!("{0}/{1}", gpu_path, "device/pp_od_clk_voltage");
    let result = fs::read_to_string(path);
    let result = result?;
    let lines = result.split('\n');

    // Parse the output
    let mut min: Option<f64> = None;
    let mut max: Option<f64> = None;
    for line in lines {
        let mut parts = line.trim().split_whitespace();
        let part1 = parts.next();
        if !part1.is_some_and(|part| part == "0:" || part == "1:") {
            continue;
        }
        let kind = part1.unwrap();

        let part2 = parts.next();
        if part2.is_none() {
            continue;
        }
        let parsed2 = part2.unwrap().trim_end_matches("Mhz").parse::<f64>();
        if parsed2.is_err() {
            continue;
        }

        match kind {
            "0:" => min = Some(parsed2.unwrap()),
            "1:" => max = Some(parsed2.unwrap()),
            _ => continue,
        }
    }

    if min.is_none() || max.is_none() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No limits found",
        ));
    }

    Ok((min.unwrap(), max.unwrap()))
}
