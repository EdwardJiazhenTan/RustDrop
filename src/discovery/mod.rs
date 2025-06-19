use anyhow::Result;
use mdns_sd::{ServiceDaemon, ServiceInfo, ServiceEvent};
use std::collections::HashMap;
use std::net::IpAddr;
use std::str::FromStr;
use std::time::Duration;
use tracing::info;
use tokio;

use crate::core::models::DeviceInfo;

const SERVICE_TYPE: &str = "_rustdrop._tcp.local.";

pub struct ServiceDiscovery {
    device_info: DeviceInfo,
    daemon: Option<ServiceDaemon>,
    service_fullname: Option<String>,
}

impl ServiceDiscovery {
    pub fn new(device_info: DeviceInfo) -> Self {
        Self {
            device_info,
            daemon: None,
            service_fullname: None,
        }
    }
    
    pub async fn register(&mut self) -> Result<&mut Self> {
        // Create a new mDNS daemon
        let daemon = ServiceDaemon::new()?;
        
        // Prepare service properties
        let mut properties = HashMap::new();
        properties.insert("name".to_string(), self.device_info.name.clone());
        properties.insert("os".to_string(), self.device_info.os.clone());
        properties.insert("id".to_string(), self.device_info.id.clone());
        
        // Create service info
        let host_ipv4 = IpAddr::from_str(&self.device_info.ip)?;
        let service_info = ServiceInfo::new(
            SERVICE_TYPE,
            &self.device_info.name,
            &format!("rustdrop-{}", self.device_info.id),
            host_ipv4,
            self.device_info.port,
            Some(properties),
        )?;
        
        // Register the service
        let service_fullname = service_info.get_fullname().to_string();
        daemon.register(service_info)?;
        
        // Store daemon and service name
        self.daemon = Some(daemon);
        self.service_fullname = Some(service_fullname);
        
        info!("Registered mDNS service: {}", SERVICE_TYPE);
        Ok(self)
    }
    
    pub async fn unregister(&mut self) -> Result<()> {
        if let (Some(daemon), Some(fullname)) = (&self.daemon, &self.service_fullname) {
            // Unregister the service - ignore any errors as they're likely harmless shutdown race conditions
            match daemon.unregister(fullname) {
                Ok(_) => info!("Unregistered mDNS service: {}", fullname),
                Err(e) => {
                    // Log as debug instead of error since these are usually harmless during shutdown
                    tracing::debug!("mDNS unregister error (likely harmless during shutdown): {}", e);
                }
            }
            
            // Give the daemon a moment to process the unregistration
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        
        // Clear references to ensure proper cleanup
        self.daemon = None;
        self.service_fullname = None;
        
        Ok(())
    }
    
    pub async fn discover() -> Result<Vec<DeviceInfo>> {
        let daemon = ServiceDaemon::new()?;
        let receiver = daemon.browse(SERVICE_TYPE)?;
        
        let mut devices = Vec::new();
        let timeout = Duration::from_secs(2);
        
        // Wait for responses with a timeout
        let start_time = std::time::Instant::now();
        
        while start_time.elapsed() < timeout {
            if let Ok(event) = receiver.recv_timeout(timeout) {
                match event {
                    ServiceEvent::ServiceResolved(info) => {
                        if let Some(device) = Self::service_to_device(&info) {
                            devices.push(device);
                        }
                    },
                    _ => {}
                }
            }
        }
        
        Ok(devices)
    }
    
    fn service_to_device(service: &ServiceInfo) -> Option<DeviceInfo> {
        let properties = service.get_properties();
        
        // Helper function to extract value from "key=value" format
        fn extract_value(property: &str, key: &str) -> Option<String> {
            let prefix = format!("{}=", key);
            if property.starts_with(&prefix) {
                Some(property.strip_prefix(&prefix)?.to_string())
            } else {
                None
            }
        }
        
        // Extract property values by parsing the key=value format
        let mut id = None;
        let mut name = None;
        let mut os = None;
        
        for property in properties.iter() {
            let prop_str = property.to_string();
            if let Some(value) = extract_value(&prop_str, "id") {
                id = Some(value);
            } else if let Some(value) = extract_value(&prop_str, "name") {
                name = Some(value);
            } else if let Some(value) = extract_value(&prop_str, "os") {
                os = Some(value);
            }
        }
        
        let id = id?;
        let name = name?;
        let os = os?;
        
        // Get the first IP address
        let addresses = service.get_addresses();
        let ip = if !addresses.is_empty() {
            addresses.iter().next()?.to_string()
        } else {
            return None;
        };
        
        let port = service.get_port();
        
        Some(DeviceInfo {
            id,
            name,
            ip,
            port,
            os,
        })
    }
}
